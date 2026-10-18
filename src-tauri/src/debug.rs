//! Development-only commands for exercising the eyes and hands (M1) without an AI in the loop.
//! Compiled only in debug builds; the Settings > Developer panel calls them.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{
    error::{AppError, AppResult},
    events::AppState,
    exec::{CancelToken, ExecError, input::{Input, MouseButton}},
    model::coords::{Monitor, model_to_pixels},
    observe::screenshot::{self, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
    window,
};

const THUMBNAIL_EDGE: u32 = 320;

fn invalid(err: impl std::fmt::Display) -> AppError {
    AppError::Invalid(err.to_string())
}

/// Runs `job` on a blocking thread after an optional delay (time to focus the target app).
/// The pill ignores the mouse while the job runs, so it can never intercept its own clicks, and
/// Stop interrupts both the delay and the job.
async fn run_input<T, F>(app: AppHandle, delay_ms: Option<u64>, job: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&mut Input, &CancelToken) -> Result<T, ExecError> + Send + 'static,
{
    let cancel = app.state::<AppState>().cancel.clone();
    cancel.reset();

    tauri::async_runtime::spawn_blocking(move || {
        cancel.sleep(Duration::from_millis(delay_ms.unwrap_or(0)))?;
        window::set_click_through(&app, true);
        let outcome = Input::new().and_then(|mut input| job(&mut input, &cancel));
        window::set_click_through(&app, false);
        outcome
    })
    .await
    .map_err(invalid)?
    .map_err(invalid)
}

#[tauri::command]
pub fn debug_monitors() -> AppResult<Vec<Monitor>> {
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

/// Captures a monitor exactly as the model would see it and returns what was captured.
#[tauri::command]
pub async fn debug_capture(monitor: Option<u32>) -> AppResult<CaptureReport> {
    tauri::async_runtime::spawn_blocking(move || {
        let frame = screenshot::capture(monitor, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY).map_err(invalid)?;
        log::info!(
            "debug_capture: {}x{} from a {}x{} monitor, {} KB, {} ms",
            frame.width, frame.height, frame.monitor.width, frame.monitor.height, frame.jpeg.len() / 1024, frame.elapsed_ms
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

/// Clicks a model-space point (0..=1000 on both axes) on a monitor, and reports where the
/// cursor actually ended up so the mapping can be checked.
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
    let button = button.as_deref().map_or(Ok(MouseButton::Left), str::parse).map_err(invalid)?;
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
    run_input(app, delay_ms, move |input, cancel| input.type_text(&text, cancel)).await
}

#[tauri::command]
pub async fn debug_keys(app: AppHandle, keys: Vec<String>, delay_ms: Option<u64>) -> AppResult<()> {
    run_input(app, delay_ms, move |input, cancel| input.press_keys(&keys, cancel)).await
}
