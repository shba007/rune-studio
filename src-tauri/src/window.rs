use serde::Deserialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::events::{OPEN_SETTINGS, SUMMON};

pub const MAIN: &str = "main";

const TOP_MARGIN: f64 = 16.0;

/// The pill's discrete window sizes. The UI animates inside the window; the window itself
/// only ever jumps between these.
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
            Mode::Open => (480.0, 76.0),
            Mode::Settings => (480.0, 420.0),
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

/// Centres the window horizontally at the top of the primary monitor.
pub fn place_top_center(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(());
    };
    let size = window.outer_size()?;
    let x = monitor.position().x + (monitor.size().width as i32 - size.width as i32) / 2;
    let y = monitor.position().y + (TOP_MARGIN * monitor.scale_factor()) as i32;
    window.set_position(PhysicalPosition::new(x, y))
}

/// Resizes around the window's current horizontal centre, so a pill the user dragged
/// elsewhere grows in place instead of snapping back to the top of the screen.
pub fn set_mode(app: &AppHandle, mode: Mode) -> tauri::Result<()> {
    let Some(window) = main_window(app) else {
        return Ok(());
    };
    let position = window.outer_position()?;
    let old_width = window.outer_size()?.width as i32;
    let center_x = position.x + old_width / 2;

    let (width, height) = mode.size();
    window.set_size(LogicalSize::new(width, height))?;

    let new_width = window.outer_size()?.width as i32;
    window.set_position(PhysicalPosition::new(center_x - new_width / 2, position.y))
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
