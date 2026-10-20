//! The agent loop: plan, then look at the screen, ask the model for one action, do it, repeat.
//!
//! One run is one task. Stop (hotkey, tray or button) cancels the run's own token; every wait in
//! here, including the model request, watches it, so Stop is felt within milliseconds.

pub mod body;
pub mod prompt;
pub mod protocol;
pub mod quick;
pub mod stuck;
pub mod verify;

use crate::sync::LockExt;
use std::{
    future::Future,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    events::{self, AgentState, AppState, TaskDone},
    exec::{CancelToken, ExecError, uia},
    model::client::{ChatClient, ModelError, Request},
    observe::screenshot::{self, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
    settings,
};

use crate::{
    settings::VerifyMode,
    trace::{self, RunInfo, StepRecord, VerdictRecord},
};
use verify::VerifyInput;
use prompt::Entry;
use protocol::{Action, Decision};

use crate::safety::{
    foreground,
    gate::{self, Verdict},
};

/// Give up after this many unusable model answers in a row.
const MAX_INVALID_IN_A_ROW: u32 = 3;
/// Give up after the user has said no this many times: they do not want this done.
const MAX_DECLINES: u32 = 3;
/// Thinking streams in token by token; the UI is told this often at most.
const THINKING_EVERY: Duration = Duration::from_millis(150);
/// How long to wait before looking again at a page that is still loading.
const LOAD_WAIT: Duration = Duration::from_millis(900);

/// How a run ended.
#[derive(Debug, PartialEq)]
enum Outcome {
    Finished { summary: String, success: bool },
    Stopped,
    Failed(String),
}

/// Starts a task on a background task and returns at once.
pub fn start(app: &AppHandle, task: String) {
    start_planned(app, task, None);
}

/// Like `start`, with the goals already worked out (a spoken task heard by an audio model comes
/// with its plan, so the separate planning call is skipped).
pub fn start_planned(app: &AppHandle, task: String, plan: Option<Vec<String>>) {
    // Synchronously, so a second start_task sees a running agent and is refused. The new scope
    // also cancels the previous run's, so nothing from an earlier task can carry on.
    let scope = app.state::<AppState>().begin_run();
    log::info!("run {} started", scope.id);
    events::set_agent_state(app, AgentState::Planning);
    // Everything captured for this task goes in one session folder. If it cannot be made, saving
    // fails closed anyway, so the run is left to report that.
    let session = crate::archive::begin_session(&task, "built-in").unwrap_or_else(|err| {
        log::error!("could not open the session folder: {err}");
        String::new()
    });
    // The run's id in the flight recorder is its session id, or a stand-in when there is no session folder.
    let run_id = if session.is_empty() { format!("norun-{}-{}", scope.id, crate::archive::now()) } else { session.clone() };
    let state = app.state::<AppState>();
    let trace = state.trace.clone();
    // Read once: a change to the setting in the middle of a task does not change how this one ends.
    let mode = state.settings.locked().verify_mode;
    let handoff: Shared = Shared::default();
    let (model, settings_json) = trace::model_and_snapshot(&state.settings);
    trace.begin_run(RunInfo {
        id: run_id.clone(),
        scope_id: i64::try_from(scope.id).ok(),
        task: task.clone(),
        source: "built-in".into(),
        started_utc: trace::now_utc(),
        plan_json: plan.as_ref().and_then(|goals| serde_json::to_string(goals).ok()),
        model: Some(model),
        app_version: Some(env!("CARGO_PKG_VERSION").into()),
        settings_json: Some(settings_json),
        session_dir: crate::archive::session_dir(&session).map(|dir| dir.display().to_string()),
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // The run is its own task, so a panic inside it cannot skip `finish`: the state would stay
        // at Planning/Acting, the pill would stay click-through and the voice session would hang.
        let work_app = app.clone();
        let leftovers = scope.token.clone();
        let work_id = run_id.clone();
        let work_handoff = handoff.clone();
        let outcome = supervised(async move { run(&work_app, &task, plan, scope.token, &work_id, mode, work_handoff).await }).await;
        stop_leftovers(&leftovers, &outcome);
        end_session(&session, &outcome);
        let (label, summary) = outcome_label(&outcome);
        trace.end_run(&run_id, label, summary);
        let (checked, evidence) = {
            let mut left = handoff.locked();
            (left.checked, left.evidence.take())
        };
        let next = after_run(mode, &outcome, checked);
        finish(&app, outcome);
        // Only now, with the user already looking at the result: the check never delays it.
        match next {
            AfterRun::Nothing => {}
            AfterRun::Check => spawn_background_check(&app, run_id, evidence),
            AfterRun::Skip => trace.verdict(skipped_record(&run_id, mode, &app.state::<AppState>().settings.locked().model)),
        }
    });
}

/// What a finished run leaves for the background check: only the one latest frame is kept.
#[derive(Debug, Clone)]
struct Evidence {
    task: String,
    goals: Vec<String>,
    history: Vec<Entry>,
    /// What the agent says it did.
    claim_summary: String,
    /// The screen at the end, as a data URI. `None` when there was no frame to keep.
    screenshot: Option<String>,
}

impl Evidence {
    fn new(task: &str, goals: &[String], history: &[Entry], claim_summary: &str, screenshot: Option<&str>) -> Self {
        Self { task: task.to_string(), goals: goals.to_vec(), history: history.to_vec(), claim_summary: claim_summary.to_string(), screenshot: screenshot.map(str::to_string) }
    }
}

/// What `run` hands to `start_planned` besides its outcome.
#[derive(Default)]
struct Leftovers {
    /// For the background check; filled when a task the agent claims succeeded is about to finish.
    evidence: Option<Evidence>,
    /// A verdict has been stored for this run already (blocking mode does it inside the run).
    checked: bool,
}

type Shared = std::sync::Arc<std::sync::Mutex<Leftovers>>;

/// What to do about the independent check once a run is over.
#[derive(Debug, PartialEq, Eq)]
enum AfterRun {
    Nothing,
    /// Run the background check.
    Check,
    /// Store that there was nothing to check.
    Skip,
}

fn after_run(mode: VerifyMode, outcome: &Outcome, checked: bool) -> AfterRun {
    let claimed_success = matches!(outcome, Outcome::Finished { success: true, .. });
    match mode {
        VerifyMode::Off => AfterRun::Nothing,
        _ if checked => AfterRun::Nothing,
        VerifyMode::Background if claimed_success => AfterRun::Check,
        // Blocking mode checks inside the run, before it finishes; a success claim that was not
        // checked has nothing left to do here.
        VerifyMode::Blocking if claimed_success => AfterRun::Nothing,
        _ => AfterRun::Skip,
    }
}

fn mode_label(mode: VerifyMode) -> &'static str {
    match mode {
        VerifyMode::Off => "off",
        VerifyMode::Background => "background",
        VerifyMode::Blocking => "blocking",
    }
}

/// The row for a run that was not checked because the agent did not claim success.
fn skipped_record(run_id: &str, mode: VerifyMode, model: &str) -> VerdictRecord {
    VerdictRecord { run_id: run_id.to_string(), mode: mode_label(mode).into(), result: "skipped".into(), model: Some(model.to_string()), created_utc: trace::now_utc(), ..VerdictRecord::default() }
}

/// The row for a check that ran (or failed to).
fn verdict_record(run_id: &str, mode: VerifyMode, model: &str, checked: &Result<verify::Verdict, String>, latency_ms: Option<i64>, challenged: bool) -> VerdictRecord {
    let (result, evidence, error) = match checked {
        Ok(verdict) => (verdict.result.as_str(), Some(verdict.evidence.clone()), None),
        Err(why) => ("error", None, Some(why.clone())),
    };
    VerdictRecord {
        run_id: run_id.to_string(),
        mode: mode_label(mode).into(),
        result: result.into(),
        evidence,
        error,
        model: Some(model.to_string()),
        latency_ms,
        claimed_success: true,
        challenged,
        created_utc: trace::now_utc(),
    }
}

/// How blocking mode answered.
enum Blocking {
    /// Stop was pressed while the check ran.
    Stopped,
    /// The check contradicted the agent: push this line into its history and go on, once.
    Challenge(String),
    /// Finish as the agent wanted.
    Finish,
}

/// Blocking mode: checks the screen before a claimed success is accepted and stores the verdict.
/// It runs inside `unless_stopped`, so Stop works; any failure of the check is stored and the
/// task finishes as it would have. Never challenges twice.
async fn blocking_check(client: &ChatClient<'_>, cancel: &CancelToken, trace: &trace::Recorder, run_id: &str, handoff: &Shared, challenged: bool, input: VerifyInput<'_>) -> Blocking {
    let started = Instant::now();
    let Some(checked) = unless_stopped(cancel, verify::verify(client, input)).await else {
        return Blocking::Stopped;
    };
    let challenge = checked.as_ref().is_ok_and(|verdict| verify::should_challenge(VerifyMode::Blocking, true, challenged, verdict.result));
    trace.verdict(verdict_record(run_id, VerifyMode::Blocking, client.model, &checked, millis(started), challenge));
    handoff.locked().checked = true;
    match (challenge, checked) {
        (true, Ok(verdict)) => {
            trace.event(run_id, "note", r#"{"text":"the independent check contradicted the claim; the agent was asked to keep going"}"#);
            Blocking::Challenge(format!("an independent check says the screen does not show the task as done: {}. Keep working, or use ask_user if you are stuck", verdict.evidence.trim_end_matches('.')))
        }
        _ => Blocking::Finish,
    }
}

/// Resolves when the user needs the screen: the check was cancelled (a new task, Stop) or the
/// agent is working again.
async fn until_the_user_needs_the_screen(app: &AppHandle, token: &CancelToken) {
    while !token.is_cancelled() && !events::agent_state(app).is_running() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Background mode: checks the finished task off its clock, stores the verdict, and tells the UI.
/// Starting a task or pressing Stop cancels it at once; it also gives up if the agent is working again.
fn spawn_background_check(app: &AppHandle, run_id: String, evidence: Option<Evidence>) {
    let token = app.state::<AppState>().begin_verification();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let (settings, api_key) = (state.settings.locked().clone(), settings::api_key());
        let client = ChatClient { http: &state.http, base_url: &settings.base_url, api_key: api_key.as_deref(), model: &settings.model };
        let started = Instant::now();
        let checked: Result<verify::Verdict, String> = match &evidence {
            Some(Evidence { task, goals, history, claim_summary, screenshot: Some(screenshot) }) => {
                let input = VerifyInput { task, goals, history, claim_summary, screenshot };
                tokio::select! {
                    checked = verify::verify(&client, input) => checked,
                    () = until_the_user_needs_the_screen(&app, &token) => Err("cancelled: the screen was needed for something else".into()),
                }
            }
            _ => Err("no final screen was kept to check".into()),
        };
        log::info!("independent check of run {run_id}: {}", match &checked { Ok(verdict) => verdict.result.as_str().to_string(), Err(why) => format!("error ({why})") });
        state.trace.verdict(verdict_record(&run_id, VerifyMode::Background, &settings.model, &checked, millis(started), false));
        if let Ok(verdict) = checked {
            let _ = app.emit(events::TASK_VERIFIED, events::TaskVerified { run_id, result: verdict.result.as_str(), evidence: verdict.evidence });
        }
    });
}

/// One step's row for the flight recorder. It is sent when it goes out of scope, so every way out of
/// a step (a `continue`, a `return`, even a panic) leaves its row. A step that ends with no result
/// recorded counts as cancelled: the early returns of the loop are all Stop.
pub(crate) struct StepRow {
    trace: trace::Recorder,
    pub(crate) rec: StepRecord,
}

impl StepRow {
    pub(crate) fn new(trace: &trace::Recorder, run_id: &str, step: u32) -> Self {
        let mut rec = StepRecord::new(run_id, step);
        rec.started_utc = Some(trace::now_utc());
        Self { trace: trace.clone(), rec }
    }

    /// Ends the step as failed with a message.
    pub(crate) fn fail(&mut self, message: &str) {
        self.rec.result = Some("failed".into());
        self.rec.error = Some(message.to_string());
    }
}

impl Drop for StepRow {
    fn drop(&mut self) {
        let mut rec = std::mem::take(&mut self.rec);
        rec.result.get_or_insert_with(|| "cancelled".into());
        self.trace.step(rec);
    }
}

pub(crate) fn millis(started: Instant) -> Option<i64> {
    i64::try_from(started.elapsed().as_millis()).ok()
}

/// Once a run is over, however it ended, tells anything it left behind (an input action already
/// in flight after a panic) to stop. Tokens are per run, so this cannot affect the next one.
fn stop_leftovers(token: &CancelToken, _outcome: &Outcome) {
    token.cancel();
}

/// Runs `work` as a separate task and turns a panic or an abort into an `Outcome`, so the caller
/// always gets one back and can end the task cleanly.
async fn supervised(work: impl Future<Output = Outcome> + Send + 'static) -> Outcome {
    settle(tokio::spawn(work)).await
}

async fn settle(run: tokio::task::JoinHandle<Outcome>) -> Outcome {
    match run.await {
        Ok(outcome) => outcome,
        Err(err) if err.is_panic() => {
            let payload = err.into_panic();
            let message = payload.downcast_ref::<&str>().map(|text| text.to_string()).or_else(|| payload.downcast_ref::<String>().cloned()).unwrap_or_else(|| "(no message)".into());
            log::error!("the agent run panicked: {message}");
            Outcome::Failed("Something went wrong inside Rune: the task was stopped".into())
        }
        Err(_) => Outcome::Stopped,
    }
}

/// How a run ended, in the words the session manifest and the flight recorder share.
fn outcome_label(outcome: &Outcome) -> (&'static str, &str) {
    match outcome {
        Outcome::Finished { summary, success: true } => ("succeeded", summary.as_str()),
        Outcome::Finished { summary, success: false } => ("reported_failure", summary.as_str()),
        Outcome::Stopped => ("stopped", ""),
        Outcome::Failed(message) => ("failed", message.as_str()),
    }
}

/// Closes the task's session folder with how the run ended. Only that session: if Stop already
/// closed it, or a newer task has opened its own, nothing happens.
fn end_session(session: &str, outcome: &Outcome) {
    if session.is_empty() {
        return;
    }
    let (label, summary) = outcome_label(outcome);
    crate::archive::end_session(Some(session), label, summary);
}

fn finish(app: &AppHandle, outcome: Outcome) {
    match outcome {
        // Stop already moved the state machine to Stopped.
        Outcome::Stopped => {}
        Outcome::Finished { summary, success } => {
            log::info!("task finished (success: {success}): {summary}");
            let _ = app.emit(events::TASK_DONE, TaskDone { summary, success });
            events::set_agent_state(
                app,
                if success {
                    AgentState::Done
                } else {
                    AgentState::Failed
                },
            );
        }
        Outcome::Failed(message) => {
            log::warn!("task failed: {message}");
            let _ = app.emit(
                events::TASK_DONE,
                TaskDone {
                    summary: message,
                    success: false,
                },
            );
            events::set_agent_state(app, AgentState::Failed);
        }
    }
    // A voice session carries on: back to listening once the result has been seen.
    crate::voice::resume_listening_soon(app);
}

/// Resolves to `None` if Stop is pressed before the future completes. The future is dropped, so
/// an in-flight model request is aborted rather than waited for.
pub(crate) async fn unless_stopped<T>(cancel: &CancelToken, work: impl Future<Output = T>) -> Option<T> {
    tokio::select! {
        output = work => Some(output),
        () = async { while !cancel.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await } } => None,
    }
}

/// Passes the model's thinking on to the UI as it streams in, at most every `THINKING_EVERY`.
fn thinking_sender(app: &AppHandle) -> impl FnMut(&str) + Send {
    let app = app.clone();
    let mut last = Instant::now()
        .checked_sub(THINKING_EVERY)
        .unwrap_or_else(Instant::now);
    move |text: &str| {
        if last.elapsed() >= THINKING_EVERY {
            last = Instant::now();
            events::emit_thinking(&app, text, false);
        }
    }
}

/// The opening call of a task: split it into goals. A failed plan is not fatal (the whole task
/// becomes one goal and the loop carries on), but Stop is.
async fn make_plan(
    app: &AppHandle,
    client: &ChatClient<'_>,
    cancel: &CancelToken,
    task: &str,
    think: bool,
) -> Result<Vec<String>, Outcome> {
    let tools = protocol::plan_tools();
    let request = Request {
        system: prompt::PLAN_SYSTEM,
        user_text: &prompt::plan_user_message(task),
        screenshot: None,
        audio: None,
        tools: &tools,
        think,
        max_tokens: None,
    };
    let mut send_thinking = thinking_sender(app);
    let started = Instant::now();
    let Some(result) = unless_stopped(cancel, client.call_retrying(&request, &mut send_thinking)).await
    else {
        return Err(Outcome::Stopped);
    };
    let fallback = || vec![task.trim().chars().take(80).collect::<String>()];
    let goals = match result {
        Ok(reply) if reply.call.name == "set_plan" => {
            if think {
                events::emit_thinking(app, &reply.thinking, true);
            }
            protocol::parse_plan(&reply.call.arguments).unwrap_or_else(|_| fallback())
        }
        Ok(_) | Err(_) => fallback(),
    };
    log::info!("plan in {} ms: {goals:?}", started.elapsed().as_millis());
    Ok(goals)
}

async fn run(app: &AppHandle, task: &str, preset_plan: Option<Vec<String>>, cancel: CancelToken, run_id: &str, mode: VerifyMode, handoff: Shared) -> Outcome {
    let state = app.state::<AppState>();
    let trace = state.trace.clone();
    let settings = state
        .settings
        .locked()
        .clone();
    let api_key = settings::api_key();
    let tools = protocol::tools();
    let think = settings.show_thinking;
    let client = ChatClient {
        http: &state.http,
        base_url: &settings.base_url,
        api_key: api_key.as_deref(),
        model: &settings.model,
    };

    // A simple task needs no plan, and the decision model can say so in a fraction of the time the
    // planning call takes. (With "Show thinking" on the user wants to watch the model work, so it runs.)
    let simple = if think || preset_plan.is_some() {
        false
    } else {
        match unless_stopped(&cancel, quick::is_simple_task(app, task)).await {
            Some(simple) => simple,
            None => return Outcome::Stopped,
        }
    };
    let plan = if let Some(plan) = preset_plan {
        plan
    } else if simple {
        log::info!("simple task, planning skipped");
        vec![task.trim().chars().take(80).collect::<String>()]
    } else {
        match make_plan(app, &client, &cancel, task, think).await {
            Ok(plan) => plan,
            Err(outcome) => return outcome,
        }
    };
    let mut current = 0_usize;
    events::emit_plan(app, &plan, current, false);
    if let Ok(goals) = serde_json::to_string(&plan) {
        trace.set_plan(run_id, &goals);
        trace.event(run_id, "plan", &goals);
    }

    let mut history: Vec<Entry> = Vec::new();
    // Blocking mode sends the agent back to work once at most.
    let mut challenged = false;
    let mut invalid_in_a_row = 0;
    let mut declines = 0;

    let mut watch = stuck::Watch::default();
    let mut elevated_watch = stuck::ElevatedWatch::default();
    // Whether an action ran since the last look, for noticing a screen that does not change.
    let mut acted: Option<stuck::Acted> = None;
    // Ids from an earlier task must never be clicked in this one.
    uia::forget();

    // Control starts from the middle of the screen. Failing to move the cursor is not worth a task.
    if let Ok(monitor) = screenshot::monitor(None) {
        let worker_cancel = cancel.clone();
        let moved = tauri::async_runtime::spawn_blocking(move || body::centre_cursor(&monitor, &worker_cancel)).await;
        if matches!(moved, Ok(Err(ExecError::Cancelled))) {
            return Outcome::Stopped;
        }
    }

    for step in 1..=settings.max_steps {
        let mut row = StepRow::new(&trace, run_id, step);
        row.rec.goal = i64::try_from(current + 1).ok();
        if cancel.is_cancelled() {
            return Outcome::Stopped;
        }

        // Anything the user said since the last step goes into the history as an instruction.
        for note in state.steer.locked().drain(..) {
            trace.event(run_id, "steer", &serde_json::json!({ "text": note }).to_string());
            history.push(Entry { action: format!("the user said: \"{note}\""), outcome: "follow this from now on".into() });
        }

        // Look. If the decision model sees the page still loading, wait a moment and look again
        // (a few times at most) instead of asking the big model about a half-drawn page.
        let mut waits = 0;
        let (mut observe_ms, mut screen_check_ms) = (0_i64, None::<i64>);
        let (frame, element_text, notes) = loop {
            events::set_agent_state(app, AgentState::Observing);
            body::announce_observe(app, None);
            // The screenshot and the list of the front window's controls are taken together.
            let observing = Instant::now();
            let (captured, elements) = tokio::join!(
                tauri::async_runtime::spawn_blocking(|| {
                    screenshot::capture(None, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY)
                }),
                tauri::async_runtime::spawn_blocking(uia::snapshot),
            );
            observe_ms += millis(observing).unwrap_or(0);
            let elements = elements.unwrap_or_default();
            let listing = uia::describe(&elements);
            uia::remember(elements);
            let frame = match captured {
                Ok(Ok(frame)) => frame,
                Ok(Err(err)) => {
                    row.fail(&format!("could not see the screen: {err}"));
                    return Outcome::Failed(format!("could not see the screen: {err}"));
                }
                Err(err) => {
                    row.fail(&err.to_string());
                    return Outcome::Failed(err.to_string());
                }
            };
            // Not on the first look: the desktop before any action is not loading anything.
            let notes = if step > 1 {
                let checking = Instant::now();
                let read = unless_stopped(&cancel, quick::read_screen(app, &frame.data_uri())).await;
                screen_check_ms = Some(screen_check_ms.unwrap_or(0) + millis(checking).unwrap_or(0));
                match read {
                    Some(notes) => notes,
                    None => return Outcome::Stopped,
                }
            } else {
                None
            };
            if waits < quick::MAX_LOAD_WAITS && notes.as_ref().is_some_and(|notes| notes.loading) {
                waits += 1;
                log::info!("step {step}: the screen is still loading, waiting ({waits})");
                if cancel.sleep(LOAD_WAIT).is_err() {
                    return Outcome::Stopped;
                }
                continue;
            }
            break (frame, listing, notes);
        };
        row.rec.observe_ms = Some(observe_ms);
        row.rec.screen_check_ms = screen_check_ms;
        row.rec.observation_path = Some(frame.path.display().to_string()).filter(|path| !path.is_empty());
        row.rec.elements_hash = Some(trace::text_hash(&element_text));

        let screenshot = frame.data_uri();

        // Going nowhere? Tell the model after two unchanged looks, give up after five.
        let unchanged = watch.look(frame.signature(), acted.as_ref());
        acted = None;
        if stuck::warning(unchanged).is_some() {
            trace.event(run_id, "stuck_warning", &serde_json::json!({ "unchanged": unchanged }).to_string());
        }
        if watch.should_give_up() {
            row.fail("the screen stopped changing");
            return Outcome::Failed("Stopped: the screen stopped changing, so the actions are not working".into());
        }
        // A window that runs as administrator ignores our clicks and typing: say so, and give up if it stays in front.
        let elevated = foreground::blocked_by_elevation();
        if elevated {
            trace.event(run_id, "elevated_warning", "{}");
        }
        if elevated_watch.look(elevated) >= stuck::ELEVATED_GIVE_UP_AFTER {
            row.fail("the front window runs as administrator");
            return Outcome::Failed("I can't control this window because it runs as administrator.".into());
        }
        // What to tell the big model: that it is stuck, anything in the way of the page, and the administrator window.
        let warning = [stuck::warning(unchanged), notes.and_then(|notes| notes.hint).map(String::from), elevated.then(|| foreground::ELEVATED_WARNING.to_string())].into_iter().flatten().collect::<Vec<_>>().join(" ");
        let warning = (!warning.is_empty()).then_some(warning);

        // Already finished? Once the last goal has had its steps, the decision model can tell from
        // the screen, without a full chat call just to say "done". Unsure means the big model decides.
        let succeeded = history.iter().filter(|entry| entry.outcome == "done").count();
        if current + 1 == plan.len() && succeeded >= plan.len() && history.last().is_some_and(|entry| entry.outcome == "done") {
            let skipped = history.len().saturating_sub(10);
            let actions: Vec<String> = history.iter().skip(skipped).map(|entry| format!("{} -> {}", entry.action, entry.outcome)).collect();
            let checking = Instant::now();
            let Some(finished) = unless_stopped(&cancel, quick::looks_finished(app, task, &actions.join("\n"), &screenshot)).await else {
                return Outcome::Stopped;
            };
            row.rec.finish_check_ms = millis(checking);
            if finished {
                log::info!("step {step}: the decision model sees the task is finished");
                trace.event(run_id, "note", r#"{"text":"the decision model saw the task was finished"}"#);
                row.rec.gate = Some("none".into());
                row.rec.result = Some("skipped".into());
                let summary = format!("Done: {}", task.trim());
                if mode == VerifyMode::Blocking && !challenged {
                    let input = VerifyInput { task, goals: &plan, history: &history, claim_summary: &summary, screenshot: &screenshot };
                    match blocking_check(&client, &cancel, &trace, run_id, &handoff, challenged, input).await {
                        Blocking::Stopped => return Outcome::Stopped,
                        Blocking::Challenge(line) => {
                            history.push(Entry { action: "done".into(), outcome: line });
                            challenged = true;
                            row.rec.error = Some("challenged by the independent check".into());
                            continue;
                        }
                        Blocking::Finish => {}
                    }
                }
                if mode == VerifyMode::Background {
                    handoff.locked().evidence = Some(Evidence::new(task, &plan, &history, &summary, Some(&screenshot)));
                }
                events::emit_plan(app, &plan, plan.len(), true);
                return Outcome::Finished { summary, success: true };
            }
        }

        // Decide.
        events::set_agent_state(app, AgentState::Planning);
        let text = prompt::user_message(task, &plan, current, &history, &element_text, warning.as_deref(), step, settings.max_steps);
        let request = Request {
            system: prompt::SYSTEM,
            user_text: &text,
            screenshot: Some(&screenshot),
            audio: None,
            tools: &tools,
            think,
            max_tokens: None,
        };
        let mut send_thinking = thinking_sender(app);
        let started = Instant::now();
        row.rec.prompt_text = Some(text.clone());
        let Some(reply) = unless_stopped(&cancel, client.call_retrying(&request, &mut send_thinking)).await
        else {
            return Outcome::Stopped;
        };
        row.rec.decide_ms = millis(started);
        let reply = match reply {
            Ok(reply) => reply,
            Err(err @ ModelError::NoToolCall(_)) => {
                invalid_in_a_row += 1;
                trace.event(run_id, "invalid_answer", &serde_json::json!({ "error": err.to_string() }).to_string());
                if invalid_in_a_row >= MAX_INVALID_IN_A_ROW {
                    row.fail(&err.to_string());
                    return Outcome::Failed(err.to_string());
                }
                row.rec.result = Some("skipped".into());
                row.rec.error = Some(err.to_string());
                history.push(Entry {
                    action: "(no action)".into(),
                    outcome: "you must call a tool".into(),
                });
                continue;
            }
            Err(err) => {
                row.fail(&err.to_string());
                return Outcome::Failed(err.to_string());
            }
        };
        row.rec.tool = Some(reply.call.name.clone());
        row.rec.args_json = Some(reply.call.arguments.to_string());
        row.rec.completion_tokens = reply.completion_tokens.and_then(|tokens| i64::try_from(tokens).ok());
        row.rec.thinking_chars = i64::try_from(reply.thinking.chars().count()).ok();
        log::info!(
            "step {step}: {} in {} ms ({:?} tokens)",
            reply.call.name,
            started.elapsed().as_millis(),
            reply.completion_tokens
        );
        if think {
            events::emit_thinking(app, &reply.thinking, true);
        }

        let Decision {
            action,
            reason,
            goal,
            risk,
        } = match protocol::parse(&reply.call.name, &reply.call.arguments) {
            Ok(decision) => decision,
            Err(problem) => {
                invalid_in_a_row += 1;
                trace.event(run_id, "invalid_answer", &serde_json::json!({ "tool": reply.call.name, "problem": problem }).to_string());
                if invalid_in_a_row >= MAX_INVALID_IN_A_ROW {
                    row.fail(&problem);
                    return Outcome::Failed(problem);
                }
                row.rec.result = Some("skipped".into());
                row.rec.error = Some(format!("invalid: {problem}"));
                history.push(Entry {
                    action: reply.call.name,
                    outcome: format!("invalid: {problem}"),
                });
                continue;
            }
        };
        invalid_in_a_row = 0;
        row.rec.reason = Some(reason.clone());
        row.rec.risk_stated = risk.map(|risk| risk.label().to_string());
        row.rec.goal = goal.map(i64::from).or(row.rec.goal);

        // The model says which goal this action serves. Goals only move forward.
        if let Some(goal) = goal {
            let index = (goal as usize - 1).min(plan.len() - 1);
            if index > current {
                current = index;
                events::emit_plan(app, &plan, current, false);
            }
        }

        if let Action::Done { summary, success } = action {
            row.rec.gate = Some("none".into());
            row.rec.result = Some("ok".into());
            if success && mode == VerifyMode::Blocking && !challenged {
                let input = VerifyInput { task, goals: &plan, history: &history, claim_summary: &summary, screenshot: &screenshot };
                match blocking_check(&client, &cancel, &trace, run_id, &handoff, challenged, input).await {
                    Blocking::Stopped => return Outcome::Stopped,
                    Blocking::Challenge(line) => {
                        history.push(Entry { action: "done".into(), outcome: line });
                        challenged = true;
                        row.rec.result = Some("skipped".into());
                        row.rec.error = Some("challenged by the independent check".into());
                        continue;
                    }
                    Blocking::Finish => {}
                }
            }
            if success && mode == VerifyMode::Background {
                handoff.locked().evidence = Some(Evidence::new(task, &plan, &history, &summary, Some(&screenshot)));
            }
            events::emit_plan(app, &plan, plan.len(), true);
            return Outcome::Finished { summary, success };
        }

        // A question for the user is answered here; it never touches the desktop.
        if let Action::AskUser { question } = &action {
            let asking = Instant::now();
            let outcome = match gate::ask_question(app, &cancel, "Rune", question).await {
                Verdict::Allow => "the user answered: yes".to_string(),
                Verdict::Denied("declined") => "the user answered: no".to_string(),
                Verdict::Denied(_) => "the user did not answer in time, which counts as no".to_string(),
                Verdict::Stopped => return Outcome::Stopped,
            };
            row.rec.gate_ms = millis(asking);
            row.rec.gate = Some("none".into());
            row.rec.result = Some("ok".into());
            trace.event(run_id, "ask_user", &serde_json::json!({ "question": question, "answer": outcome }).to_string());
            history.push(Entry { action: action.describe(), outcome });
            continue;
        }

        // Ask first if it is risky (or if the user wants every action confirmed).
        let gating = Instant::now();
        let checked = gate::check_detailed(app, &cancel, "Rune", &action, &reason, risk).await;
        row.rec.gate_ms = millis(gating);
        row.rec.risk_final = Some(checked.risk_final.label().to_string());
        row.rec.risk_reasons_json = serde_json::to_string(&checked.reasons).ok();
        row.rec.gate = Some(checked.decision.to_string());
        match checked.verdict {
            Verdict::Allow => {}
            Verdict::Stopped => return Outcome::Stopped,
            Verdict::Denied(why) => {
                declines += 1;
                row.rec.result = Some("skipped".into());
                if declines >= MAX_DECLINES {
                    return Outcome::Failed("Stopped: you said no to this several times".into());
                }
                history.push(Entry {
                    action: action.describe(),
                    outcome: format!("the user did not allow it ({why}). Do not try another way to do the same thing; finish with done, or choose a different goal"),
                });
                continue;
            }
        }

        // Act.
        let described = body::announce(app, step, &reason, &action, &frame.monitor);
        let acted_now = stuck::Acted::new(action.tool_name(), action.describe());
        events::set_agent_state(app, AgentState::Acting);
        let settle = body::settle_time(&action);
        let monitor = frame.monitor;
        let worker_cancel = cancel.clone();
        let worker_app = app.clone();
        let acting = Instant::now();
        let executed = tauri::async_runtime::spawn_blocking(move || {
            body::execute_reporting(&worker_app, &action, &monitor, &worker_cancel)
        })
        .await;
        row.rec.act_ms = millis(acting);
        let result = match executed {
            Ok(result) => result,
            Err(err) => {
                row.fail(&err.to_string());
                return Outcome::Failed(err.to_string());
            }
        };
        if !matches!(result, Err(ExecError::Cancelled)) {
            body::finish_step(app, step, &result);
        }
        match result {
            Err(ExecError::Cancelled) => return Outcome::Stopped,
            Err(err) => {
                row.fail(&err.to_string());
                history.push(Entry {
                    action: described,
                    outcome: format!("failed: {err}"),
                });
            }
            Ok(()) => {
                row.rec.result = Some("ok".into());
                history.push(Entry {
                    action: described,
                    outcome: "done".into(),
                });
            }
        }

        acted = Some(acted_now);
        let settling = Instant::now();
        let settled = cancel.sleep(settle);
        row.rec.settle_ms = millis(settling);
        if settled.is_err() {
            return Outcome::Stopped;
        }
    }
    Outcome::Failed(format!(
        "stopped after {} steps without finishing",
        settings.max_steps
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stop_aborts_a_pending_future() {
        let cancel = CancelToken::default();
        let stopper = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            stopper.cancel();
        });
        let outcome = unless_stopped(&cancel, tokio::time::sleep(Duration::from_secs(30))).await;
        assert!(outcome.is_none());
    }

    #[tokio::test]
    async fn a_panic_in_a_run_becomes_a_failed_outcome() {
        let outcome = supervised(async {
            if true {
                panic!("boom");
            }
            Outcome::Stopped
        })
        .await;
        assert_eq!(outcome, Outcome::Failed("Something went wrong inside Rune: the task was stopped".into()));
    }

    #[tokio::test]
    async fn a_run_that_returns_passes_its_outcome_through() {
        let finished = Outcome::Finished { summary: "ok".into(), success: true };
        assert_eq!(supervised(async { Outcome::Finished { summary: "ok".into(), success: true } }).await, finished);
        assert_eq!(supervised(async { Outcome::Stopped }).await, Outcome::Stopped);
        assert_eq!(supervised(async { Outcome::Failed("no".into()) }).await, Outcome::Failed("no".into()));
    }

    #[test]
    fn a_finished_run_cancels_what_it_left_behind() {
        for outcome in [Outcome::Failed("boom".into()), Outcome::Stopped, Outcome::Finished { summary: "ok".into(), success: true }] {
            let token = CancelToken::default();
            let in_flight = token.clone();
            stop_leftovers(&token, &outcome);
            assert!(in_flight.is_cancelled(), "{outcome:?}");
        }
        let next_run = CancelToken::default();
        stop_leftovers(&CancelToken::default(), &Outcome::Stopped);
        assert!(!next_run.is_cancelled(), "another run's token is not touched");
    }

    fn finished(success: bool) -> Outcome {
        Outcome::Finished { summary: "s".into(), success }
    }

    #[test]
    fn what_to_do_about_the_check_after_a_run() {
        use VerifyMode::*;
        let outcomes = [finished(true), finished(false), Outcome::Stopped, Outcome::Failed("boom".into())];
        // Off: nothing, whatever happened.
        for outcome in &outcomes {
            assert_eq!(after_run(Off, outcome, false), AfterRun::Nothing);
            assert_eq!(after_run(Off, outcome, true), AfterRun::Nothing);
        }
        // Background: a claimed success is checked after the run; everything else is recorded as skipped.
        assert_eq!(after_run(Background, &finished(true), false), AfterRun::Check);
        for outcome in &outcomes[1..] {
            assert_eq!(after_run(Background, outcome, false), AfterRun::Skip, "{outcome:?}");
        }
        // Blocking checks inside the run: a claimed success is left alone, the rest is skipped...
        assert_eq!(after_run(Blocking, &finished(true), false), AfterRun::Nothing);
        for outcome in &outcomes[1..] {
            assert_eq!(after_run(Blocking, outcome, false), AfterRun::Skip, "{outcome:?}");
        }
        // ...unless the run already stored a verdict (a challenged task that later failed keeps its verdict).
        for outcome in &outcomes {
            for mode in [Background, Blocking] {
                assert_eq!(after_run(mode, outcome, true), AfterRun::Nothing, "{mode:?} {outcome:?}");
            }
        }
    }

    #[test]
    fn a_check_becomes_a_verdict_row_and_a_failure_becomes_an_error_row() {
        let ok = Ok(verify::Verdict { result: verify::VerdictResult::Contradicted, evidence: "The window is empty.".into() });
        let row = verdict_record("r1", VerifyMode::Blocking, "gemma", &ok, Some(4_200), true);
        assert_eq!((row.run_id.as_str(), row.mode.as_str(), row.result.as_str()), ("r1", "blocking", "contradicted"));
        assert_eq!((row.evidence.as_deref(), row.error.as_deref(), row.model.as_deref()), (Some("The window is empty."), None, Some("gemma")));
        assert_eq!((row.latency_ms, row.claimed_success, row.challenged), (Some(4_200), true, true));
        let failed = Err("the check took longer than 30 seconds".to_string());
        let row = verdict_record("r2", VerifyMode::Background, "gemma", &failed, Some(30_001), false);
        assert_eq!((row.result.as_str(), row.evidence.as_deref(), row.error.as_deref()), ("error", None, Some("the check took longer than 30 seconds")));
        assert_eq!((row.mode.as_str(), row.challenged), ("background", false));
        let skipped = skipped_record("r3", VerifyMode::Background, "gemma");
        assert_eq!((skipped.result.as_str(), skipped.claimed_success, skipped.challenged, skipped.latency_ms), ("skipped", false, false, None));
        assert_eq!((mode_label(VerifyMode::Off), mode_label(VerifyMode::Background), mode_label(VerifyMode::Blocking)), ("off", "background", "blocking"));
    }

    #[test]
    fn the_evidence_keeps_the_latest_frame_and_omits_a_missing_one() {
        let goals = vec!["Open".to_string(), "Type".to_string()];
        let history: Vec<Entry> = (1..=20).map(|n| Entry { action: format!("click {n}"), outcome: "done".into() }).collect();
        let evidence = Evidence::new("write hello", &goals, &history, "Done: write hello", Some("data:image/jpeg;base64,AA"));
        assert_eq!((evidence.task.as_str(), evidence.goals.len(), evidence.history.len()), ("write hello", 2, 20));
        assert_eq!(evidence.screenshot.as_deref(), Some("data:image/jpeg;base64,AA"));
        assert_eq!(Evidence::new("t", &[], &[], "", None).screenshot, None);
        // What the check shows of those 20 steps is capped at twelve (plus the note that older ones were left out).
        let shown = verify::user_message(&evidence.task, &evidence.goals, &evidence.history, &evidence.claim_summary);
        assert_eq!(shown.lines().filter(|line| line.contains(" -> ")).count(), 12);
        assert!(shown.contains("8 earlier steps omitted"));
    }

    #[test]
    fn the_handoff_starts_empty_and_gives_up_its_evidence_once() {
        let handoff: Shared = Shared::default();
        assert!(handoff.locked().evidence.is_none() && !handoff.locked().checked, "a run that never finishes leaves nothing to check");
        handoff.locked().evidence = Some(Evidence::new("t", &[], &[], "c", Some("data:x")));
        let taken = handoff.locked().evidence.take();
        assert!(taken.is_some() && handoff.locked().evidence.is_none(), "the evidence is taken once");
    }

    #[test]
    fn every_outcome_has_one_label_for_the_manifest_and_the_recorder() {
        assert_eq!(outcome_label(&Outcome::Finished { summary: "ok".into(), success: true }), ("succeeded", "ok"));
        assert_eq!(outcome_label(&Outcome::Finished { summary: "no".into(), success: false }), ("reported_failure", "no"));
        assert_eq!(outcome_label(&Outcome::Stopped), ("stopped", ""));
        assert_eq!(outcome_label(&Outcome::Failed("boom".into())), ("failed", "boom"));
    }

    #[test]
    fn a_step_row_is_sent_when_it_is_dropped_and_defaults_to_cancelled() {
        // No writer behind it: nothing is sent, nothing panics, and the default is applied.
        let trace = trace::Recorder::noop();
        let mut row = StepRow::new(&trace, "r1", 4);
        row.fail("no screen");
        assert_eq!((row.rec.result.as_deref(), row.rec.error.as_deref(), row.rec.step), (Some("failed"), Some("no screen"), 4));
        drop(row);
        let untouched = StepRow::new(&trace, "r1", 5);
        assert!(untouched.rec.result.is_none());
        assert!(untouched.rec.started_utc.is_some());
    }

    #[test]
    fn a_dropped_step_row_reaches_the_database() {
        let dir = std::env::temp_dir().join(format!("rune-agent-trace-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = trace::db_path(&dir);
        let recorder = trace::Recorder::start(&path);
        recorder.begin_run(RunInfo { id: "r1".into(), task: "t".into(), source: "built-in".into(), started_utc: trace::now_utc(), ..RunInfo::default() });
        {
            let mut cancelled = StepRow::new(&recorder, "r1", 1);
            cancelled.rec.tool = Some("click".into());
            let mut failed = StepRow::new(&recorder, "r1", 2);
            failed.fail("no screen");
            // Both go out of scope here, as they do on every way out of a loop iteration.
            let _ = (&mut cancelled, &mut failed);
        }
        recorder.flush();
        let steps = trace::store::get_run(&path, "r1").unwrap().unwrap().steps;
        let summary: Vec<_> = steps.iter().map(|row| (row.step, row.tool.as_deref(), row.result.as_deref(), row.error.as_deref())).collect();
        assert_eq!(summary, [(1, Some("click"), Some("cancelled"), None), (2, None, Some("failed"), Some("no screen"))]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_aborted_run_counts_as_stopped() {
        let run = tokio::spawn(std::future::pending::<Outcome>());
        run.abort();
        assert_eq!(settle(run).await, Outcome::Stopped);
    }

    #[tokio::test]
    async fn a_finished_future_wins_when_there_is_no_stop() {
        let cancel = CancelToken::default();
        assert_eq!(unless_stopped(&cancel, async { 7 }).await, Some(7));
    }
}

/// Talks to a real rune-infer server: asks for a plan with thinking on and checks that thinking
/// streams in and a usable plan comes out. Run with `-- --ignored --nocapture`.
#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "talks to a local rune-infer server"]
    async fn a_plan_comes_back_with_thinking_streamed() {
        let http = reqwest::Client::new();
        let key = std::env::var("RUNE_API_KEY").unwrap_or_else(|_| "sk-rune-local-test-key".into());
        let client = ChatClient {
            http: &http,
            base_url: "http://127.0.0.1:8080",
            api_key: Some(&key),
            model: "ternary-bonsai-2-27b-pq2-0",
        };
        let tools = protocol::plan_tools();
        let request = Request {
            system: prompt::PLAN_SYSTEM,
            user_text: &prompt::plan_user_message("Open Notepad and write hello"),
            screenshot: None,
            audio: None,
            tools: &tools,
            think: true,
            max_tokens: None,
        };
        let mut updates = 0;
        let mut longest = 0;
        let mut on_thinking = |text: &str| {
            updates += 1;
            longest = longest.max(text.chars().count());
        };
        let reply = client
            .call(&request, &mut on_thinking)
            .await
            .expect("the model should answer");
        let goals = protocol::parse_plan(&reply.call.arguments).expect("a usable plan");
        println!(
            "thinking updates: {updates}, longest thinking: {longest} chars, final thinking: {} chars",
            reply.thinking.chars().count()
        );
        println!("goals: {goals:?}");
        assert!(updates > 1, "thinking should stream in pieces");
        assert!(!goals.is_empty() && goals.len() <= protocol::MAX_GOALS);
    }
}
