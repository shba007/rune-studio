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
    // The command line (`rune-studio enable`) can change the outside-agent switch behind our back.
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
pub fn set_mode(app: AppHandle, mode: window::Mode) -> AppResult<()> {
    Ok(window::set_mode(&app, mode)?)
}

#[tauri::command]
pub fn stop(app: AppHandle) {
    // Always cancel, even from Idle: Stop must interrupt typing the instant it is pressed.
    app.state::<AppState>().cancel.cancel();
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
    }
}

#[tauri::command]
pub fn start_task(app: AppHandle, state: State<'_, AppState>, text: String) -> AppResult<()> {
    let task = text.trim();
    if task.is_empty() {
        return Err(AppError::Invalid("Type or say what you want done".into()));
    }
    if matches!(
        events::agent_state(&app),
        AgentState::Planning | AgentState::AwaitingConfirm | AgentState::Acting | AgentState::Observing
    ) {
        return Err(AppError::Invalid("A task is already running".into()));
    }
    if state.lease.lock().expect("lease lock poisoned").is_held() {
        return Err(AppError::Invalid("An outside agent is in control".into()));
    }
    // Starting a task ends listening: the microphone must not stay open behind it.
    drop(state.mic.lock().expect("mic lock poisoned").take());
    agent::start(&app, task.to_string());
    Ok(())
}

#[tauri::command]
pub fn steer(_text: String) -> AppResult<()> {
    Err(AppError::NotImplemented("steer (M2)"))
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

    if matches!(
        events::agent_state(&app),
        AgentState::Planning | AgentState::AwaitingConfirm | AgentState::Acting | AgentState::Observing
    ) {
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
pub fn confirm(_id: String, _approved: bool) -> AppResult<()> {
    Err(AppError::NotImplemented("confirm (M3)"))
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
        // The `rune-studio` command sits next to this windowed app.
        let exe = std::env::current_exe()
            .map(|path| path.with_file_name("rune-studio.exe").display().to_string())
            .unwrap_or_else(|_| "rune-studio".into());
        let command = format!("claude mcp add --scope user rune-studio -- \"{exe}\" mcp");
        BridgeInfo { available: true, url, command }
    }
    #[cfg(not(feature = "agent-bridge"))]
    {
        BridgeInfo { available: false, url: String::new(), command: String::new() }
    }
}
