use std::{str::FromStr, time::Duration};

use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use super::{CancelToken, ExecError};

/// Pause after moving the cursor, so the target app sees the pointer before the button goes down.
const SETTLE_AFTER_MOVE: Duration = Duration::from_millis(30);
/// Gap between the two clicks of a double click (well inside Windows' default double-click time).
const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(60);
/// Pause between typed characters. 4 ms made Windows 11 Notepad drop letters and repeat others (about
/// half of the lines in a live test were garbled); 8 ms and slower typed 25 of 25 lines cleanly. 15 ms
/// leaves margin and still types about 70 characters a second.
const TYPE_DELAY: Duration = Duration::from_millis(15);

/// Experiment knob: RUNE_TYPE_DELAY_MS overrides the pause between typed characters.
fn type_delay() -> Duration {
    static DELAY: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();
    *DELAY.get_or_init(|| std::env::var("RUNE_TYPE_DELAY_MS").ok().and_then(|v| v.parse().ok()).map_or(TYPE_DELAY, Duration::from_millis))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl FromStr for MouseButton {
    type Err = ExecError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.trim().to_ascii_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "middle" => Ok(Self::Middle),
            other => Err(ExecError::UnknownKey(other.to_string())),
        }
    }
}

impl From<MouseButton> for Button {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }
}

fn input_error(err: impl std::fmt::Display) -> ExecError {
    ExecError::Input(err.to_string())
}

/// Real mouse and keyboard input on the desktop. All coordinates are physical desktop pixels;
/// the model-space conversion happens in `model::coords`, never here.
pub struct Input {
    enigo: Enigo,
}

impl Input {
    pub fn new() -> Result<Self, ExecError> {
        Ok(Self { enigo: Enigo::new(&Settings::default()).map_err(input_error)? })
    }

#[cfg_attr(not(debug_assertions), allow(dead_code))]
    /// Where the cursor is now, in physical desktop pixels.
    pub fn cursor(&self) -> Result<(i32, i32), ExecError> {
        self.enigo.location().map_err(input_error)
    }

    pub fn move_to(&mut self, px: i32, py: i32, cancel: &CancelToken) -> Result<(), ExecError> {
        cancel.check()?;
        self.enigo.move_mouse(px, py, Coordinate::Abs).map_err(input_error)?;
        cancel.sleep(SETTLE_AFTER_MOVE)
    }

    pub fn click(
        &mut self,
        px: i32,
        py: i32,
        button: MouseButton,
        double: bool,
        cancel: &CancelToken,
    ) -> Result<(), ExecError> {
        self.move_to(px, py, cancel)?;
        self.enigo.button(button.into(), Direction::Click).map_err(input_error)?;
        if double {
            cancel.sleep(DOUBLE_CLICK_GAP)?;
            self.enigo.button(button.into(), Direction::Click).map_err(input_error)?;
        }
        Ok(())
    }

    /// Types text one character at a time, checking for Stop between characters.
    pub fn type_text(&mut self, text: &str, cancel: &CancelToken) -> Result<(), ExecError> {
        let mut buffer = [0_u8; 4];
        for character in text.chars() {
            cancel.check()?;
            self.enigo.text(character.encode_utf8(&mut buffer)).map_err(input_error)?;
            cancel.sleep(type_delay())?;
        }
        Ok(())
    }

    /// Presses a key combination: every key but the last is held, the last is clicked, and then
    /// the held keys are released in reverse order. The release runs even if Stop or an error
    /// interrupts the combination, so a modifier can never stay stuck down.
    pub fn press_keys(&mut self, names: &[String], cancel: &CancelToken) -> Result<(), ExecError> {
        let keys = names.iter().map(|name| parse_key(name)).collect::<Result<Vec<_>, _>>()?;
        let Some((last, held)) = keys.split_last() else {
            return Ok(());
        };

        let mut pressed: Vec<Key> = Vec::new();
        let outcome = (|| {
            for key in held {
                cancel.check()?;
                self.enigo.key(*key, Direction::Press).map_err(input_error)?;
                pressed.push(*key);
            }
            cancel.check()?;
            self.enigo.key(*last, Direction::Click).map_err(input_error)?;
            Ok(())
        })();

        for key in pressed.into_iter().rev() {
            let _ = self.enigo.key(key, Direction::Release);
        }
        outcome
    }
}

/// Key names the model may use. Case-insensitive; a single character is typed as itself.
pub fn parse_key(name: &str) -> Result<Key, ExecError> {
    let lowered = name.trim().to_ascii_lowercase();
    let key = match lowered.as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" | "option" => Key::Alt,
        "win" | "windows" | "meta" | "super" | "cmd" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "space" => Key::Space,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        single if single.chars().count() == 1 => Key::Unicode(single.chars().next().unwrap_or_default()),
        _ => return Err(ExecError::UnknownKey(name.to_string())),
    };
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys_and_aliases_parse() {
        assert!(matches!(parse_key("Ctrl"), Ok(Key::Control)));
        assert!(matches!(parse_key("CONTROL"), Ok(Key::Control)));
        assert!(matches!(parse_key("win"), Ok(Key::Meta)));
        assert!(matches!(parse_key("Escape"), Ok(Key::Escape)));
        assert!(matches!(parse_key("pagedown"), Ok(Key::PageDown)));
        assert!(matches!(parse_key("F12"), Ok(Key::F12)));
        assert!(matches!(parse_key(" enter "), Ok(Key::Return)));
    }

    #[test]
    fn single_characters_are_typed_as_themselves() {
        assert!(matches!(parse_key("s"), Ok(Key::Unicode('s'))));
        assert!(matches!(parse_key("7"), Ok(Key::Unicode('7'))));
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert!(matches!(parse_key("hyper"), Err(ExecError::UnknownKey(_))));
        assert!(matches!(parse_key(""), Err(ExecError::UnknownKey(_))));
    }

    #[test]
    fn typing_is_not_faster_than_notepad_can_take() {
        // A live test dropped letters at 4 ms; see TYPE_DELAY. Do not speed this up without re-running it.
        assert!(TYPE_DELAY >= Duration::from_millis(10));
    }

    #[test]
    fn mouse_buttons_parse() {
        assert_eq!("LEFT".parse::<MouseButton>().ok(), Some(MouseButton::Left));
        assert_eq!("right".parse::<MouseButton>().ok(), Some(MouseButton::Right));
        assert!("wheel".parse::<MouseButton>().is_err());
    }

    #[test]
    fn stop_interrupts_a_sleep_immediately() {
        let cancel = CancelToken::default();
        cancel.cancel();
        assert!(matches!(cancel.sleep(Duration::from_secs(5)), Err(ExecError::Cancelled)));
        cancel.reset();
        assert!(cancel.sleep(Duration::from_millis(5)).is_ok());
    }
}
