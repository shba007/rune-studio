use tauri::{AppHandle, State};

use crate::{
    backend::{self, BackendStatus},
    error::{AppError, AppResult},
    events::{self, AgentState, AppState},
    hotkeys::{self, Hotkeys},
    settings::{self, Settings},
    window,
};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().expect("settings lock poisoned").clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> AppResult<()> {
    Hotkeys::validate(&settings).map_err(AppError::Invalid)?;
    settings::save(&app, &settings)?;

    let next = Hotkeys::from_settings(&settings);
    *state.settings.lock().expect("settings lock poisoned") = settings;
    let previous = std::mem::replace(&mut *state.hotkeys.lock().expect("hotkeys lock poisoned"), next.clone());
    if previous != next {
        hotkeys::apply(&app, Some(&previous), &next).map_err(AppError::Invalid)?;
    }
    Ok(())
}

#[tauri::command]
pub fn has_api_key() -> bool {
    settings::api_key().is_some()
}

#[tauri::command]
pub fn set_api_key(key: String) -> AppResult<()> {
    settings::set_api_key(key.trim())
}

#[tauri::command]
pub async fn check_backend(state: State<'_, AppState>) -> AppResult<BackendStatus> {
    let settings = state.settings.lock().expect("settings lock poisoned").clone();
    Ok(backend::check(&state.http, &settings, settings::api_key().as_deref()).await)
}

#[tauri::command]
pub fn set_mode(app: AppHandle, mode: window::Mode) -> AppResult<()> {
    Ok(window::set_mode(&app, mode)?)
}

#[tauri::command]
pub fn stop(app: AppHandle) {
    if events::agent_state(&app) != AgentState::Idle {
        events::set_agent_state(&app, AgentState::Stopped);
    }
}

#[tauri::command]
pub fn start_task(_text: String) -> AppResult<()> {
    Err(AppError::NotImplemented("start_task (M2)"))
}

#[tauri::command]
pub fn steer(_text: String) -> AppResult<()> {
    Err(AppError::NotImplemented("steer (M2)"))
}

#[tauri::command]
pub fn toggle_listening() -> AppResult<()> {
    Err(AppError::NotImplemented("toggle_listening (M5)"))
}

#[tauri::command]
pub fn confirm(_id: String, _approved: bool) -> AppResult<()> {
    Err(AppError::NotImplemented("confirm (M3)"))
}

/// Dev-only: drives the state machine by hand so the UI states and click-through
/// can be checked before the agent loop exists.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn debug_set_state(app: AppHandle, state: AgentState) {
    events::set_agent_state(&app, state);
}
