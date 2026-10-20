//! The screen overlay: the "agent has control" effect and the screen analysis markers.
//! The "agent has control" screen effect: one transparent, click-through window per monitor
//! that draws a dot-matrix frame along the screen edges while the agent is acting.
//!
//! The windows never take focus, ignore the mouse and are excluded from capture, so they
//! neither get in the user's way nor show up in the screenshots the model reads.

use std::{sync::Mutex, time::Duration};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

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

/// Who the overlay belongs to right now. Control (the agent driving) and analysis exclude each other,
/// so the overlay has exactly one owner at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Owner {
    None,
    Control,
    Analysis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    ControlOn,
    ControlOff,
    AnalysisOn,
    AnalysisOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Nothing,
    Show,
    /// Fade out and hide.
    Hide,
}

/// The owner state machine. Control always wins (a running task never gives up the overlay to an
/// analysis), an analysis only takes a free overlay, and only the owner can release it.
pub fn step(owner: Owner, change: Change) -> (Owner, Effect) {
    match (owner, change) {
        (_, Change::ControlOn) => (Owner::Control, Effect::Show),
        (Owner::Control, Change::ControlOff) => (Owner::None, Effect::Hide),
        (other, Change::ControlOff) => (other, Effect::Nothing),
        (Owner::None, Change::AnalysisOn) => (Owner::Analysis, Effect::Show),
        (other, Change::AnalysisOn) => (other, Effect::Nothing),
        (Owner::Analysis, Change::AnalysisOff) => (Owner::None, Effect::Hide),
        (other, Change::AnalysisOff) => (other, Effect::Nothing),
    }
}

static OWNER: Mutex<Owner> = Mutex::new(Owner::None);

pub const MODE_CHANGED: &str = "overlay_mode";

#[derive(Clone, Serialize)]
pub struct ModeChanged {
    pub mode: Owner,
}

/// The current owner (what the overlay pages draw).
pub fn owner() -> Owner {
    OWNER.lock().map(|owner| *owner).unwrap_or(Owner::None)
}

fn change(app: &AppHandle, change: Change, linger: Duration) {
    let (next, effect) = {
        let Ok(mut owner) = OWNER.lock() else { return };
        let (next, effect) = step(*owner, change);
        let changed = next != *owner;
        *owner = next;
        if changed {
            let _ = app.emit(MODE_CHANGED, ModeChanged { mode: next });
        }
        (next, effect)
    };
    match effect {
        Effect::Show => show(app),
        Effect::Hide => hide_after_fade(app, linger),
        Effect::Nothing => {}
    }
    let _ = next;
}

/// What an overlay page needs to know about itself: who owns the overlay and whether this window
/// is on the primary monitor (the analysis draws there only).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayInfo {
    pub mode: Owner,
    pub primary: bool,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

pub fn info(window: &tauri::WebviewWindow) -> OverlayInfo {
    let primary = match (window.current_monitor(), window.primary_monitor()) {
        (Ok(Some(here)), Ok(Some(main))) => {
            here.position() == main.position() && here.size() == main.size()
        }
        _ => false,
    };
    let size = window.inner_size().unwrap_or_default();
    OverlayInfo {
        mode: owner(),
        primary,
        width: size.width,
        height: size.height,
        scale_factor: window.scale_factor().unwrap_or(1.0),
    }
}

/// Shows or hides the control overlay for the new state. Safe to call from any thread.
///
/// The work runs on its own thread: building and configuring windows from inside the main
/// thread's event handling blocks on that same thread and the windows never finish setting up.
pub fn sync(app: &AppHandle, state: AgentState) {
    let handle = app.clone();
    std::thread::spawn(move || {
        if has_control(state) {
            crate::cursor::apply();
            change(&handle, Change::ControlOn, FADE_OUT);
        } else {
            crate::cursor::restore();
            let linger = if matches!(state, AgentState::Done | AgentState::Failed) {
                LINGER_AFTER_END
            } else {
                FADE_OUT
            };
            change(&handle, Change::ControlOff, linger);
        }
    });
}

/// Brings the overlay up or takes it down for a screen analysis (see `analysis::state::wants_overlay`).
pub fn sync_analysis(app: &AppHandle, wanted: bool) {
    let handle = app.clone();
    std::thread::spawn(move || {
        change(
            &handle,
            if wanted {
                Change::AnalysisOn
            } else {
                Change::AnalysisOff
            },
            FADE_OUT,
        );
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
        crate::window::show_without_focus(&window);
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
    crate::window::protect(&window)?;
    Ok(window)
}

fn hide_after_fade(app: &AppHandle, wait: Duration) {
    std::thread::sleep(wait);
    // Control (or an analysis) may have come back while the page was fading out.
    if has_control(agent_state(app)) || owner() != Owner::None {
        return;
    }
    for window in overlays(app) {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Change::*;
    use Owner::*;

    #[test]
    fn control_always_takes_the_overlay_and_gives_it_back_when_it_ends() {
        assert_eq!(step(None, ControlOn), (Control, Effect::Show));
        assert_eq!(
            step(Control, ControlOn),
            (Control, Effect::Show),
            "repeated states just show again"
        );
        assert_eq!(step(Control, ControlOff), (None, Effect::Hide));
        assert_eq!(
            step(Analysis, ControlOn),
            (Control, Effect::Show),
            "control wins over an analysis"
        );
    }

    #[test]
    fn an_analysis_only_takes_a_free_overlay_and_only_its_owner_releases_it() {
        assert_eq!(step(None, AnalysisOn), (Analysis, Effect::Show));
        assert_eq!(
            step(Analysis, AnalysisOn),
            (Analysis, Effect::Nothing),
            "no second show"
        );
        assert_eq!(
            step(Control, AnalysisOn),
            (Control, Effect::Nothing),
            "a running task keeps it"
        );
        assert_eq!(step(Analysis, AnalysisOff), (None, Effect::Hide));
        assert_eq!(
            step(Control, AnalysisOff),
            (Control, Effect::Nothing),
            "an analysis cannot hide control's overlay"
        );
        assert_eq!(step(None, AnalysisOff), (None, Effect::Nothing));
    }

    #[test]
    fn control_ending_does_not_hide_an_analysis_overlay() {
        assert_eq!(step(Analysis, ControlOff), (Analysis, Effect::Nothing));
        assert_eq!(step(None, ControlOff), (None, Effect::Nothing));
    }

    #[test]
    fn the_mode_is_sent_as_a_lowercase_word() {
        assert_eq!(
            serde_json::to_string(&Owner::Analysis).unwrap(),
            "\"analysis\""
        );
        assert_eq!(serde_json::to_string(&Owner::None).unwrap(), "\"none\"");
    }
}
