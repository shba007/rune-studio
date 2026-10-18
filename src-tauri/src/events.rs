use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{hotkeys::Hotkeys, settings::Settings, window};

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
    let app_state = app.state::<AppState>();
    *app_state.agent_state.lock().expect("agent state lock poisoned") = state;
    window::set_click_through(app, state == AgentState::Acting);
    let _ = app.emit(STATE_CHANGED, StateChanged { state });
}

pub fn agent_state(app: &AppHandle) -> AgentState {
    *app.state::<AppState>().agent_state.lock().expect("agent state lock poisoned")
}
