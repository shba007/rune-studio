//! The agent's body: acting on the desktop and announcing what it does.
//!
//! The built-in model loop and the external-agent bridge both drive the PC through here, so an
//! action looks and behaves the same whoever decided it: the same caption and click ripple in
//! the overlay, the same settling pause, the same Stop handling.

use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::{
    events::{self, Point, StepFinished, StepStarted},
    exec::{
        CancelToken, ExecError, apps, caret,
        input::{Input, MouseButton, cursor_position, glide_duration, typing_duration},
        uia,
    },
    model::coords::{Monitor, model_to_pixels},
};

use super::protocol::Action;

/// Pause after an action so animations settle before the next screenshot.
const SETTLE: Duration = Duration::from_millis(500);
/// Launching an app takes longer to show a window than a click takes to react.
const SETTLE_AFTER_LAUNCH: Duration = Duration::from_millis(1_800);

/// How long to wait after `action` before looking at the screen again.
pub fn settle_time(action: &Action) -> Duration {
    match action {
        Action::OpenApp { .. } => SETTLE_AFTER_LAUNCH,
        Action::Wait { .. } | Action::Done { .. } => Duration::ZERO,
        _ => SETTLE,
    }
}

/// What the control overlay shows for an action: a caption detail, and for clicks where they land.
fn visual(action: &Action, monitor: &Monitor) -> (String, Option<Point>, bool, &'static str) {
    match action {
        Action::Click {
            x,
            y,
            button,
            double,
        } => {
            let (px, py) = model_to_pixels(monitor, *x, *y);
            let name = match button {
                MouseButton::Left => "left",
                MouseButton::Right => "right",
                MouseButton::Middle => "middle",
            };
            (String::new(), Some(Point { x: px, y: py }), *double, name)
        }
        Action::ClickElement { id, button, double } => {
            let point = uia::lookup(id).map(|element| {
                let (x, y) = element.centre();
                Point { x, y }
            });
            let name = match button {
                MouseButton::Left => "left",
                MouseButton::Right => "right",
                MouseButton::Middle => "middle",
            };
            (String::new(), point, *double, name)
        }
        Action::Scroll {
            x,
            y,
            direction,
            amount,
        } => {
            let (px, py) = model_to_pixels(monitor, *x, *y);
            (
                format!("{} {amount}", direction.name()),
                Some(Point { x: px, y: py }),
                false,
                "left",
            )
        }
        Action::Drag { from, .. } => {
            let (px, py) = model_to_pixels(monitor, from.0, from.1);
            (String::new(), Some(Point { x: px, y: py }), false, "left")
        }
        Action::AskUser { question } => (question.clone(), None, false, "left"),
        Action::TypeText { text } => (text.clone(), None, false, "left"),
        Action::PressKeys { keys } => (keys.join(" + "), None, false, "left"),
        Action::OpenApp { name } => (name.clone(), None, false, "left"),
        Action::Wait { ms } => (format!("{:.1} s", *ms as f64 / 1000.0), None, false, "left"),
        Action::Done { .. } => (String::new(), None, false, "left"),
    }
}

/// Tells the UI a step is starting (status line, overlay caption and ripple). Returns the one-line
/// description of the action for the history.
pub fn announce(
    app: &AppHandle,
    step: u32,
    reason: &str,
    action: &Action,
    monitor: &Monitor,
) -> String {
    let described = action.describe();
    let (detail, point, double, button) = visual(action, monitor);
    let duration_ms = match (action, &point) {
        (
            Action::Click { .. }
            | Action::ClickElement { .. }
            | Action::Scroll { .. }
            | Action::Drag { .. },
            Some(target),
        ) => {
            let (from_x, from_y) = cursor_position().unwrap_or((target.x, target.y));
            glide_duration(f64::from(target.x - from_x).hypot(f64::from(target.y - from_y)))
                .as_millis() as u64
        }
        (Action::TypeText { text }, _) => typing_duration(text).as_millis() as u64,
        _ => 0,
    };
    let _ = app.emit(
        events::STEP_STARTED,
        StepStarted {
            step,
            tool: action.tool_name(),
            reason: reason.to_string(),
            text: if reason.is_empty() {
                described.clone()
            } else {
                reason.to_string()
            },
            detail,
            point,
            double,
            button,
            duration_ms,
        },
    );
    described
}

/// Tells the UI the agent is looking at the screen (or a zoomed part of it).
pub fn announce_observe(app: &AppHandle, region: Option<[f64; 4]>) {
    let _ = app.emit(
        events::OBSERVE_STARTED,
        events::ObserveStarted {
            region,
            continuous: false,
        },
    );
}

/// Tells the UI how a step ended.
pub fn finish_step(app: &AppHandle, step: u32, result: &Result<(), ExecError>) {
    let (ok, error) = match result {
        Ok(()) => (true, None),
        Err(err) => (false, Some(err.to_string())),
    };
    let _ = app.emit(events::STEP_FINISHED, StepFinished { step, ok, error });
}

/// Performs an action and tells the UI what it did: for typed text, where it went, so the overlay
/// can mark the spot the way it marks a click.
pub fn execute_reporting(
    app: &AppHandle,
    action: &Action,
    monitor: &Monitor,
    cancel: &CancelToken,
) -> Result<(), ExecError> {
    let result = execute(action, monitor, cancel);
    if result.is_ok() && matches!(action, Action::TypeText { .. }) {
        // Apps that do not report a caret still have the mouse where the agent clicked to focus them.
        let point = caret::position()
            .or_else(cursor_position)
            .map(|(x, y)| Point { x, y });
        let _ = app.emit(events::TYPED, events::Typed { point });
    }
    result
}

/// Glides the cursor to the middle of the monitor, where the agent starts from, so a person
/// watching sees control change hands instead of the cursor staying wherever it was left.
pub fn centre_cursor(monitor: &Monitor, cancel: &CancelToken) -> Result<(), ExecError> {
    let x = monitor.x + (monitor.width / 2) as i32;
    let y = monitor.y + (monitor.height / 2) as i32;
    Input::new()?.glide_to(x, y, cancel)
}

/// Performs one action on the real desktop. `Done` does nothing here: ending a task is the
/// caller's business.
pub fn execute(action: &Action, monitor: &Monitor, cancel: &CancelToken) -> Result<(), ExecError> {
    match action {
        Action::Click {
            x,
            y,
            button,
            double,
        } => {
            let (px, py) = model_to_pixels(monitor, *x, *y);
            Input::new()?.click(px, py, *button, *double, cancel)
        }
        Action::ClickElement { id, button, double } => {
            // Where the control is now, not where it was when the list was made.
            let element = uia::resolve(id).map_err(ExecError::Input)?;
            let (px, py) = element.centre();
            Input::new()?.click(px, py, *button, *double, cancel)
        }
        Action::Scroll {
            x,
            y,
            direction,
            amount,
        } => {
            let (px, py) = model_to_pixels(monitor, *x, *y);
            Input::new()?.scroll(px, py, *direction, *amount, cancel)
        }
        Action::Drag { from, to } => {
            let start = model_to_pixels(monitor, from.0, from.1);
            let end = model_to_pixels(monitor, to.0, to.1);
            Input::new()?.drag(start, end, cancel)
        }
        // Asking is the loop's business; there is nothing to do on the desktop.
        Action::AskUser { .. } => Ok(()),
        Action::TypeText { text } => Input::new()?.type_text(text, cancel),
        Action::PressKeys { keys } => Input::new()?.press_keys(keys, cancel),
        Action::OpenApp { name } => apps::open(name),
        Action::Wait { ms } => cancel.sleep(Duration::from_millis(*ms)),
        Action::Done { .. } => Ok(()),
    }
}
