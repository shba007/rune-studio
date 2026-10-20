//! The "agent has control" screen effect: one transparent, click-through window per monitor
//! that draws a dot-matrix frame along the screen edges while the agent is acting.
//!
//! The windows never take focus, ignore the mouse and are excluded from capture, so they
//! neither get in the user's way nor show up in the screenshots the model reads.

use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::events::{AgentState, agent_state};

const PREFIX: &str = "overlay-";
/// Long enough for the page's fade-out to finish before the window is hidden.
const FADE_OUT: Duration = Duration::from_millis(500);
/// After a task ends the window stays a little longer, so the agent's last words stay in the speech
/// bubble at the orb. The frame itself fades out as usual.
const LINGER_AFTER_END: Duration = Duration::from_millis(6_500);

/// States in which the agent is driving the PC.
pub fn has_control(state: AgentState) -> bool {
    // Planning too: that is when the model thinks, and the overlay shows what it is saying.
    matches!(
        state,
        AgentState::Planning
            | AgentState::Observing
            | AgentState::Acting
            | AgentState::AwaitingConfirm
    )
}

/// Shows or hides the overlay for the new state. Safe to call from any thread.
///
/// The work runs on its own thread: building and configuring windows from inside the main
/// thread's event handling blocks on that same thread and the windows never finish setting up.
pub fn sync(app: &AppHandle, state: AgentState) {
    let handle = app.clone();
    std::thread::spawn(move || {
        if has_control(state) {
            crate::cursor::apply();
            show(&handle);
        } else {
            crate::cursor::restore();
            let linger = if matches!(state, AgentState::Done | AgentState::Failed) {
                LINGER_AFTER_END
            } else {
                FADE_OUT
            };
            hide_after_fade(&handle, linger);
        }
    });
}

fn overlays(app: &AppHandle) -> Vec<tauri::WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(PREFIX))
        .map(|(_, window)| window)
        .collect()
}

fn show(app: &AppHandle) {
    let monitors = match app.available_monitors() {
        Ok(monitors) => monitors,
        Err(err) => {
            log::warn!("could not list monitors for the overlay: {err}");
            return;
        }
    };
    // Monitors may have changed since the last time; start from a clean slate if the count differs.
    let existing = overlays(app);
    if existing.len() != monitors.len() {
        for window in existing {
            let _ = window.destroy();
        }
    }
    for (index, monitor) in monitors.iter().enumerate() {
        let label = format!("{PREFIX}{index}");
        let window = match app.get_webview_window(&label) {
            Some(window) => window,
            None => match build(app, &label) {
                Ok(window) => window,
                Err(err) => {
                    log::warn!("could not create the overlay window: {err}");
                    continue;
                }
            },
        };
        let _ = window.set_position(PhysicalPosition::new(
            monitor.position().x,
            monitor.position().y,
        ));
        let _ = window.set_size(PhysicalSize::new(
            monitor.size().width,
            monitor.size().height,
        ));
        show_without_focus(&window);
    }
    // The pill sits above the frame, never under it.
    if let Some(pill) = app.get_webview_window(crate::window::MAIN) {
        let _ = pill.set_always_on_top(true);
    }
}

fn build(app: &AppHandle, label: &str) -> tauri::Result<tauri::WebviewWindow> {
    let window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App("index.html?view=overlay".into()),
    )
    .title("Rune Studio overlay")
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .focusable(false)
    .visible(false)
    .build()?;
    window.set_ignore_cursor_events(true)?;
    crate::window::exclude_from_capture(&window)?;
    Ok(window)
}

fn hide_after_fade(app: &AppHandle, wait: Duration) {
    std::thread::sleep(wait);
    // Control may have come back while the page was fading out.
    if has_control(agent_state(app)) {
        return;
    }
    for window in overlays(app) {
        let _ = window.hide();
    }
}

/// Shows the window without activating it, so the app the user is working in keeps the keyboard.
fn show_without_focus(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
        // SAFETY: the handle belongs to a live window of this process.
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        return;
    }
    let _ = window.show();
}
