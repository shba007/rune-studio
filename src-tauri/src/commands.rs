use tauri::{AppHandle, Manager, State};

use crate::{
    agent,
    backend::{self, BackendStatus},
    error::{AppError, AppResult},
    events::{self, AgentState, AppState},
    hotkeys::{self, Hotkeys},
    settings::{self, Settings},
    voice::capture::Capture,
    window,
};

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Settings {
    // The command line (`rune-studio-cli enable`) can change the outside-agent switch behind our back.
    let on_disk = settings::load(&app).external_agent;
    let mut settings = state.settings.lock().expect("settings lock poisoned");
    settings.external_agent = on_disk;
    settings.clone()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> AppResult<()> {
    Hotkeys::validate(&settings).map_err(AppError::Invalid)?;
    settings::save(&app, &settings)?;

    let next = Hotkeys::from_settings(&settings);
    // Turning the switch back on is how the user re-arms an agent that Stop locked out.
    let was_on = state.settings.lock().expect("settings lock poisoned").external_agent;
    if settings.external_agent && !was_on {
        state.lease.lock().expect("lease lock poisoned").rearm();
    }
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
pub fn fit_height(app: AppHandle, height: f64) -> AppResult<()> {
    Ok(window::fit_height(&app, height)?)
}

#[tauri::command]
pub fn set_mode(app: AppHandle, mode: window::Mode) -> AppResult<()> {
    Ok(window::set_mode(&app, mode)?)
}

/// Where the pill is on the desktop (physical pixels), so the full-screen overlay can put the
/// speech bubble at the orb's head. `None` if the window is hidden.
#[derive(serde::Serialize)]
pub struct PillRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn pill_rect(app: AppHandle) -> Option<PillRect> {
    let window = app.get_webview_window(window::MAIN)?;
    if !window.is_visible().unwrap_or(false) {
        return None;
    }
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(PillRect { x: position.x, y: position.y, width: size.width, height: size.height })
}

#[tauri::command]
pub fn stop(app: AppHandle) {
    // Always cancel, even from Idle: Stop must interrupt typing the instant it is pressed.
    app.state::<AppState>().stop_current();
    // An outside agent loses control and may not simply start again.
    let had_agent = {
        let state = app.state::<AppState>();
        let mut lease = state.lease.lock().expect("lease lock poisoned");
        let had = lease.is_held();
        lease.revoke();
        had
    };
    if had_agent {
        let _ = tauri::Emitter::emit(&app, events::EXTERNAL_AGENT, events::ExternalAgent { agent: None });
    }
    if events::agent_state(&app) != AgentState::Idle {
        events::set_agent_state(&app, AgentState::Stopped);
        crate::voice::resume_listening_soon(&app);
    }
}

#[tauri::command]
pub fn start_task(app: AppHandle, text: String) -> AppResult<()> {
    begin_task(&app, &text, false)
}

/// Starts a task. `keep_mic` leaves the microphone open (a task started by voice stays
/// listening, so the user can say stop, answer a question or steer); typing a task closes it.
pub fn begin_task(app: &AppHandle, text: &str, keep_mic: bool) -> AppResult<()> {
    begin_task_planned(app, text, keep_mic, None)
}

/// `begin_task` with the plan already made (see `agent::start_planned`).
pub fn begin_task_planned(app: &AppHandle, text: &str, keep_mic: bool, plan: Option<Vec<String>>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let task = text.trim();
    state.steer.lock().expect("steer lock poisoned").clear();
    if task.is_empty() {
        return Err(AppError::Invalid("Type or say what you want done".into()));
    }
    if events::agent_state(&app).is_running() {
        return Err(AppError::Invalid("A task is already running".into()));
    }
    if state.lease.lock().expect("lease lock poisoned").is_held() {
        return Err(AppError::Invalid("An outside agent is in control".into()));
    }
    // Starting a task ends listening: the microphone must not stay open behind it.
    if !keep_mic {
        drop(state.mic.lock().expect("mic lock poisoned").take());
    }
    agent::start_planned(app, task.to_string(), plan);
    Ok(())
}

/// Words for the running agent: it reads them before its next step. Returns false if there was
/// nothing worth keeping.
pub fn push_steer(app: &AppHandle, text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    app.state::<AppState>().steer.lock().expect("steer lock poisoned").push(text.chars().take(300).collect());
    true
}

#[tauri::command]
pub fn steer(app: AppHandle, text: String) -> AppResult<()> {
    if !events::agent_state(&app).is_running() {
        return Err(AppError::Invalid("There is no running task to give instructions to".into()));
    }
    if push_steer(&app, &text) { Ok(()) } else { Err(AppError::Invalid("Say what the agent should do differently".into())) }
}

/// Closes the microphone (and leaves the listening state).
pub fn close_microphone(app: &AppHandle) {
    let state = app.state::<AppState>();
    let was_open = state.mic.lock().expect("mic lock poisoned").take().is_some();
    if was_open && events::agent_state(app) == AgentState::Listening {
        events::set_agent_state(app, AgentState::Idle);
    }
}

/// Opens or closes the microphone. Only the live level is wired up so far; speech
/// segmentation and transcription arrive with M5.
#[tauri::command]
pub async fn toggle_listening(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if state.mic.lock().expect("mic lock poisoned").take().is_some() {
        if events::agent_state(&app) == AgentState::Listening {
            events::set_agent_state(&app, AgentState::Idle);
        }
        return Ok(());
    }

    if events::agent_state(&app).is_running() {
        return Err(AppError::Invalid("Stop the current task before listening".into()));
    }

    let handle = app.clone();
    let capture = tauri::async_runtime::spawn_blocking(move || Capture::start(handle))
        .await
        .map_err(|err| AppError::Invalid(err.to_string()))?
        .map_err(AppError::Invalid)?;
    *state.mic.lock().expect("mic lock poisoned") = Some(capture);
    events::set_agent_state(&app, AgentState::Listening);
    Ok(())
}

#[tauri::command]
pub fn confirm(app: AppHandle, id: String, approved: bool) -> AppResult<()> {
    if crate::safety::gate::answer(&app, &id, approved) {
        Ok(())
    } else {
        Err(AppError::Invalid("There is nothing waiting for an answer".into()))
    }
}

/// Dev-only: drives the state machine by hand so the UI states and click-through
/// can be checked before the agent loop exists.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn debug_set_state(app: AppHandle, state: AgentState) {
    events::preview_agent_state(&app, state);
}

/// Name of the outside agent currently in control, for the overlay's label.
#[tauri::command]
pub fn get_control_label(state: State<'_, AppState>) -> Option<String> {
    state.lease.lock().expect("lease lock poisoned").agent().map(str::to_string)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeInfo {
    /// Whether this build has the bridge at all.
    pub available: bool,
    pub url: String,
    /// A ready-to-paste command that connects Claude Code. It carries no secret: the app itself
    /// supplies the token when an agent connects.
    pub command: String,
}

/// What Settings shows for the External agent group.
#[tauri::command]
pub fn bridge_info() -> BridgeInfo {
    #[cfg(feature = "agent-bridge")]
    {
        let url = crate::bridge::server::url();
        // The `rune-studio-cli` command sits next to this windowed app.
        let exe = std::env::current_exe()
            .map(|path| path.with_file_name(crate::bridge::exe_name("rune-studio-cli")).display().to_string())
            .unwrap_or_else(|_| "rune-studio".into());
        let command = format!("claude mcp add --scope user rune-studio -- \"{exe}\" mcp");
        BridgeInfo { available: true, url, command }
    }
    #[cfg(not(feature = "agent-bridge"))]
    {
        BridgeInfo { available: false, url: String::new(), command: String::new() }
    }
}
