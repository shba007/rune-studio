use crate::sync::LockExt;
use tauri::{AppHandle, Emitter, Manager, State};

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
pub async fn installed_versions() -> serde_json::Value {
    let cli = tauri::async_runtime::spawn_blocking(|| -> Option<String> {
        use std::{
            io::Read,
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let path = std::env::current_exe()
            .ok()?
            .with_file_name(if cfg!(windows) {
                "rune-studio-cli.exe"
            } else {
                "rune-studio-cli"
            });
        if !path.is_file() {
            return None;
        }
        let mut command = Command::new(path);
        command
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().ok()?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().ok()? {
                if !status.success() {
                    return None;
                }
                let mut text = String::new();
                child
                    .stdout
                    .take()?
                    .take(256)
                    .read_to_string(&mut text)
                    .ok()?;
                return text
                    .trim()
                    .strip_prefix("rune-studio-cli ")
                    .map(str::to_owned);
            }
            if started.elapsed() >= Duration::from_secs(2) {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })
    .await
    .ok()
    .flatten();
    serde_json::json!({"studio":crate::VERSION,"cli":cli,"platform":std::env::consts::OS,"arch":std::env::consts::ARCH})
}

fn project_link(target: &str) -> Result<&'static str, String> {
    match target {
        "docs" => Ok("https://github.com/shba007/rune-studio#readme"),
        "releases" => Ok("https://github.com/shba007/rune-studio/releases"),
        "feedback" => Ok("https://github.com/shba007/rune-studio/issues/new/choose"),
        "license" => Ok("https://github.com/shba007/rune-studio/blob/main/LICENSE"),
        _ => Err("Choose a supported Rune Studio help link".into()),
    }
}
#[tauri::command]
pub fn open_project_link(target: String) -> Result<(), String> {
    let url = project_link(&target)?;
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("rundll32.exe");
        c.arg("url.dll,FileProtocolHandler")
            .arg(url)
            .creation_flags(0x08000000);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    command.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod project_link_tests {
    #[test]
    fn help_links_are_fixed_project_destinations_only() {
        for target in ["docs", "releases", "feedback", "license"] {
            assert!(
                super::project_link(target)
                    .unwrap()
                    .starts_with("https://github.com/shba007/rune-studio")
            );
        }
        for target in ["https://example.com", "file:///secret", "../other", ""] {
            assert!(super::project_link(target).is_err());
        }
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Settings {
    let on_disk = settings::load(&app).external_agent;
    let analysis = state.analysis.locked();
    let busy = events::agent_state(&app).is_running()
        || state.lease.locked().is_held()
        || state.mic.locked().is_some()
        || crate::analysis::state::in_flight(analysis.status);
    let mut settings = state.settings.locked();
    let changed = !busy && settings.external_agent != on_disk;
    if !busy {
        settings.external_agent = on_disk;
    }
    let result = settings.clone();
    drop(settings);
    if changed {
        state.screen_requests.locked().cancel();
        crate::bridge::requests::publish(&app);
    }
    result
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> AppResult<()> {
    Hotkeys::validate(&settings).map_err(AppError::Invalid)?;
    let analysis = state.analysis.locked();
    let mut current = state.settings.locked();
    settings::ensure_current(&app, &current)?;
    if settings.connections_revision != current.connections_revision
        || settings.active_connection_id != current.active_connection_id
        || settings.connections != current.connections
    {
        return Err(AppError::Invalid(
            "Connections changed in another window. Reload settings before saving.".into(),
        ));
    }
    if let Some(selected) = current
        .active_connection()
        .and_then(|c| c.selected_models.as_ref())
    {
        let before = [
            &current.models.agent,
            &current.models.verifier,
            &current.models.analysis,
            &current.models.voice,
            &current.models.decision,
            &current.models.advisor,
        ];
        let after = [
            &settings.models.agent,
            &settings.models.verifier,
            &settings.models.analysis,
            &settings.models.voice,
            &settings.models.decision,
            &settings.models.advisor,
        ];
        if before
            .iter()
            .zip(after)
            .any(|(old, new)| *old != new && !new.trim().is_empty() && !selected.contains(new))
        {
            return Err(AppError::Invalid(
                "Choose a model selected for the active connection.".into(),
            ));
        }
    }
    let was_on = current.external_agent;
    validate_mode_change(
        was_on,
        settings.external_agent,
        events::agent_state(&app).is_running()
            || state.lease.locked().is_held()
            || state.mic.locked().is_some()
            || crate::analysis::state::in_flight(analysis.status),
    )?;
    let endpoint_changed = settings.base_url != current.base_url;
    if endpoint_changed {
        crate::connections::ensure_idle(&app, &state, analysis.status)?;
        let active = settings
            .connections
            .iter_mut()
            .find(|c| c.id == settings.active_connection_id)
            .ok_or_else(|| AppError::Invalid("Active connection is unavailable".into()))?;
        settings::validate_connection(&active.name, &settings.base_url)?;
        settings.base_url = settings.base_url.trim().into();
        active.base_url = settings.base_url.clone();
        settings.connections_revision = settings
            .connections_revision
            .checked_add(1)
            .ok_or_else(|| AppError::Invalid("Connection revision limit reached".into()))?;
    }
    settings::save(&app, &settings)?;

    let next = Hotkeys::from_settings(&settings);
    if settings.external_agent && !was_on {
        state.lease.locked().rearm();
    }
    let demo_changed = current.demo_mode != settings.demo_mode;
    let hide_previews = current.developer_mode && !settings.developer_mode;
    let mode_changed = was_on != settings.external_agent;
    let visual_changed = current.visual_analysis != settings.visual_analysis;
    *current = settings;
    if endpoint_changed {
        state.capabilities.invalidate();
    }
    drop(current);
    crate::hub::changed(&app);
    if mode_changed {
        state.screen_requests.locked().cancel();
    }
    drop(analysis);
    if visual_changed {
        crate::analysis::run::publish(&app);
    }
    if hide_previews {
        events::preview_agent_state(&app, events::agent_state(&app));
    }
    if mode_changed {
        crate::bridge::requests::publish(&app);
    }
    if demo_changed {
        crate::window::apply_demo_mode(&app);
    }
    let previous = std::mem::replace(&mut *state.hotkeys.locked(), next.clone());
    if previous != next {
        hotkeys::apply(&app, Some(&previous), &next).map_err(AppError::Invalid)?;
    }
    Ok(())
}

fn validate_mode_change(current: bool, next: bool, busy: bool) -> AppResult<()> {
    if current != next && busy {
        return Err(AppError::Invalid(
            "Finish or stop the current task before changing execution mode".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod execution_tests {
    #[test]
    fn either_mode_is_fixed_until_the_session_ends() {
        for external in [false, true] {
            assert!(super::validate_mode_change(external, !external, true).is_err());
            assert!(super::validate_mode_change(external, external, true).is_ok());
            assert!(super::validate_mode_change(external, !external, false).is_ok());
        }
    }
}

#[tauri::command]
pub fn has_api_key(state: State<'_, AppState>) -> bool {
    settings::api_key_for(&state.settings.locked()).is_some()
}

#[tauri::command]
pub fn set_api_key(app: AppHandle, state: State<'_, AppState>, key: String) -> AppResult<()> {
    let snapshot = state.settings.locked().clone();
    let active = snapshot
        .connections
        .iter()
        .find(|c| c.id == snapshot.active_connection_id)
        .ok_or_else(|| AppError::Invalid("Active connection is unavailable".into()))?;
    let clear_key = key.trim().is_empty();
    crate::connections::connections_save(
        app,
        state,
        snapshot.connections_revision,
        Some(active.id.clone()),
        active.name.clone(),
        active.base_url.clone(),
        if clear_key { None } else { Some(key) },
        clear_key,
    )?;
    Ok(())
}

#[tauri::command]
pub async fn open_workspace(app: AppHandle, section: Option<String>) -> AppResult<()> {
    crate::workspace::open(&app, section.as_deref())
        .map_err(|err| AppError::Invalid(format!("Could not open Knowledge: {err}")))
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle, section: Option<String>) -> AppResult<()> {
    crate::hub::open(&app, section.as_deref())
        .map_err(|err| AppError::Invalid(format!("Could not open Settings: {err}")))
}

#[derive(Clone, serde::Serialize)]
pub struct BackendProbe {
    revision: u64,
    #[serde(rename = "baseUrl")]
    base_url: String,
    #[serde(rename = "connectionId")]
    connection_id: String,
    #[serde(rename = "connectionRevision")]
    connection_revision: u64,
    status: BackendStatus,
}

#[tauri::command]
pub async fn check_backend(app: AppHandle, state: State<'_, AppState>) -> AppResult<BackendProbe> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let revision = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (settings, api_key) = settings::snapshot_with_api_key(&state.settings);
    let probe = BackendProbe {
        revision,
        base_url: settings.base_url.clone(),
        connection_id: settings.active_connection_id.clone(),
        connection_revision: settings.connections_revision,
        status: backend::check(&state.http, &settings, api_key.as_deref()).await,
    };
    let _ = app.emit("backend_status", &probe);
    Ok(probe)
}

#[tauri::command]
pub fn fit_height(app: AppHandle, height: f64) -> AppResult<()> {
    Ok(window::fit_height(&app, height)?)
}

#[tauri::command]
pub fn set_mode(app: AppHandle, mode: window::Mode) -> AppResult<()> {
    Ok(window::set_mode(&app, mode)?)
}

#[derive(Clone, serde::Serialize)]
pub struct PillRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(rename = "scaleFactor")]
    pub scale_factor: f64,
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
        scale_factor: window.scale_factor().unwrap_or(1.0),
    })
}

#[tauri::command]
pub fn stop(app: AppHandle) {
    crate::intake::stop();
    app.state::<AppState>().stop_current();
    let _domain = crate::domain::MUTATION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    crate::tasks::cancel_all(&app);
    stop_desktop(app);
}

pub(crate) fn stop_desktop(app: AppHandle) {
    crate::recording::stop(false);
    app.state::<AppState>().stop_current();
    let dismiss_findings = {
        let state = app.state::<AppState>();
        let analysis = state.analysis.locked();
        analysis.id.starts_with("insight-")
            && analysis.status == crate::analysis::report::Status::Done
    };
    if dismiss_findings {
        crate::analysis::run::close(&app);
    }
    crate::bridge::requests::publish(&app);
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
    crate::intake::submit(&app, &text, false)
        .map(|_| ())
        .map_err(AppError::Invalid)
}

#[cfg(feature = "eval")]
pub fn begin_task(app: &AppHandle, text: &str, keep_mic: bool) -> AppResult<()> {
    begin_task_owned(app, text, keep_mic, None, None, None, Some(text))
}

pub fn begin_task_planned(
    app: &AppHandle,
    text: &str,
    keep_mic: bool,
    _plan: Option<Vec<String>>,
) -> AppResult<()> {
    crate::intake::submit(app, text, keep_mic)
        .map(|_| ())
        .map_err(AppError::Invalid)
}

pub(crate) fn begin_board_task(
    app: &AppHandle,
    text: &str,
    id: u64,
    steps: Vec<String>,
    keep_mic: bool,
    request_id: u64,
    original: &str,
) -> AppResult<()> {
    begin_task_owned(
        app,
        text,
        keep_mic,
        Some(steps),
        Some(id),
        Some(crate::recording::Owner::Request(request_id)),
        Some(original),
    )
}

fn begin_task_owned(
    app: &AppHandle,
    text: &str,
    keep_mic: bool,
    plan: Option<Vec<String>>,
    board_id: Option<u64>,
    recording_owner: Option<crate::recording::Owner>,
    original: Option<&str>,
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let task = text.trim();
    let analysis = state.analysis.locked();
    if state.settings.locked().external_agent {
        return Err(AppError::Invalid("External agent mode is selected. Start tasks in your agent host, or select API mode before a new task.".into()));
    }
    crate::reflect::touch(app, crate::reflect::idle::Input::Task);
    state.steer.locked().clear();
    if task.is_empty() {
        return Err(AppError::Invalid("Type or say what you want done".into()));
    }
    if crate::recording::busy() && !recording_owner.is_some_and(crate::recording::permits) {
        return Err(AppError::Invalid(
            "Finish or cancel the recording before starting a desktop task".into(),
        ));
    }
    if events::agent_state(app).is_running() {
        return Err(AppError::Invalid("A task is already running".into()));
    }
    if state.lease.locked().is_held() {
        return Err(AppError::Invalid("An outside agent is in control".into()));
    }
    if crate::analysis::state::in_flight(analysis.status) {
        return Err(AppError::Invalid(
            "A visual analysis is still running".into(),
        ));
    }
    if !keep_mic {
        drop(state.mic.locked().take());
    }
    agent::start_planned(app, task.to_string(), plan, board_id, original);
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

    if state.settings.locked().external_agent {
        return Err(AppError::Invalid(
            "Use your external agent host for voice input in external agent mode".into(),
        ));
    }

    if events::agent_state(&app).is_running() {
        return Err(AppError::Invalid(
            "Stop the current task before listening".into(),
        ));
    }

    let (settings, api_key) = settings::snapshot_with_api_key(&state.settings);
    let http = state.http.clone();
    let voice_model = settings.model_for_role(crate::model::role::ModelRole::Voice);
    let profile = crate::model::profile::resolve_profile(voice_model, None, None);
    if let Some(reason) = crate::voice::seam::voice_disabled_reason(voice_model, &profile) {
        return Err(AppError::Invalid(reason));
    }
    tauri::async_runtime::spawn(async move {
        let status = backend::check(&http, &settings, api_key.as_deref()).await;
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

#[tauri::command]
pub fn debug_set_state(app: AppHandle, state: AgentState) -> AppResult<()> {
    crate::debug::require_idle(&app)?;
    events::preview_agent_state(&app, state);
    Ok(())
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
    {
        let state = app.state::<AppState>();
        let _analysis = state.analysis.locked();
        if state.settings.locked().external_agent {
            if !cfg!(feature = "agent-bridge") {
                return Err(AppError::Invalid(
                    "This build does not include the external agent bridge".into(),
                ));
            }
            state.screen_requests.locked().enqueue(
                crate::analysis::report::question_or_default(question.as_deref().unwrap_or("")),
                std::time::Instant::now(),
            );
            drop(_analysis);
            crate::bridge::requests::publish(&app);
            return Ok(());
        }
    }
    Err(AppError::Invalid(
        "Select a screen/window in Visual Analysis before capturing".into(),
    ))
}

#[tauri::command]
pub fn screen_request_state(state: State<'_, AppState>) -> crate::bridge::requests::Snapshot {
    state
        .screen_requests
        .locked()
        .snapshot(std::time::Instant::now())
}

#[tauri::command]
pub fn cancel_screen_request(app: AppHandle) {
    app.state::<AppState>().screen_requests.locked().cancel();
    crate::bridge::requests::publish(&app);
}

#[tauri::command]
pub async fn analyze_again(app: AppHandle, question: String) -> AppResult<()> {
    crate::analysis::run::run(&app, &question, false).await
}

#[tauri::command]
pub fn analysis_state(state: State<'_, AppState>) -> crate::analysis::state::Snapshot {
    let mut snapshot = state.analysis.locked().snapshot();
    snapshot.visible_groups = state.settings.locked().visual_analysis.clone();
    snapshot
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
    let state = app.state::<AppState>();
    let mut runs = crate::flight::runs(
        &trace_db(&app)?,
        limit.unwrap_or(30).min(200),
        before.as_deref(),
    )
    .map_err(read_error)?;
    for run in &mut runs {
        let attempts = crate::domain::attempts(&trace_db(&app)?, &run.id).map_err(read_error)?;
        if let Some(last) = attempts.last().filter(|id| *id != &run.id)
            && let Some(header) =
                crate::flight::header(&trace_db(&app)?, last).map_err(read_error)?
        {
            run.outcome = header.outcome;
            run.ended_utc = header.ended_utc;
        }
        run.active = attempts.iter().any(|id| state.trace.is_active(id));
    }
    Ok(runs)
}

#[tauri::command]
pub fn task_attempts(app: AppHandle, id: String) -> AppResult<Vec<String>> {
    crate::domain::attempts(&trace_db(&app)?, &id).map_err(read_error)
}

#[tauri::command]
pub fn flight_task_history(app: AppHandle, id: String) -> AppResult<Vec<serde_json::Value>> {
    app.state::<AppState>()
        .trace
        .sync()
        .map_err(AppError::Invalid)?;
    crate::trace::store::task_history(&trace_db(&app)?, &id).map_err(read_error)
}

#[tauri::command]
pub async fn flight_original_prompt(app: AppHandle, id: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AppState>().trace.sync().map_err(AppError::Invalid)?;
        crate::flight::original_prompt(&trace_db(&app)?, &id).map_err(read_error)?
            .ok_or_else(|| AppError::Invalid("The original prompt was not retained for this task. Older records may contain only shortened or expanded execution text.".into()))
    }).await.map_err(read_error)?
}

#[tauri::command]
pub async fn flight_delete(app: AppHandle, id: String) -> AppResult<Option<String>> {
    static DELETION: std::sync::Mutex<()> = std::sync::Mutex::new(());
    tauri::async_runtime::spawn_blocking(move || {
        let _deletion = DELETION
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _domain = crate::domain::MUTATION
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let state = app.state::<AppState>();
        state.trace.sync().map_err(AppError::Invalid)?;
        let db = trace_db(&app)?;
        let members = crate::flight::session_members(&db, &id).map_err(read_error)?;
        for (index, member) in members.iter().enumerate() {
            if let Err(error) = state.trace.block_session(member) {
                for blocked in &members[..index] {
                    state.trace.unblock_session(blocked);
                }
                return Err(AppError::Invalid(error));
            }
        }
        let result = (|| {
            let mut staged = Vec::new();
            for member in &members {
                if let Some(dir) = crate::flight::session_dir(&db, member).map_err(read_error)? {
                    if crate::flight::folder_shared(&db, member, &dir).map_err(read_error)? {
                        return Err(AppError::Invalid(
                            "This folder is shared with another task; it cannot be deleted safely."
                                .into(),
                        ));
                    }
                    if let Some(folder) = crate::archive::stage_session_deletion(&dir, member)
                        .map_err(|error| AppError::Invalid(error.to_string()))?
                    {
                        staged.push(folder);
                    }
                }
            }
            crate::flight::delete_record(&db, &id).map_err(read_error)?;
            let failures = staged
                .into_iter()
                .filter_map(|folder| folder.finish().err().map(|error| error.to_string()))
                .collect::<Vec<_>>();
            let warning = (!failures.is_empty()).then(|| {
                let message = format!(
                    "Task history was deleted, but some task files could not be removed: {}",
                    failures.join("; ")
                );
                log::warn!("{message}");
                message
            });
            Ok(warning)
        })();
        if result.is_err() {
            for member in &members {
                state.trace.unblock_session(member);
            }
        }
        result
    })
    .await
    .map_err(read_error)?
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
        .ok_or_else(|| AppError::Invalid("This task has no recording folder.".into()))?;
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
    let run_id = run_id
        .filter(|id| !id.is_empty())
        .map(|id| crate::domain::task_id(&trace_db(&app)?, &id).map_err(read_error))
        .transpose()?;
    crate::flight::open(&app, run_id.as_deref())
        .map_err(|err| AppError::Invalid(format!("Could not open the Flight Player: {err}")))
}

#[tauri::command]
pub fn flight_reveal(app: AppHandle, id: String) -> AppResult<()> {
    let dir = crate::flight::session_dir(&trace_db(&app)?, &id)
        .map_err(read_error)?
        .filter(|dir| dir.is_dir())
        .ok_or_else(|| AppError::Invalid("This task has no recording folder on disk.".into()))?;
    crate::flight::reveal(&dir)
        .map_err(|err| AppError::Invalid(format!("Could not open the folder: {err}")))
}

#[tauri::command]
pub async fn run_workflow(
    app: AppHandle,
    workflow_id: String,
    input: serde_json::Value,
) -> Result<(), String> {
    crate::workflow::run(&app, &workflow_id, input).await
}
#[tauri::command]
pub async fn screen_insight_targets()
-> Result<Vec<crate::observe::screenshot::TargetOption>, String> {
    tauri::async_runtime::spawn_blocking(crate::observe::screenshot::insight_targets)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
