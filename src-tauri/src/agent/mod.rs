//! The agent loop: plan, then look at the screen, ask the model for one action, do it, repeat.
//!
//! One run is one task. Stop (hotkey, tray or button) cancels the run's own token; every wait in
//! here, including the model request, watches it, so Stop is felt within milliseconds.

pub mod body;
pub mod prompt;
pub mod protocol;
pub mod quick;
pub mod stuck;

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
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // The run is its own task, so a panic inside it cannot skip `finish`: the state would stay
        // at Planning/Acting, the pill would stay click-through and the voice session would hang.
        let work_app = app.clone();
        let outcome = supervised(async move { run(&work_app, &task, plan, scope.token).await }).await;
        finish(&app, outcome);
    });
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
async fn unless_stopped<T>(cancel: &CancelToken, work: impl Future<Output = T>) -> Option<T> {
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

async fn run(app: &AppHandle, task: &str, preset_plan: Option<Vec<String>>, cancel: CancelToken) -> Outcome {
    let state = app.state::<AppState>();
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

    let mut history: Vec<Entry> = Vec::new();
    let mut invalid_in_a_row = 0;
    let mut declines = 0;

    let mut watch = stuck::Watch::default();
    let mut elevated_watch = stuck::ElevatedWatch::default();
    // Whether an action ran since the last look, for noticing a screen that does not change.
    let mut acted = false;
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
        if cancel.is_cancelled() {
            return Outcome::Stopped;
        }

        // Anything the user said since the last step goes into the history as an instruction.
        for note in state.steer.locked().drain(..) {
            history.push(Entry { action: format!("the user said: \"{note}\""), outcome: "follow this from now on".into() });
        }

        // Look. If the decision model sees the page still loading, wait a moment and look again
        // (a few times at most) instead of asking the big model about a half-drawn page.
        let mut waits = 0;
        let (frame, element_text, notes) = loop {
            events::set_agent_state(app, AgentState::Observing);
            body::announce_observe(app, None);
            // The screenshot and the list of the front window's controls are taken together.
            let (captured, elements) = tokio::join!(
                tauri::async_runtime::spawn_blocking(|| {
                    screenshot::capture(None, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY)
                }),
                tauri::async_runtime::spawn_blocking(uia::snapshot),
            );
            let elements = elements.unwrap_or_default();
            let listing = uia::describe(&elements);
            uia::remember(elements);
            let frame = match captured {
                Ok(Ok(frame)) => frame,
                Ok(Err(err)) => return Outcome::Failed(format!("could not see the screen: {err}")),
                Err(err) => return Outcome::Failed(err.to_string()),
            };
            // Not on the first look: the desktop before any action is not loading anything.
            let notes = if step > 1 {
                match unless_stopped(&cancel, quick::read_screen(app, &frame.data_uri())).await {
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

        let screenshot = frame.data_uri();

        // Going nowhere? Tell the model after two unchanged looks, give up after five.
        let unchanged = watch.look(frame.signature(), acted);
        acted = false;
        if unchanged >= stuck::GIVE_UP_AFTER {
            return Outcome::Failed("Stopped: the screen stopped changing, so the actions are not working".into());
        }
        // A window that runs as administrator ignores our clicks and typing: say so, and give up if it stays in front.
        let elevated = foreground::blocked_by_elevation();
        if elevated_watch.look(elevated) >= stuck::ELEVATED_GIVE_UP_AFTER {
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
            let Some(finished) = unless_stopped(&cancel, quick::looks_finished(app, task, &actions.join("\n"), &screenshot)).await else {
                return Outcome::Stopped;
            };
            if finished {
                log::info!("step {step}: the decision model sees the task is finished");
                events::emit_plan(app, &plan, plan.len(), true);
                return Outcome::Finished { summary: format!("Done: {}", task.trim()), success: true };
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
        };
        let mut send_thinking = thinking_sender(app);
        let started = Instant::now();
        let Some(reply) = unless_stopped(&cancel, client.call_retrying(&request, &mut send_thinking)).await
        else {
            return Outcome::Stopped;
        };
        let reply = match reply {
            Ok(reply) => reply,
            Err(err @ ModelError::NoToolCall(_)) => {
                invalid_in_a_row += 1;
                if invalid_in_a_row >= MAX_INVALID_IN_A_ROW {
                    return Outcome::Failed(err.to_string());
                }
                history.push(Entry {
                    action: "(no action)".into(),
                    outcome: "you must call a tool".into(),
                });
                continue;
            }
            Err(err) => return Outcome::Failed(err.to_string()),
        };
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
                if invalid_in_a_row >= MAX_INVALID_IN_A_ROW {
                    return Outcome::Failed(problem);
                }
                history.push(Entry {
                    action: reply.call.name,
                    outcome: format!("invalid: {problem}"),
                });
                continue;
            }
        };
        invalid_in_a_row = 0;

        // The model says which goal this action serves. Goals only move forward.
        if let Some(goal) = goal {
            let index = (goal as usize - 1).min(plan.len() - 1);
            if index > current {
                current = index;
                events::emit_plan(app, &plan, current, false);
            }
        }

        if let Action::Done { summary, success } = action {
            events::emit_plan(app, &plan, plan.len(), true);
            return Outcome::Finished { summary, success };
        }

        // A question for the user is answered here; it never touches the desktop.
        if let Action::AskUser { question } = &action {
            let outcome = match gate::ask_question(app, &cancel, "Rune", question).await {
                Verdict::Allow => "the user answered: yes".to_string(),
                Verdict::Denied("declined") => "the user answered: no".to_string(),
                Verdict::Denied(_) => "the user did not answer in time, which counts as no".to_string(),
                Verdict::Stopped => return Outcome::Stopped,
            };
            history.push(Entry { action: action.describe(), outcome });
            continue;
        }

        // Ask first if it is risky (or if the user wants every action confirmed).
        match gate::check(app, &cancel, "Rune", &action, &reason, risk).await {
            Verdict::Allow => {}
            Verdict::Stopped => return Outcome::Stopped,
            Verdict::Denied(why) => {
                declines += 1;
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
        events::set_agent_state(app, AgentState::Acting);
        let settle = body::settle_time(&action);
        let monitor = frame.monitor;
        let worker_cancel = cancel.clone();
        let worker_app = app.clone();
        let executed = tauri::async_runtime::spawn_blocking(move || {
            body::execute_reporting(&worker_app, &action, &monitor, &worker_cancel)
        })
        .await;
        let result = match executed {
            Ok(result) => result,
            Err(err) => return Outcome::Failed(err.to_string()),
        };
        if !matches!(result, Err(ExecError::Cancelled)) {
            body::finish_step(app, step, &result);
        }
        match result {
            Err(ExecError::Cancelled) => return Outcome::Stopped,
            Err(err) => {
                history.push(Entry {
                    action: described,
                    outcome: format!("failed: {err}"),
                });
            }
            Ok(()) => {
                history.push(Entry {
                    action: described,
                    outcome: "done".into(),
                });
            }
        }

        acted = true;
        if cancel.sleep(settle).is_err() {
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
