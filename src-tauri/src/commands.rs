use crate::sync::LockExt;
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
    let on_disk = settings::load(&app).external_agent;
    let mut settings = state.settings.locked();
    settings.external_agent = on_disk;
    settings.clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<()> {
    Hotkeys::validate(&settings).map_err(AppError::Invalid)?;
    settings::save(&app, &settings)?;
    crate::hub::changed(&app);

    let next = Hotkeys::from_settings(&settings);
    let was_on = state.settings.locked().external_agent;
    if settings.external_agent && !was_on {
        state.lease.locked().rearm();
    }
    let demo_changed = state.settings.locked().demo_mode != settings.demo_mode;
    *state.settings.locked() = settings;
    if demo_changed {
        crate::window::apply_demo_mode(&app);
    }
    let previous = std::mem::replace(&mut *state.hotkeys.locked(), next.clone());
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
pub fn set_api_key(app: AppHandle, key: String) -> AppResult<()> {
    settings::set_api_key(key.trim())?;
    crate::hub::changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle, section: Option<String>) -> AppResult<()> {
    crate::hub::open(&app, section.as_deref())
        .map_err(|err| AppError::Invalid(format!("Could not open Settings: {err}")))
}

#[tauri::command]
pub async fn check_backend(state: State<'_, AppState>) -> AppResult<BackendStatus> {
    let settings = state.settings.locked().clone();
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
    Some(PillRect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

#[tauri::command]
pub fn stop(app: AppHandle) {
    app.state::<AppState>().stop_current();
    let had_agent = {
        let state = app.state::<AppState>();
        let mut lease = state.lease.locked();
        let had = lease.is_held();
        lease.revoke();
        had
    };
    if had_agent {
        events::set_external(&app, None);
    }
    crate::archive::end_session(None, "stopped", "");
    app.state::<AppState>().trace.end_open_run("stopped", "");
    if events::agent_state(&app) != AgentState::Idle {
        events::set_agent_state(&app, AgentState::Stopped);
        crate::voice::resume_listening_soon(&app);
    }
}

#[tauri::command]
pub fn start_task(app: AppHandle, text: String) -> AppResult<()> {
    begin_task(&app, &text, false)
}

pub fn begin_task(app: &AppHandle, text: &str, keep_mic: bool) -> AppResult<()> {
    begin_task_planned(app, text, keep_mic, None)
}

pub fn begin_task_planned(
    app: &AppHandle,
    text: &str,
    keep_mic: bool,
    plan: Option<Vec<String>>,
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let task = text.trim();
    crate::reflect::touch(app, crate::reflect::idle::Input::Task);
    state.steer.locked().clear();
    if task.is_empty() {
        return Err(AppError::Invalid("Type or say what you want done".into()));
    }
    if events::agent_state(app).is_running() {
        return Err(AppError::Invalid("A task is already running".into()));
    }
    if state.lease.locked().is_held() {
        return Err(AppError::Invalid("An outside agent is in control".into()));
    }
    if crate::analysis::state::in_flight(state.analysis.locked().status) {
        return Err(AppError::Invalid(
            "A screen analysis is still running".into(),
        ));
    }
    if !keep_mic {
        drop(state.mic.locked().take());
    }
    agent::start_planned(app, task.to_string(), plan);
    Ok(())
}

pub fn push_steer(app: &AppHandle, text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    app.state::<AppState>()
        .steer
        .locked()
        .push(text.chars().take(300).collect());
    true
}

#[tauri::command]
pub fn steer(app: AppHandle, text: String) -> AppResult<()> {
    if !events::agent_state(&app).is_running() {
        return Err(AppError::Invalid(
            "There is no running task to give instructions to".into(),
        ));
    }
    if push_steer(&app, &text) {
        Ok(())
    } else {
        Err(AppError::Invalid(
            "Say what the agent should do differently".into(),
        ))
    }
}

pub fn close_microphone(app: &AppHandle) {
    let state = app.state::<AppState>();
    let was_open = state.mic.locked().take().is_some();
    if was_open && events::agent_state(app) == AgentState::Listening {
        events::set_agent_state(app, AgentState::Idle);
    }
}

#[tauri::command]
pub async fn toggle_listening(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if state.mic.locked().take().is_some() {
        if events::agent_state(&app) == AgentState::Listening {
            events::set_agent_state(&app, AgentState::Idle);
        }
        return Ok(());
    }

    if events::agent_state(&app).is_running() {
        return Err(AppError::Invalid(
            "Stop the current task before listening".into(),
        ));
    }

    let (settings, http) = (state.settings.locked().clone(), state.http.clone());
    let voice_model = settings.model_for_role(crate::model::role::ModelRole::Voice);
    let profile = crate::model::profile::resolve_profile(voice_model, None, None);
    if let Some(reason) = crate::voice::seam::voice_disabled_reason(voice_model, &profile) {
        return Err(AppError::Invalid(reason));
    }
    tauri::async_runtime::spawn(async move {
        let status = backend::check(&http, &settings, settings::api_key().as_deref()).await;
        if backend::lacks_audio(&status.model_capabilities) {
            log::warn!("the configured model does not list Audio: voice will not be understood");
        }
    });

    let handle = app.clone();
    let capture = tauri::async_runtime::spawn_blocking(move || Capture::start(handle))
        .await
        .map_err(|err| AppError::Invalid(err.to_string()))?
        .map_err(AppError::Invalid)?;
    *state.mic.locked() = Some(capture);
    events::set_agent_state(&app, AgentState::Listening);
    Ok(())
}

#[tauri::command]
pub fn confirm(app: AppHandle, id: String, approved: bool) -> AppResult<()> {
    if crate::safety::gate::answer(&app, &id, approved) {
        Ok(())
    } else {
        Err(AppError::Invalid(
            "There is nothing waiting for an answer".into(),
        ))
    }
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn debug_set_state(app: AppHandle, state: AgentState) {
    events::preview_agent_state(&app, state);
}

#[tauri::command]
pub fn get_control_label(state: State<'_, AppState>) -> Option<String> {
    state.lease.locked().agent().map(str::to_string)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeInfo {
    pub available: bool,
    pub url: String,
    pub command: String,
}

#[tauri::command]
pub fn bridge_info() -> BridgeInfo {
    #[cfg(feature = "agent-bridge")]
    {
        let url = crate::bridge::server::url();
        let exe = std::env::current_exe()
            .map(|path| {
                path.with_file_name(crate::bridge::exe_name("rune-studio-cli"))
                    .display()
                    .to_string()
            })
            .unwrap_or_else(|_| "rune-studio".into());
        let command =
            format!("claude mcp add --scope user rune-studio -- \"{exe}\" mcp --agent claude");
        BridgeInfo {
            available: true,
            url,
            command,
        }
    }
    #[cfg(not(feature = "agent-bridge"))]
    {
        BridgeInfo {
            available: false,
            url: String::new(),
            command: String::new(),
        }
    }
}

#[tauri::command]
pub async fn analyze_screen(app: AppHandle, question: Option<String>) -> AppResult<()> {
    crate::analysis::run::run(&app, question.as_deref().unwrap_or(""), true).await
}

#[tauri::command]
pub async fn analyze_again(app: AppHandle, question: String) -> AppResult<()> {
    crate::analysis::run::run(&app, &question, false).await
}

#[tauri::command]
pub fn analysis_state(state: State<'_, AppState>) -> crate::analysis::state::Snapshot {
    state.analysis.locked().snapshot()
}

#[tauri::command]
pub fn cancel_analysis(app: AppHandle) {
    crate::analysis::run::cancel(&app);
}

#[tauri::command]
pub fn close_analysis(app: AppHandle) {
    crate::analysis::run::close(&app);
}

#[tauri::command]
pub fn select_finding(app: AppHandle, n: Option<u8>) {
    crate::analysis::run::select(&app, n);
}

#[tauri::command]
pub fn overlay_info(window: tauri::WebviewWindow) -> crate::overlay::OverlayInfo {
    crate::overlay::info(&window)
}

#[tauri::command]
pub fn external_agent(state: State<'_, AppState>) -> events::ExternalAgent {
    state
        .external
        .locked()
        .as_ref()
        .map_or_else(events::ExternalAgent::none, events::ExternalAgent::of)
}

#[tauri::command]
pub fn reflection_answer(app: AppHandle, start: bool) {
    crate::reflect::answer(&app, start);
}

#[tauri::command]
pub fn session_feedback(app: AppHandle, run_id: String, complete: bool, reason: Option<String>) {
    let reason = reason
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    let data = serde_json::json!({ "complete": complete, "reason": reason });
    app.state::<AppState>()
        .trace
        .event(&run_id, "user_feedback", &data.to_string());
}

#[tauri::command]
pub fn pill_activity(app: AppHandle) {
    crate::reflect::touch(&app, crate::reflect::idle::Input::Pill);
}

fn trace_db(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    app.state::<AppState>()
        .trace
        .db_path()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| AppError::Invalid("The flight record is not available.".into()))
}

fn read_error(err: impl std::fmt::Display) -> AppError {
    AppError::Invalid(format!("Could not read the flight record: {err}"))
}

#[tauri::command]
pub fn flight_runs(
    app: AppHandle,
    limit: Option<usize>,
    before: Option<String>,
) -> AppResult<Vec<crate::flight::FlightRun>> {
    crate::flight::runs(
        &trace_db(&app)?,
        limit.unwrap_or(30).min(200),
        before.as_deref(),
    )
    .map_err(read_error)
}

#[tauri::command]
pub fn flight_run(app: AppHandle, id: String) -> AppResult<Option<crate::flight::FlightHeader>> {
    crate::flight::header(&trace_db(&app)?, &id).map_err(read_error)
}

#[tauri::command]
pub fn flight_steps(app: AppHandle, id: String) -> AppResult<Vec<serde_json::Value>> {
    crate::flight::steps(&trace_db(&app)?, &id).map_err(read_error)
}

#[tauri::command]
pub fn flight_events(app: AppHandle, id: String) -> AppResult<Vec<serde_json::Value>> {
    crate::flight::events(&trace_db(&app)?, &id).map_err(read_error)
}

#[tauri::command]
pub fn flight_calls(
    app: AppHandle,
    id: String,
    step: Option<i64>,
) -> AppResult<Vec<crate::flight::FlightCall>> {
    crate::flight::calls(&trace_db(&app)?, &id, step).map_err(read_error)
}

#[tauri::command]
pub fn flight_call_body(app: AppHandle, call_id: i64) -> AppResult<Option<(String, String)>> {
    crate::flight::call_bodies(&trace_db(&app)?, call_id).map_err(read_error)
}

#[tauri::command]
pub fn flight_file(app: AppHandle, id: String, relative: String) -> AppResult<String> {
    let dir = crate::flight::session_dir(&trace_db(&app)?, &id)
        .map_err(read_error)?
        .ok_or_else(|| AppError::Invalid("This run has no session folder.".into()))?;
    let path = crate::flight::files::resolve(&dir, &relative).map_err(|refused| {
        AppError::Invalid(format!("That file cannot be opened: {}.", refused.why()))
    })?;
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|err| AppError::Invalid(format!("Could not open the file: {err}")))?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn open_flight(app: AppHandle, run_id: Option<String>) -> AppResult<()> {
    crate::flight::open(&app, run_id.as_deref())
        .map_err(|err| AppError::Invalid(format!("Could not open the Flight Player: {err}")))
}

#[tauri::command]
pub fn flight_reveal(app: AppHandle, id: String) -> AppResult<()> {
    let dir = crate::flight::session_dir(&trace_db(&app)?, &id)
        .map_err(read_error)?
        .filter(|dir| dir.is_dir())
        .ok_or_else(|| AppError::Invalid("This run has no session folder on disk.".into()))?;
    crate::flight::reveal(&dir)
        .map_err(|err| AppError::Invalid(format!("Could not open the folder: {err}")))
}
