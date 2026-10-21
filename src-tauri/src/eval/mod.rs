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

const POLL_EVERY: Duration = Duration::from_millis(500);
const BETWEEN_RUNS: Duration = Duration::from_secs(3);
const BEFORE_CHECK: Duration = Duration::from_secs(1);
const IDLE_WAIT: Duration = Duration::from_secs(60);
const TRACE_WAIT: Duration = Duration::from_secs(10);
const VERIFY_WAIT: Duration = Duration::from_secs(60);

#[derive(Debug, PartialEq, Eq)]
pub enum Poll {
    Running,
    NeedsDenial,
    Finished,
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

const USAGE_BELIEVED_FROM: i64 = 2;

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

#[derive(Debug, Clone, PartialEq)]
pub struct Verification {
    pub mode: &'static str,
    pub verdict: Option<String>,
    pub verify_ms: Option<i64>,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunResult {
    pub task: String,
    pub repeat: u32,
    pub run_id: String,
    pub outcome: String,
    pub self_reported_success: bool,
    pub check_passed: bool,
    pub expect: &'static str,
    pub passed: bool,
    pub steps: usize,
    pub duration_ms: u64,
    pub decide_ms_total: i64,
    pub observe_ms_total: i64,
    pub act_ms_total: i64,
    pub completion_tokens_total: i64,
    pub tokens_estimated: bool,
    pub denied_prompts: u32,
    pub verify_mode: &'static str,
    pub verdict: Option<String>,
    pub verify_ms: Option<i64>,
    pub challenged: bool,
    pub error: Option<String>,
}

impl RunResult {
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

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

async fn wait_until_idle(app: &AppHandle) {
    let started = Instant::now();
    while events::agent_state(app).is_running() && started.elapsed() < IDLE_WAIT {
        tokio::time::sleep(POLL_EVERY).await;
    }
    tokio::time::sleep(BETWEEN_RUNS).await;
}

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
