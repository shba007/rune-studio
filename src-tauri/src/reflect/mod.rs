//! The idle reflection shell: after the user has been away for `idleMinutes`, the pill asks "Rune
//! wants to review its recent tasks" with Start and Deny and a 30 second countdown (no answer starts
//! it). While it runs the agent state is `Reflecting`, the ring shows `reflection_progress`, and any
//! task start, Stop or outside agent cancels it within a fraction of a second.
//!
//! In R5 the job is a stub (`job`): it needs no screen and no model. The pure parts (`idle`,
//! `prompt`, `job`) are unit tested; the wiring below needs an `AppHandle` and a window.

pub mod idle;
pub mod job;
pub mod prompt;

use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    events::{self, AgentState, AppState},
    exec::CancelToken,
    sync::LockExt,
    trace::{self, ReflectionRecord},
};
use idle::{IdleClock, Input};
use prompt::{ANSWER_SECS, Do, Phase};

pub const PROMPT: &str = "reflection_prompt";
pub const RESOLVED: &str = "reflection_resolved";
pub const PROGRESS: &str = "reflection_progress";

/// How often the watcher looks.
const TICK: Duration = Duration::from_secs(2);
/// How long a stub stage lasts.
const STAGE_TIME: Duration = Duration::from_secs(1);

/// Milliseconds since the app started, on a clock that does not jump.
fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub struct State {
    clock: IdleClock,
    phase: Phase,
    /// Cancels the job in flight; a task start or Stop cancels it (`AppState::begin_run`, `stop_current`).
    pub cancel: CancelToken,
}

impl Default for State {
    fn default() -> Self {
        Self {
            clock: IdleClock::new(now_ms()),
            phase: Phase::Hidden,
            cancel: CancelToken::default(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub timeout_secs: u64,
    /// Milliseconds from now until the time runs out (the window counts on its own clock).
    pub ends_in_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct Resolved {
    pub started: bool,
}

#[derive(Clone, Serialize)]
pub struct Progress {
    pub stage: String,
    pub done: u32,
    pub total: u32,
}

/// The user did something: the idle stretch starts again.
pub fn touch(app: &AppHandle, input: Input) {
    app.state::<AppState>()
        .reflect
        .locked()
        .clock
        .touch(now_ms(), input);
}

/// Starts the watcher that decides when to ask. Safe to call once at startup.
pub fn start_watcher(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            tick(&app);
        }
    });
}

/// Whether something other than idleness is going on right now.
fn busy(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    events::agent_state(app).is_running()
        || state.lease.locked().is_held()
        || crate::analysis::state::in_flight(state.analysis.locked().status)
        || app
            .get_webview_window(crate::analysis::window::LABEL)
            .is_some()
        || state.mic.locked().is_some()
}

fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (enabled, idle_minutes) = {
        let settings = state.settings.locked();
        (
            settings.reflection_enabled,
            settings.idle_minutes.clamp(1, 120),
        )
    };
    let now = now_ms();
    let is_busy = busy(app);
    let mut reflect = state.reflect.locked();
    if !enabled {
        // Off: nothing is ever shown, and re-enabling starts a full idle period.
        reflect.clock.restart(now);
        if matches!(reflect.phase, Phase::Shown { .. }) {
            let (phase, action) = reflect.phase.preempt();
            reflect.phase = phase;
            drop(reflect);
            act(app, action);
        }
        return;
    }
    // Something real started while the prompt was up: withdraw it.
    if is_busy && matches!(reflect.phase, Phase::Shown { .. }) {
        let (phase, action) = reflect.phase.preempt();
        reflect.phase = phase;
        drop(reflect);
        act(app, action);
        return;
    }
    // The timeout starts reflection.
    let (phase, action) = reflect.phase.tick(now);
    if action != Do::Nothing {
        reflect.phase = phase;
        drop(reflect);
        act(app, action);
        return;
    }
    if reflect.phase == Phase::Hidden
        && reflect
            .clock
            .tick(now, u64::from(idle_minutes) * 60_000, is_busy)
    {
        let (phase, action) = reflect.phase.show(now);
        reflect.phase = phase;
        drop(reflect);
        act(app, action);
    }
}

/// The user's answer to the prompt (Start or Deny).
pub fn answer(app: &AppHandle, start: bool) {
    let state = app.state::<AppState>();
    let mut reflect = state.reflect.locked();
    let (phase, action) = reflect.phase.answer(start);
    reflect.phase = phase;
    drop(reflect);
    act(app, action);
}

/// Does what a state step asked for.
fn act(app: &AppHandle, action: Do) {
    let state = app.state::<AppState>();
    match action {
        Do::Nothing => {}
        Do::ShowPrompt { .. } => {
            let _ = app.emit(
                PROMPT,
                Prompt {
                    timeout_secs: ANSWER_SECS,
                    ends_in_ms: ANSWER_SECS * 1000,
                },
            );
            // The pill may be hidden; the card must be seen, but never take the keyboard from the window in use.
            if let Some(pill) = app.get_webview_window(crate::window::MAIN) {
                crate::window::show_without_focus(&pill);
            }
        }
        Do::Denied => {
            state.reflect.locked().clock.restart(now_ms());
            let _ = app.emit(RESOLVED, Resolved { started: false });
        }
        Do::Start => {
            let cancel = CancelToken::default();
            state.reflect.locked().cancel = cancel.clone();
            let _ = app.emit(RESOLVED, Resolved { started: true });
            events::set_agent_state(app, AgentState::Reflecting);
            let app = app.clone();
            tauri::async_runtime::spawn(async move { run(&app, cancel).await });
        }
        Do::Preempted | Do::Finished => {}
    }
}

/// The job: runs the stub, records it and gives the state back.
async fn run(app: &AppHandle, cancel: CancelToken) {
    let started_utc = trace::now_utc();
    let id = format!("reflect-{}", crate::archive::now());
    let emit_progress = |stage: &str, done: u32, total: u32| {
        let _ = app.emit(
            PROGRESS,
            Progress {
                stage: stage.into(),
                done,
                total,
            },
        );
    };
    let outcome = job::run(&cancel, STAGE_TIME, emit_progress).await;
    let (label, stages) = match outcome {
        job::Outcome::Completed => ("completed", job::STAGES.len() as i64),
        job::Outcome::Preempted { stages } => ("preempted", i64::from(stages)),
    };
    let state = app.state::<AppState>();
    state.trace.reflection(ReflectionRecord {
        id,
        started_utc,
        ended_utc: Some(trace::now_utc()),
        outcome: label.into(),
        stages,
    });
    // Free the phase (a pre-empting task may already have done it) and restart the idle clock.
    {
        let mut reflect = state.reflect.locked();
        let (phase, _) = if label == "completed" {
            reflect.phase.finish()
        } else {
            reflect.phase.preempt()
        };
        reflect.phase = phase;
        reflect.clock.restart(now_ms());
    }
    // Only if nothing else has taken the state: a task that pre-empted us has already moved it on.
    if events::agent_state(app) == AgentState::Reflecting {
        events::set_agent_state(app, AgentState::Idle);
    }
    let _ = app.emit(
        PROGRESS,
        Progress {
            stage: String::new(),
            done: 0,
            total: 0,
        },
    );
}
