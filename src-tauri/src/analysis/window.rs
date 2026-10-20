//! The analysis window: a normal, resizable window (unlike the overlay it takes the keyboard and
//! the mouse), excluded from capture so it can never show up in a screenshot.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::{events::AppState, sync::LockExt};

pub const LABEL: &str = "analysis";

/// Opens the window, or brings the open one to the front. Opening it changes nothing about the
/// agent or the pill.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show()?;
        window.unminimize()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("index.html?view=analysis".into()),
    )
    .title("Rune Studio - Screen analysis")
    .inner_size(1040.0, 680.0)
    .min_inner_size(720.0, 480.0)
    .resizable(true)
    .decorations(true)
    .center()
    .visible(false)
    .build()?;
    // Excluded before it is ever shown.
    crate::window::exclude_from_capture(&window)?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        // Closing the window ends the call in flight.
        if matches!(event, WindowEvent::Destroyed) {
            handle.state::<AppState>().analysis.locked().cancel.cancel();
        }
    });
    window.show()?;
    window.set_focus()
}
