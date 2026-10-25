use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{
    error::{AppError, AppResult},
    events::{self, AgentState, AppState},
    exec::{
        CancelToken, ExecError,
        input::{Input, MouseButton},
    },
    model::coords::{Monitor, model_to_pixels},
    observe::screenshot::{self, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
    sync::LockExt,
    window,
};

const THUMBNAIL_EDGE: u32 = 320;

fn invalid(err: impl std::fmt::Display) -> AppError {
    AppError::Invalid(err.to_string())
}

fn permission(enabled: bool, busy: bool) -> AppResult<()> {
    if !enabled {
        return Err(invalid("Enable Developer mode in Settings first"));
    }
    if busy {
        return Err(invalid(
            "Finish or stop the session before using developer controls",
        ));
    }
    Ok(())
}

pub fn require_idle(app: &AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let analysis = state.analysis.locked();
    let enabled = state.settings.locked().developer_mode;
    permission(
        enabled,
        events::agent_state(app).is_running()
            || state.lease.locked().is_held()
            || crate::analysis::state::in_flight(analysis.status),
    )
}

async fn run_input<T, F>(app: AppHandle, delay_ms: Option<u64>, job: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&mut Input, &CancelToken) -> Result<T, ExecError> + Send + 'static,
{
    let scope = {
        let state = app.state::<AppState>();
        let analysis = state.analysis.locked();
        permission(
            state.settings.locked().developer_mode,
            events::agent_state(&app).is_running()
                || state.lease.locked().is_held()
                || crate::analysis::state::in_flight(analysis.status),
        )?;
        let scope = state.begin_run();
        events::set_agent_state(&app, AgentState::Acting);
        scope
    };
    let cancel = scope.token.clone();
    let handle = app.clone();

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        cancel.sleep(Duration::from_millis(delay_ms.unwrap_or(0)))?;
        window::set_click_through(&app, true);
        Input::new().and_then(|mut input| job(&mut input, &cancel))
    })
    .await
    .map_err(invalid);
    if !scope.token.is_cancelled() {
        window::set_click_through(&handle, false);
        events::set_agent_state(&handle, AgentState::Idle);
    }
    outcome?.map_err(invalid)
}

#[tauri::command]
pub fn debug_monitors(app: AppHandle) -> AppResult<Vec<Monitor>> {
    require_idle(&app)?;
    screenshot::monitors().map_err(invalid)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReport {
    monitor: Monitor,
    width: u32,
    height: u32,
    bytes: usize,
    millis: u64,
    thumbnail: String,
}

#[tauri::command]
pub async fn debug_capture(app: AppHandle, monitor: Option<u32>) -> AppResult<CaptureReport> {
    require_idle(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let frame =
            screenshot::capture(monitor, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY).map_err(invalid)?;
        crate::overlay::show_capture(&app, frame.monitor);
        log::info!(
            "debug_capture: {}x{} from a {}x{} monitor, {} KB, {} ms",
            frame.width,
            frame.height,
            frame.monitor.width,
            frame.monitor.height,
            frame.jpeg.len() / 1024,
            frame.elapsed_ms
        );
        Ok(CaptureReport {
            monitor: frame.monitor,
            width: frame.width,
            height: frame.height,
            bytes: frame.jpeg.len(),
            millis: frame.elapsed_ms,
            thumbnail: frame.thumbnail_data_uri(THUMBNAIL_EDGE).map_err(invalid)?,
        })
    })
    .await
    .map_err(invalid)?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickReport {
    monitor: Monitor,
    pixel_x: i32,
    pixel_y: i32,
    cursor_x: i32,
    cursor_y: i32,
}

#[tauri::command]
pub async fn debug_click(
    app: AppHandle,
    x: f64,
    y: f64,
    monitor: Option<u32>,
    button: Option<String>,
    double: Option<bool>,
    delay_ms: Option<u64>,
) -> AppResult<ClickReport> {
    let monitor = screenshot::monitor(monitor).map_err(invalid)?;
    let button = button
        .as_deref()
        .map_or(Ok(MouseButton::Left), str::parse)
        .map_err(invalid)?;
    let (pixel_x, pixel_y) = model_to_pixels(&monitor, x, y);

    run_input(app, delay_ms, move |input, cancel| {
        input.click(pixel_x, pixel_y, button, double.unwrap_or(false), cancel)?;
        let (cursor_x, cursor_y) = input.cursor()?;
        log::info!("debug_click: model ({x}, {y}) -> pixel ({pixel_x}, {pixel_y}), cursor at ({cursor_x}, {cursor_y})");
        Ok(ClickReport { monitor, pixel_x, pixel_y, cursor_x, cursor_y })
    })
    .await
}

#[tauri::command]
pub async fn debug_type(app: AppHandle, text: String, delay_ms: Option<u64>) -> AppResult<()> {
    run_input(app, delay_ms, move |input, cancel| {
        input.type_text(&text, cancel)
    })
    .await
}

#[tauri::command]
pub async fn debug_keys(app: AppHandle, keys: Vec<String>, delay_ms: Option<u64>) -> AppResult<()> {
    run_input(app, delay_ms, move |input, cancel| {
        input.press_keys(&keys, cancel)
    })
    .await
}

#[tauri::command]
pub async fn debug_move(app: AppHandle, x: f64, y: f64, delay_ms: Option<u64>) -> AppResult<()> {
    let monitor = screenshot::monitor(None).map_err(invalid)?;
    let (x, y) = model_to_pixels(&monitor, x, y);
    run_input(app, delay_ms, move |input, cancel| {
        input.move_to(x, y, cancel)
    })
    .await
}

#[cfg(test)]
mod tests {
    #[test]
    fn developer_controls_require_opt_in_and_cannot_interrupt_either_flow() {
        assert!(super::permission(false, false).is_err());
        assert!(super::permission(false, true).is_err());
        assert!(super::permission(true, true).is_err());
        assert!(super::permission(true, false).is_ok());
    }
}
