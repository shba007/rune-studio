use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{exec::CancelToken, hotkeys::Hotkeys, overlay, settings::Settings, voice::capture::Capture, window};

pub const STATE_CHANGED: &str = "state_changed";
pub const SUMMON: &str = "summon";
pub const OPEN_SETTINGS: &str = "open_settings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Listening,
    Planning,
    AwaitingConfirm,
    Acting,
    Observing,
    Done,
    Failed,
    Stopped,
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub hotkeys: Mutex<Hotkeys>,
    /// Present while the microphone is open.
    pub mic: Mutex<Option<Capture>>,
    /// Set by every Stop; checked by the executor between steps.
    pub cancel: CancelToken,
    pub agent_state: Mutex<AgentState>,
    pub http: reqwest::Client,
}

#[derive(Clone, Serialize)]
struct StateChanged {
    state: AgentState,
}

/// Single place the agent state changes. Keeps the UI event and the window's
/// click-through behaviour in sync: the pill must never block a click while acting.
pub fn set_agent_state(app: &AppHandle, state: AgentState) {
    apply_state(app, state, true);
}

/// Like `set_agent_state`, but never turns click-through on. For the dev state switcher, which
/// forces states by hand and must not trap the user in a window that ignores the mouse.
#[cfg(debug_assertions)]
pub fn preview_agent_state(app: &AppHandle, state: AgentState) {
    apply_state(app, state, false);
}

fn apply_state(app: &AppHandle, state: AgentState, allow_click_through: bool) {
    let app_state = app.state::<AppState>();
    *app_state.agent_state.lock().expect("agent state lock poisoned") = state;
    window::set_click_through(app, allow_click_through && state == AgentState::Acting);
    overlay::sync(app, state);
    let _ = app.emit(STATE_CHANGED, StateChanged { state });
}

pub fn agent_state(app: &AppHandle) -> AgentState {
    *app.state::<AppState>().agent_state.lock().expect("agent state lock poisoned")
}

pub const STEP_STARTED: &str = "step_started";
pub const STEP_FINISHED: &str = "step_finished";
pub const TASK_DONE: &str = "task_done";

#[derive(Clone, Serialize)]
pub struct StepStarted {
    pub step: u32,
    pub tool: &'static str,
    pub reason: String,
    pub text: String,
}

#[derive(Clone, Serialize)]
pub struct StepFinished {
    pub step: u32,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct TaskDone {
    pub summary: String,
    pub success: bool,
}
