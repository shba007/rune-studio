use crate::sync::LockExt;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::events::{OPEN_ARTIFACTS, OPEN_SETTINGS, SUMMON};

pub const MAIN: &str = "main";

const EDGE_MARGIN: f64 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Ambient,
    Open,
    Task,
    Analysis,
    Settings,
    Artifacts,
}

impl Mode {
    fn size(self) -> (f64, f64) {
        match self {
            Mode::Ambient => (64.0, 64.0),
            Mode::Open => (376.0, 64.0),
            Mode::Task => (376.0, 336.0),
            Mode::Analysis => (376.0, 336.0),
            Mode::Settings => (376.0, 592.0),
            Mode::Artifacts => (376.0, 592.0),
        }
    }
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

pub fn demo_mode(app: &AppHandle) -> bool {
    app.state::<crate::events::AppState>()
        .settings
        .locked()
        .demo_mode
}

pub fn protect(window: &WebviewWindow) -> tauri::Result<()> {
    window.set_content_protected(!demo_mode(window.app_handle()))
}

pub fn apply_demo_mode(app: &AppHandle) {
    for window in app.webview_windows().values() {
        if let Err(err) = protect(window) {
            log::warn!(
                "could not change screen capture protection of {}: {err}",
                window.label()
            );
        }
    }
}

pub fn masked_rects(app: &AppHandle) -> Vec<crate::observe::mask::Rect> {
    use crate::observe::mask::{Rect, overlay_rects};

    if !demo_mode(app) {
        return Vec::new();
    }
    let mut rects = Vec::new();
    for (label, window) in app.webview_windows() {
        if !window.is_visible().unwrap_or(false) {
            continue;
        }
        let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
            continue;
        };
        let rect = Rect {
            x: position.x,
            y: position.y,
            w: size.width,
            h: size.height,
        };
        if label.starts_with("overlay-") {
            rects.extend(overlay_rects(rect, window.scale_factor().unwrap_or(1.0)));
        } else {
            rects.push(rect);
        }
    }
    rects
}

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

pub fn set_mode(app: &AppHandle, mode: Mode) -> tauri::Result<()> {
    let (width, height) = mode.size();
    resize_keeping_corner(app, Some(width), height)
}

pub fn fit_height(app: &AppHandle, height: f64) -> tauri::Result<()> {
    let ceiling = main_window(app)
        .and_then(|window| {
            let area = window.current_monitor().ok()??.work_area().size.height;
            Some(max_pill_height(
                f64::from(area),
                window.scale_factor().ok()?,
            ))
        })
        .unwrap_or(1_200.0);
    resize_keeping_corner(app, None, height.clamp(64.0, ceiling))
}

pub fn max_pill_height(work_area_physical: f64, scale: f64) -> f64 {
    (0.9 * work_area_physical / scale.max(0.1)).clamp(200.0, 1_200.0)
}

fn resize_keeping_corner(app: &AppHandle, width: Option<f64>, height: f64) -> tauri::Result<()> {
    let Some(window) = main_window(app) else {
        return Ok(());
    };
    let position = window.outer_position()?;
    let old = window.outer_size()?;
    let right = position.x + old.width as i32;
    let bottom = position.y + old.height as i32;

    let width = match width {
        Some(width) => width,
        None => f64::from(old.width) / window.scale_factor()?,
    };
    window.set_size(LogicalSize::new(width, height))?;

    let new = window.outer_size()?;
    let mut x = right - new.width as i32;
    let mut y = bottom - new.height as i32;
    if let Some(monitor) = window.current_monitor()? {
        let area = monitor.work_area();
        x = x.clamp(
            area.position.x,
            (area.position.x + area.size.width as i32 - new.width as i32).max(area.position.x),
        );
        y = y.clamp(
            area.position.y,
            (area.position.y + area.size.height as i32 - new.height as i32).max(area.position.y),
        );
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

pub fn open_artifacts(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit(OPEN_ARTIFACTS, ());
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

pub fn show_without_focus(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        return;
    }
    let _ = window.show();
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_pill_never_grows_past_ninety_percent_of_the_work_area() {
        use super::max_pill_height;
        assert_eq!(max_pill_height(1040.0, 1.0), 936.0);
        assert_eq!(max_pill_height(1040.0, 1.25), 748.8);
        assert_eq!(max_pill_height(1040.0, 1.5).round(), 624.0);
        assert_eq!(max_pill_height(5000.0, 1.0), 1200.0, "capped");
        assert_eq!(max_pill_height(100.0, 1.0), 200.0, "never useless");
    }
}
