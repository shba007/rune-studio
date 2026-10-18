use serde::Deserialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::events::{OPEN_SETTINGS, SUMMON};

pub const MAIN: &str = "main";

/// Gap between the pill and the screen edges, in logical pixels.
const EDGE_MARGIN: f64 = 16.0;

/// The pill's discrete window sizes. The UI animates inside the window; the window itself
/// only ever jumps between these. Keep in sync with the layout in `AgentPill.vue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Ambient,
    Open,
    Settings,
}

impl Mode {
    fn size(self) -> (f64, f64) {
        match self {
            Mode::Ambient => (76.0, 76.0),
            Mode::Open => (572.0, 268.0),
            Mode::Settings => (572.0, 560.0),
        }
    }
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// Keeps the pill out of screenshots so the model never sees (or reads) its own UI.
pub fn exclude_from_capture(window: &WebviewWindow) -> tauri::Result<()> {
    window.set_content_protected(true)
}

/// Puts the window in the bottom-right corner of the primary monitor's work area, so it
/// sits above the taskbar.
pub fn place_bottom_right(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(());
    };
    let area = monitor.work_area();
    let size = window.outer_size()?;
    let margin = (EDGE_MARGIN * monitor.scale_factor()) as i32;
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    window.set_position(PhysicalPosition::new(x, y))
}

/// Resizes while keeping the bottom-right corner fixed, so the pill grows up and to the left
/// from wherever the user has put it. The result is kept inside the monitor's work area.
pub fn set_mode(app: &AppHandle, mode: Mode) -> tauri::Result<()> {
    let Some(window) = main_window(app) else {
        return Ok(());
    };
    let position = window.outer_position()?;
    let old = window.outer_size()?;
    let right = position.x + old.width as i32;
    let bottom = position.y + old.height as i32;

    let (width, height) = mode.size();
    window.set_size(LogicalSize::new(width, height))?;

    let new = window.outer_size()?;
    let mut x = right - new.width as i32;
    let mut y = bottom - new.height as i32;
    if let Some(monitor) = window.current_monitor()? {
        let area = monitor.work_area();
        x = x.clamp(area.position.x, (area.position.x + area.size.width as i32 - new.width as i32).max(area.position.x));
        y = y.clamp(area.position.y, (area.position.y + area.size.height as i32 - new.height as i32).max(area.position.y));
    }
    window.set_position(PhysicalPosition::new(x, y))
}

pub fn set_click_through(app: &AppHandle, enabled: bool) {
    if let Some(window) = main_window(app)
        && let Err(err) = window.set_ignore_cursor_events(enabled)
    {
        log::warn!("could not change click-through: {err}");
    }
}

/// Shows the window if needed and asks the UI to open (or, if already open, collapse) the capsule.
pub fn summon(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit(SUMMON, ());
    }
}

pub fn open_settings(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit(OPEN_SETTINGS, ());
    }
}

pub fn toggle_visibility(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
    }
}
