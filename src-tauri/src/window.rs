use crate::sync::LockExt;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::events::{OPEN_SETTINGS, SUMMON};

pub const MAIN: &str = "main";

/// Gap between the pill and the screen edges, in logical pixels.
const EDGE_MARGIN: f64 = 16.0;

/// The pill's discrete window sizes. The UI animates inside the window; the window itself
/// only ever jumps between these. Widths are ORB_SIZE + PAGE_GAP + PAGE_WIDTH (64 + 8 + 272) from
/// `src/config/layout.ts`; keep the two in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Ambient,
    Open,
    /// The composer with the task's progress panel above it.
    Task,
    Settings,
}

impl Mode {
    fn size(self) -> (f64, f64) {
        match self {
            Mode::Ambient => (64.0, 64.0),
            // Open and Task start from these and then fit their content (see `fit_height`).
            Mode::Open => (344.0, 64.0),
            Mode::Task => (344.0, 336.0),
            Mode::Settings => (344.0, 592.0),
        }
    }
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// Whether "Show Rune in screen recordings" is on.
pub fn demo_mode(app: &AppHandle) -> bool {
    app.state::<crate::events::AppState>()
        .settings
        .locked()
        .demo_mode
}

/// **The one place Rune's windows are excluded from (or opened to) screen capture.** By default every
/// window is excluded, so recorders and the model never see Rune. In demo mode the exclusion is lifted
/// so OBS, Game Bar and the Snipping Tool can record Rune; the model is kept from seeing it by
/// painting the windows' rectangles over its screenshots instead (`observe::mask`).
pub fn protect(window: &WebviewWindow) -> tauri::Result<()> {
    window.set_content_protected(!demo_mode(window.app_handle()))
}

/// Applies the setting to every window that is open (new windows call `protect` as they are made).
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

/// What the model's screenshots must not show while demo mode is on: the visible Rune windows (the
/// pill, the analysis window) and the pieces of the full-screen overlay. Empty when demo mode is off.
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
    let (width, height) = mode.size();
    resize_keeping_corner(app, Some(width), height)
}

/// Makes the window as tall as its content (`height` in logical pixels), keeping its width and the
/// bottom-right corner. The composer grows with the text; the window follows.
pub fn fit_height(app: &AppHandle, height: f64) -> tauri::Result<()> {
    resize_keeping_corner(app, None, height.clamp(64.0, 1_200.0))
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

/// Shows the window without activating it, so the app the user is working in keeps the keyboard.
pub fn show_without_focus(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
        // SAFETY: the handle belongs to a live window of this process.
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        return;
    }
    let _ = window.show();
}
