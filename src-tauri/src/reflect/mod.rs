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

const TICK: Duration = Duration::from_secs(2);
const STAGE_TIME: Duration = Duration::from_secs(1);

fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub struct State {
    clock: IdleClock,
    phase: Phase,
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

pub fn touch(app: &AppHandle, input: Input) {
    app.state::<AppState>()
        .reflect
        .locked()
        .clock
        .touch(now_ms(), input);
}

pub fn start_watcher(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            tick(&app);
        }
    });
}

fn busy(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    events::agent_state(app).is_running()
        || state.lease.locked().is_held()
        || state.analysis.locked().status != crate::analysis::report::Status::Idle
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
        reflect.clock.restart(now);
        if matches!(reflect.phase, Phase::Shown { .. }) {
            let (phase, action) = reflect.phase.preempt();
            reflect.phase = phase;
            drop(reflect);
            act(app, action);
        }
        return;
    }
    if is_busy && matches!(reflect.phase, Phase::Shown { .. }) {
        let (phase, action) = reflect.phase.preempt();
        reflect.phase = phase;
        drop(reflect);
        act(app, action);
        return;
    }
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

pub fn answer(app: &AppHandle, start: bool) {
    let state = app.state::<AppState>();
    let mut reflect = state.reflect.locked();
    let (phase, action) = reflect.phase.answer(start);
    reflect.phase = phase;
    drop(reflect);
    act(app, action);
}

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
