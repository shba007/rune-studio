use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

use crate::events::{AgentState, agent_state};

const PREFIX: &str = "overlay-";
const FADE_OUT: Duration = Duration::from_millis(500);
const LINGER_AFTER_END: Duration = Duration::from_millis(6_500);
const CAPTURE_WAVE: Duration = Duration::from_millis(1_800);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFeedback {
    serial: u64,
    monitor: crate::model::coords::Monitor,
    remaining_ms: u64,
}

struct CapturePreview {
    serial: u64,
    monitor: crate::model::coords::Monitor,
    started: Instant,
}

impl CapturePreview {
    fn snapshot(&self, now: Instant) -> Option<CaptureFeedback> {
        let remaining = CAPTURE_WAVE.checked_sub(now.duration_since(self.started))?;
        (!remaining.is_zero()).then_some(CaptureFeedback {
            serial: self.serial,
            monitor: self.monitor,
            remaining_ms: remaining.as_millis() as u64,
        })
    }
}

static CAPTURE_PREVIEW: Mutex<Option<CapturePreview>> = Mutex::new(None);

fn capture_feedback() -> Option<CaptureFeedback> {
    CAPTURE_PREVIEW
        .lock()
        .ok()?
        .as_ref()?
        .snapshot(Instant::now())
}

pub fn show_capture(app: &AppHandle, monitor: crate::model::coords::Monitor) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let Ok(mut preview) = CAPTURE_PREVIEW.lock() else {
        return;
    };
    *preview = Some(CapturePreview {
        serial,
        monitor,
        started: Instant::now(),
    });
    drop(preview);
    show(app);
    if let Ok(mut preview) = CAPTURE_PREVIEW.lock()
        && let Some(current) = preview.as_mut().filter(|current| current.serial == serial)
    {
        current.started = Instant::now();
    }
    if let Some(feedback) = capture_feedback() {
        let _ = app.emit("capture_feedback", feedback);
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(CAPTURE_WAVE);
        if let Ok(mut preview) = CAPTURE_PREVIEW.lock() {
            if preview
                .as_ref()
                .is_some_and(|current| current.serial == serial)
            {
                *preview = None;
            } else {
                return;
            }
        }
        hide_after_fade(&app, FADE_OUT);
    });
}

pub fn has_control(state: AgentState) -> bool {
    matches!(
        state,
        AgentState::Planning
            | AgentState::Observing
            | AgentState::Acting
            | AgentState::AwaitingConfirm
    )
}

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
    Hide,
}

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayInfo {
    pub mode: Owner,
    pub primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub capture_feedback: Option<CaptureFeedback>,
    pub recording: crate::recording::Snapshot,
}

pub fn info(window: &tauri::WebviewWindow) -> OverlayInfo {
    let primary = match (window.current_monitor(), window.primary_monitor()) {
        (Ok(Some(here)), Ok(Some(main))) => {
            here.position() == main.position() && here.size() == main.size()
        }
        _ => false,
    };
    let size = window.inner_size().unwrap_or_default();
    let position = window.inner_position().unwrap_or_default();
    OverlayInfo {
        mode: owner(),
        primary,
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        scale_factor: window.scale_factor().unwrap_or(1.0),
        capture_feedback: capture_feedback(),
        recording: crate::recording::snapshot(),
    }
}

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

pub fn recording_changed(app: &AppHandle) {
    if crate::recording::capturing() {
        show(app);
    } else {
        let app = app.clone();
        std::thread::spawn(move || hide_after_fade(&app, FADE_OUT));
    }
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
    if let Some(pill) = app.get_webview_window(crate::window::MAIN) {
        let _ = pill.set_always_on_top(true);
    }
    // Overlays must stay behind visible settings and replay windows, including
    // when a new result causes the overlay to be shown again. Do not steal focus.
    for label in [crate::hub::WINDOW, crate::flight::WINDOW] {
        if let Some(window) = app.get_webview_window(label)
            && window.is_visible().unwrap_or(false)
            && !window.is_minimized().unwrap_or(false)
        {
            let _ = window.set_always_on_top(true);
        }
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
    if has_control(agent_state(app))
        || owner() != Owner::None
        || capture_feedback().is_some()
        || crate::recording::capturing()
    {
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
    fn capture_feedback_expires_and_late_overlay_startup_gets_only_the_remaining_wave() {
        let started = Instant::now();
        let preview = CapturePreview {
            serial: 7,
            monitor: crate::model::coords::Monitor {
                id: 1,
                x: -1920,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.25,
                primary: false,
            },
            started,
        };
        let fresh = preview.snapshot(started).unwrap();
        assert_eq!(fresh.serial, 7);
        assert_eq!(fresh.remaining_ms, 1800);
        assert_eq!(fresh.monitor.x, -1920);
        assert_eq!(
            preview
                .snapshot(started + Duration::from_millis(300))
                .unwrap()
                .remaining_ms,
            1500
        );
        assert!(preview.snapshot(started + CAPTURE_WAVE).is_none());
        assert!(
            preview
                .snapshot(started + CAPTURE_WAVE + Duration::from_secs(1))
                .is_none()
        );
    }

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
