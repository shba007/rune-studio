//! The evaluation harness: a fixed suite of desktop tasks run through the built-in agent, with the
//! outcome of each decided by looking at the machine, never by what the model says.
//!
//! Compiled only with `--features eval`, and only active when `RUNE_EVAL_SUITE` and
//! `RUNE_EVAL_OUT` are both set. A normal build has no way to start a task from a file. It adds no
//! network endpoint, reads no file but the suite, and never writes `settings.json`: the overrides in
//! the suite are applied to the settings in memory only.

pub mod check;
pub mod spec;
pub mod steps;
pub mod windows;

use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::{
    backend, commands,
    events::{self, AgentState, AppState},
    safety::gate,
    settings,
    sync::LockExt,
    trace::{self, StepRecord, store},
};
use spec::{Expect, KILL_ALLOWLIST, Suite, Task, Vars};

use crate::settings::VerifyMode;
use crate::trace::VerdictRecord;

/// How often the run is looked at.
const POLL_EVERY: Duration = Duration::from_millis(500);
/// A pause before the next task, so the desktop settles.
const BETWEEN_RUNS: Duration = Duration::from_secs(3);
/// How long after the last action the checker waits, so a page title or a saved file has landed.
const BEFORE_CHECK: Duration = Duration::from_secs(1);
/// How long to wait for the agent to be idle, and for the recorder to have written the run's end.
const IDLE_WAIT: Duration = Duration::from_secs(60);
const TRACE_WAIT: Duration = Duration::from_secs(10);
/// How long a background check may take before the harness stops waiting for it.
const VERIFY_WAIT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------- pure decisions

/// What to do about a run in progress.
#[derive(Debug, PartialEq, Eq)]
pub enum Poll {
    Running,
    /// A question is waiting: answer No.
    NeedsDenial,
    Finished,
    /// Out of time: stop it and call it a timeout.
    TimedOut,
}

pub fn poll_decision(state: AgentState, elapsed: Duration, timeout: Duration) -> Poll {
    if elapsed >= timeout {
        Poll::TimedOut
    } else if state == AgentState::AwaitingConfirm {
        Poll::NeedsDenial
    } else if state.is_running() {
        Poll::Running
    } else {
        Poll::Finished
    }
}

/// Whether the run may start: nothing it would close is already open (it would close the user's
/// own windows), and the model server is there and has the model.
pub fn preflight(
    running: &[String],
    reachable: bool,
    auth_ok: bool,
    missing_models: &[String],
    backend_error: Option<&str>,
) -> Result<(), String> {
    let open: Vec<&str> = KILL_ALLOWLIST
        .iter()
        .copied()
        .filter(|name| {
            running
                .iter()
                .any(|process| process.eq_ignore_ascii_case(name))
        })
        .collect();
    if !open.is_empty() {
        return Err(format!(
            "close these first, the run would close them: {}",
            open.join(", ")
        ));
    }
    if !reachable {
        return Err(format!(
            "the model server cannot be reached ({})",
            backend_error.unwrap_or("no details")
        ));
    }
    if !auth_ok {
        return Err("the model server rejected the API key".into());
    }
    if !missing_models.is_empty() {
        return Err(format!(
            "the model server does not serve: {}",
            missing_models.join(", ")
        ));
    }
    Ok(())
}

/// `C:\a b\fixtures` as `file:///C:/a%20b/fixtures`.
pub fn file_url(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    let encoded: String = text
        .chars()
        .map(|c| {
            if c == ' ' {
                "%20".to_string()
            } else {
                c.to_string()
            }
        })
        .collect();
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

/// The server's `usage.completion_tokens` has read 1 for every step with thinking off (62 of 62 in the
/// field test), so a number below this is not believed.
const USAGE_BELIEVED_FROM: i64 = 2;

/// The completion tokens of one step and whether the number is an estimate. The server's count is
/// used when it is plausible; otherwise the characters of what the model produced (the tool
/// arguments and the thinking) divided by 4. `None` when the step has neither.
pub fn tokens_of(step: &StepRecord) -> Option<(i64, bool)> {
    match step.completion_tokens {
        Some(tokens) if tokens >= USAGE_BELIEVED_FROM => Some((tokens, false)),
        reported => {
            let chars = step
                .args_json
                .as_ref()
                .map_or(0, |args| args.chars().count() as i64)
                + step.thinking_chars.unwrap_or(0);
            if chars > 0 {
                Some(((chars + 3) / 4, true))
            } else {
                reported.map(|tokens| (tokens, false))
            }
        }
    }
}

/// Sums over a run's steps: decide, observe and act milliseconds, and completion tokens.
pub fn totals(steps: &[StepRecord]) -> (i64, i64, i64, i64) {
    let sum = |pick: fn(&StepRecord) -> Option<i64>| steps.iter().filter_map(pick).sum::<i64>();
    (
        sum(|step| step.decide_ms),
        sum(|step| step.observe_ms),
        sum(|step| step.act_ms),
        steps
            .iter()
            .filter_map(tokens_of)
            .map(|(tokens, _)| tokens)
            .sum(),
    )
}

/// What the independent check said about a run, for `results.jsonl`.
#[derive(Debug, Clone, PartialEq)]
pub struct Verification {
    /// The mode the run was made in: `off`, `background` or `blocking`.
    pub mode: &'static str,
    /// The check's result, `timeout` when the harness gave up waiting, or nothing.
    pub verdict: Option<String>,
    /// How long the check took.
    pub verify_ms: Option<i64>,
    /// Blocking mode sent the agent back to work.
    pub challenged: bool,
}

impl Verification {
    pub fn off() -> Self {
        Self {
            mode: "off",
            verdict: None,
            verify_ms: None,
            challenged: false,
        }
    }
}

fn mode_word(mode: VerifyMode) -> &'static str {
    match mode {
        VerifyMode::Off => "off",
        VerifyMode::Background => "background",
        VerifyMode::Blocking => "blocking",
    }
}

/// What to report about the check of one run: the stored verdict, or `timeout` if the harness
/// waited for a background check and it never came.
pub fn verification(
    mode: VerifyMode,
    stored: Option<&VerdictRecord>,
    waited_out: bool,
) -> Verification {
    if mode == VerifyMode::Off {
        return Verification::off();
    }
    match (stored, waited_out) {
        (Some(record), _) => Verification {
            mode: mode_word(mode),
            verdict: Some(record.result.clone()),
            verify_ms: record.latency_ms,
            challenged: record.challenged,
        },
        (None, true) => Verification {
            mode: mode_word(mode),
            verdict: Some("timeout".into()),
            verify_ms: None,
            challenged: false,
        },
        (None, false) => Verification {
            mode: mode_word(mode),
            verdict: None,
            verify_ms: None,
            challenged: false,
        },
    }
}

/// One line of `results.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunResult {
    pub task: String,
    pub repeat: u32,
    pub run_id: String,
    /// The trace's outcome label, or `timeout`.
    pub outcome: String,
    /// What the model claimed: true when the run ended `succeeded`.
    pub self_reported_success: bool,
    /// What the checker found, whatever the model said.
    pub check_passed: bool,
    pub expect: &'static str,
    /// The checker **and** the expectation.
    pub passed: bool,
    pub steps: usize,
    pub duration_ms: u64,
    pub decide_ms_total: i64,
    pub observe_ms_total: i64,
    pub act_ms_total: i64,
    pub completion_tokens_total: i64,
    /// Some step's count is an estimate (characters / 4) because the server's `usage` was not believable.
    pub tokens_estimated: bool,
    pub denied_prompts: u32,
    /// The mode of the independent check, its verdict (`verified`, `contradicted`, `unsure`, `error`,
    /// `skipped`, or `timeout`), how long it took, and whether blocking mode challenged the agent.
    /// `duration_ms` is the task's own time: a background check is outside it, a blocking one inside.
    pub verify_mode: &'static str,
    pub verdict: Option<String>,
    pub verify_ms: Option<i64>,
    pub challenged: bool,
    /// Preflight, setup, teardown or checker problems.
    pub error: Option<String>,
}

impl RunResult {
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Builds the result of one run from what was measured.
#[allow(clippy::too_many_arguments)]
pub fn build_result(
    task: &Task,
    repeat: u32,
    run_id: &str,
    outcome: &str,
    check_passed: bool,
    steps: &[StepRecord],
    duration: Duration,
    denied_prompts: u32,
    error: Option<String>,
    verification: Verification,
) -> RunResult {
    let (decide_ms_total, observe_ms_total, act_ms_total, completion_tokens_total) = totals(steps);
    RunResult {
        task: task.id.clone(),
        repeat,
        run_id: run_id.to_string(),
        outcome: outcome.to_string(),
        self_reported_success: outcome == "succeeded",
        check_passed,
        expect: if task.expect == Expect::Failure {
            "failure"
        } else {
            "success"
        },
        passed: check::passed(task, check_passed, outcome),
        steps: steps.len(),
        duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        decide_ms_total,
        observe_ms_total,
        act_ms_total,
        completion_tokens_total,
        tokens_estimated: steps
            .iter()
            .filter_map(tokens_of)
            .any(|(_, estimated)| estimated),
        denied_prompts,
        verify_mode: verification.mode,
        verdict: verification.verdict,
        verify_ms: verification.verify_ms,
        challenged: verification.challenged,
        error,
    }
}

// ---------------------------------------------------------------- the run

/// Starts the harness if this is an eval build and both variables are set; otherwise does nothing.
pub fn start(app: &AppHandle) {
    let (Some(suite), Some(out)) = (
        std::env::var_os("RUNE_EVAL_SUITE"),
        std::env::var_os("RUNE_EVAL_OUT"),
    ) else {
        return;
    };
    let (suite, out) = (PathBuf::from(suite), PathBuf::from(out));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        log::info!("eval: running {} into {}", suite.display(), out.display());
        let code = execute(&app, &suite, &out).await;
        log::info!("eval: finished with exit code {code}");
        app.exit(code);
    });
}

fn refuse(out: &Path, why: &str) -> i32 {
    log::error!("eval: not starting: {why}");
    let _ = std::fs::create_dir_all(out);
    let _ = std::fs::write(out.join("preflight.txt"), format!("{why}\n"));
    2
}

async fn execute(app: &AppHandle, suite_path: &Path, out: &Path) -> i32 {
    // The suite is the only file this reads.
    let text = match std::fs::read_to_string(suite_path) {
        Ok(text) => text,
        Err(err) => {
            return refuse(
                out,
                &format!("could not read the suite {}: {err}", suite_path.display()),
            );
        }
    };
    let suite = match Suite::parse(&text) {
        Ok(suite) => suite,
        Err(why) => return refuse(out, &why),
    };
    if let Err(err) = std::fs::create_dir_all(out) {
        return refuse(out, &format!("could not make the output folder: {err}"));
    }
    let state = app.state::<AppState>();

    // Overrides live in memory only; settings.json is never written.
    {
        let mut settings = state.settings.locked();
        if let Some(fast_checks) = suite.config.fast_checks {
            settings.fast_checks = fast_checks;
        }
        if let Some(show_thinking) = suite.config.show_thinking {
            settings.show_thinking = show_thinking;
        }
        if let Some(model) = &suite.config.model {
            settings.model = model.clone();
        }
        settings.max_steps = suite.defaults.max_steps;
        // Off unless the suite says otherwise, so an earlier baseline stays comparable.
        settings.verify_mode = suite.verify_mode();
    }
    let effective = state.settings.locked().clone();

    let running = match check::Machine::process_names(&windows::System) {
        Ok(running) => running,
        Err(why) => {
            return refuse(
                out,
                &format!("cannot tell what is running ({why}); the evaluation needs Windows"),
            );
        }
    };
    let status = backend::check(&state.http, &effective, settings::api_key().as_deref()).await;
    if let Err(why) = preflight(
        &running,
        status.reachable,
        status.auth_ok,
        &status.missing_models,
        status.error.as_deref(),
    ) {
        return refuse(out, &why);
    }

    let env = json!({
        "model": effective.model,
        "settings": serde_json::from_str::<serde_json::Value>(&trace::settings_snapshot(&effective)).unwrap_or_default(),
        "app_version": crate::VERSION,
        "git_commit": option_env!("RUNE_GIT_COMMIT"),
        "date": crate::archive::iso(crate::archive::now()),
        "suite": suite.name,
        "suite_hash": trace::text_hash(&text),
        "repeats": suite.repeats,
    });
    if let Err(err) = std::fs::write(
        out.join("env.json"),
        serde_json::to_string_pretty(&env).unwrap_or_default(),
    ) {
        return refuse(out, &format!("could not write env.json: {err}"));
    }

    let fixtures = suite_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("fixtures");
    let Ok(data_dir) = app.path().app_data_dir() else {
        return refuse(out, "no app data folder, so the runs cannot be read back");
    };
    let database = trace::db_path(&data_dir);
    let results = out.join("results.jsonl");
    let mut any_problem = false;

    for (index, task) in suite.tasks.iter().enumerate() {
        for repeat in 1..=suite.repeats {
            let vars = Vars {
                work: out.join("work").join(&task.id).join(repeat.to_string()),
                run: format!("r{}x{repeat}-{}", index + 1, crate::archive::now()),
                fixtures: fixtures.clone(),
                fixtures_url: file_url(&fixtures),
            };
            let timeout =
                Duration::from_secs(task.timeout_secs.unwrap_or(suite.defaults.timeout_secs));
            let (result, housekeeping_failed) = run_one(
                app,
                &database,
                &task.resolve(&vars),
                repeat,
                out,
                &vars.work,
                timeout,
                suite.verify_mode(),
            )
            .await;
            log::info!(
                "eval: {} #{repeat}: outcome {} check {} passed {}",
                result.task,
                result.outcome,
                result.check_passed,
                result.passed
            );
            any_problem |= housekeeping_failed;
            let written = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&results)
                .and_then(|mut file| writeln!(file, "{}", result.to_line()));
            if let Err(err) = written {
                return refuse(out, &format!("could not write results.jsonl: {err}"));
            }
        }
    }
    i32::from(any_problem)
}

/// One run of one task, start to finish. Never fails: problems go in the result's `error`. The
/// second value says whether setup or teardown failed (which makes the whole run exit with 1).
#[allow(clippy::too_many_arguments)]
async fn run_one(
    app: &AppHandle,
    database: &Path,
    task: &Task,
    repeat: u32,
    out: &Path,
    work: &Path,
    timeout: Duration,
    verify_mode: VerifyMode,
) -> (RunResult, bool) {
    let roots = vec![work.to_path_buf(), out.to_path_buf()];
    let started = Instant::now();
    let mut problems: Vec<String> = Vec::new();
    let failed = |problems: Vec<String>, started: Instant| {
        (
            build_result(
                task,
                repeat,
                "",
                "setup_failed",
                false,
                &[],
                started.elapsed(),
                0,
                Some(problems.join("; ")),
                verification(verify_mode, None, false),
            ),
            true,
        )
    };

    if let Err(why) = task.confine(&roots) {
        return failed(vec![why], started);
    }
    if let Err(why) = steps::apply_all(&task.setup, &roots) {
        let _ = steps::apply_all(&task.teardown, &roots);
        return failed(vec![format!("setup: {why}")], started);
    }
    wait_until_idle(app).await;

    if let Err(err) = commands::begin_task(app, &task.prompt, false) {
        let _ = steps::apply_all(&task.teardown, &roots);
        return failed(vec![format!("could not start the task: {err}")], started);
    }
    let started = Instant::now();
    let run_id = app
        .state::<AppState>()
        .trace
        .open_run_id()
        .unwrap_or_default();

    // Watch it. A question is answered No (never by the harness approving anything); out of time means Stop.
    let mut denied = 0_u32;
    let mut timed_out = false;
    loop {
        tokio::time::sleep(POLL_EVERY).await;
        match poll_decision(events::agent_state(app), started.elapsed(), timeout) {
            Poll::Running => {}
            Poll::NeedsDenial => {
                if let Some((id, kind, text)) = gate::pending_question(app) {
                    log::info!("eval: answering No to a {kind}: {text}");
                    if gate::answer(app, &id, false) {
                        denied += 1;
                    }
                }
            }
            Poll::Finished => break,
            Poll::TimedOut => {
                timed_out = true;
                commands::stop(app.clone());
                break;
            }
        }
    }
    let duration = started.elapsed();
    wait_until_idle(app).await;

    // Judge it by the machine, then clean up, then read what the recorder wrote.
    tokio::time::sleep(BEFORE_CHECK).await;
    let check_passed = match check::evaluate(&task.check, &windows::System) {
        Ok(passed) => passed,
        Err(why) => {
            problems.push(format!("check: {why}"));
            false
        }
    };
    let mut teardown_failed = false;
    if let Err(why) = steps::apply_all(&task.teardown, &roots) {
        problems.push(format!("teardown: {why}"));
        teardown_failed = true;
    }
    let detail = read_run(database, &run_id).await;
    let (outcome, recorded_steps) = match (&detail, timed_out) {
        (_, true) => (
            "timeout".to_string(),
            detail
                .as_ref()
                .map(|detail| detail.steps.clone())
                .unwrap_or_default(),
        ),
        (Some(detail), false) => (
            detail
                .run
                .outcome
                .clone()
                .unwrap_or_else(|| "unknown".into()),
            detail.steps.clone(),
        ),
        (None, false) => ("unknown".to_string(), Vec::new()),
    };
    if detail.is_none() {
        problems.push("the run could not be read back from the trace database".into());
    }
    // A background check runs after the task. Wait for it here, off the task's clock, so it never
    // competes with the next task for the model server (and is not cancelled by that task starting).
    let mut stored = detail.as_ref().and_then(|detail| detail.verdict.clone());
    let mut waited_out = false;
    if verify_mode == VerifyMode::Background && !run_id.is_empty() && stored.is_none() {
        stored = wait_for_verdict(database, &run_id).await;
        waited_out = stored.is_none();
    }
    let result = build_result(
        task,
        repeat,
        &run_id,
        &outcome,
        check_passed,
        &recorded_steps,
        duration,
        denied,
        (!problems.is_empty()).then(|| problems.join("; ")),
        verification(verify_mode, stored.as_ref(), waited_out),
    );
    (result, teardown_failed)
}

/// Waits (up to a minute) for the verdict of a run to appear in the trace database.
async fn wait_for_verdict(database: &Path, run_id: &str) -> Option<VerdictRecord> {
    let started = Instant::now();
    while started.elapsed() < VERIFY_WAIT {
        if let Ok(Some(detail)) = store::get_run(database, run_id)
            && detail.verdict.is_some()
        {
            return detail.verdict;
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
    None
}

/// Waits for the agent to be idle (or done), then a few seconds more.
async fn wait_until_idle(app: &AppHandle) {
    let started = Instant::now();
    while events::agent_state(app).is_running() && started.elapsed() < IDLE_WAIT {
        tokio::time::sleep(POLL_EVERY).await;
    }
    tokio::time::sleep(BETWEEN_RUNS).await;
}

/// Reads the finished run from the trace database, waiting a little for the recorder to write its end.
async fn read_run(database: &Path, run_id: &str) -> Option<store::RunDetail> {
    if run_id.is_empty() {
        return None;
    }
    let started = Instant::now();
    loop {
        if let Ok(Some(detail)) = store::get_run(database, run_id)
            && detail.run.ended_utc.is_some()
        {
            return Some(detail);
        }
        if started.elapsed() >= TRACE_WAIT {
            return store::get_run(database, run_id).ok().flatten();
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[cfg(test)]
mod tests;
