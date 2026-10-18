use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::events::FOCUS_INPUT;

pub const MAIN: &str = "main";

const COLLAPSED: (f64, f64) = (680.0, 96.0);
const EXPANDED: (f64, f64) = (680.0, 480.0);
const TOP_MARGIN: f64 = 16.0;

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

pub fn set_expanded(app: &AppHandle, expanded: bool) -> tauri::Result<()> {
    let Some(window) = main_window(app) else {
        return Ok(());
    };
    let (width, height) = if expanded { EXPANDED } else { COLLAPSED };
    window.set_size(LogicalSize::new(width, height))?;
    place_top_center(&window)
}

pub fn set_click_through(app: &AppHandle, enabled: bool) {
    if let Some(window) = main_window(app)
        && let Err(err) = window.set_ignore_cursor_events(enabled)
    {
        log::warn!("could not change click-through: {err}");
    }
}

pub fn show(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit(FOCUS_INPUT, ());
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}
