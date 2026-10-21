use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

use super::{CancelToken, ExecError};

const SETTLE_AFTER_MOVE: Duration = Duration::from_millis(30);
const DOUBLE_CLICK_GAP: Duration = Duration::from_millis(60);
const TYPE_DELAY: Duration = Duration::from_millis(15);

const GLIDE_STEP: Duration = Duration::from_millis(8);

pub fn glide_duration(distance: f64) -> Duration {
    if distance < 2.0 {
        return Duration::ZERO;
    }
    Duration::from_millis((120.0 + distance * 0.25).clamp(120.0, 450.0) as u64)
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(from: i32, to: i32, amount: f64) -> i32 {
    (f64::from(from) + f64::from(to - from) * amount).round() as i32
}

#[cfg(windows)]
pub fn cursor_position() -> Option<(i32, i32)> {
    use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some((point.x, point.y))
}

#[cfg(not(windows))]
pub fn cursor_position() -> Option<(i32, i32)> {
    None
}

pub fn typing_duration(text: &str) -> Duration {
    type_delay() * text.chars().count() as u32
}

fn type_delay() -> Duration {
    static DELAY: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();
    *DELAY.get_or_init(|| {
        std::env::var("RUNE_TYPE_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .map_or(TYPE_DELAY, Duration::from_millis)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn name(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

impl FromStr for ScrollDirection {
    type Err = ExecError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name.trim().to_ascii_lowercase().as_str() {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            other => Err(ExecError::UnknownKey(other.to_string())),
        }
    }
}

const SCROLL_GAP: Duration = Duration::from_millis(60);
const DRAG_GRAB: Duration = Duration::from_millis(120);

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

pub struct Input {
    enigo: Enigo,
}

impl Input {
    pub fn new() -> Result<Self, ExecError> {
        Ok(Self {
            enigo: Enigo::new(&Settings::default()).map_err(input_error)?,
        })
    }

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub fn cursor(&self) -> Result<(i32, i32), ExecError> {
        self.enigo.location().map_err(input_error)
    }

    pub fn move_to(&mut self, px: i32, py: i32, cancel: &CancelToken) -> Result<(), ExecError> {
        cancel.check()?;
        self.enigo
            .move_mouse(px, py, Coordinate::Abs)
            .map_err(input_error)?;
        cancel.sleep(SETTLE_AFTER_MOVE)
    }

    pub fn glide_to(&mut self, px: i32, py: i32, cancel: &CancelToken) -> Result<(), ExecError> {
        let (from_x, from_y) = self.cursor()?;
        let distance = f64::from(px - from_x).hypot(f64::from(py - from_y));
        let total = glide_duration(distance);
        let started = Instant::now();
        loop {
            cancel.check()?;
            let elapsed = started.elapsed();
            if elapsed >= total {
                break;
            }
            let amount = ease(elapsed.as_secs_f64() / total.as_secs_f64());
            self.enigo
                .move_mouse(
                    lerp(from_x, px, amount),
                    lerp(from_y, py, amount),
                    Coordinate::Abs,
                )
                .map_err(input_error)?;
            cancel.sleep(GLIDE_STEP)?;
        }
        self.move_to(px, py, cancel)
    }

    pub fn click(
        &mut self,
        px: i32,
        py: i32,
        button: MouseButton,
        double: bool,
        cancel: &CancelToken,
    ) -> Result<(), ExecError> {
        self.glide_to(px, py, cancel)?;
        self.enigo
            .button(button.into(), Direction::Click)
            .map_err(input_error)?;
        if double {
            cancel.sleep(DOUBLE_CLICK_GAP)?;
            self.enigo
                .button(button.into(), Direction::Click)
                .map_err(input_error)?;
        }
        Ok(())
    }

    pub fn scroll(
        &mut self,
        px: i32,
        py: i32,
        direction: ScrollDirection,
        amount: u32,
        cancel: &CancelToken,
    ) -> Result<(), ExecError> {
        self.glide_to(px, py, cancel)?;
        let (length, axis) = match direction {
            ScrollDirection::Up => (-1, Axis::Vertical),
            ScrollDirection::Down => (1, Axis::Vertical),
            ScrollDirection::Left => (-1, Axis::Horizontal),
            ScrollDirection::Right => (1, Axis::Horizontal),
        };
        for _ in 0..amount {
            cancel.check()?;
            self.enigo.scroll(length, axis).map_err(input_error)?;
            cancel.sleep(SCROLL_GAP)?;
        }
        Ok(())
    }

    pub fn drag(
        &mut self,
        from: (i32, i32),
        to: (i32, i32),
        cancel: &CancelToken,
    ) -> Result<(), ExecError> {
        self.glide_to(from.0, from.1, cancel)?;
        self.enigo
            .button(Button::Left, Direction::Press)
            .map_err(input_error)?;
        let outcome = (|| {
            cancel.sleep(DRAG_GRAB)?;
            self.glide_to(to.0, to.1, cancel)?;
            cancel.sleep(DRAG_GRAB)
        })();
        let _ = self.enigo.button(Button::Left, Direction::Release);
        outcome
    }

    pub fn type_text(&mut self, text: &str, cancel: &CancelToken) -> Result<(), ExecError> {
        let mut buffer = [0_u8; 4];
        for character in text.chars() {
            cancel.check()?;
            self.enigo
                .text(character.encode_utf8(&mut buffer))
                .map_err(input_error)?;
            cancel.sleep(type_delay())?;
        }
        Ok(())
    }

    pub fn press_keys(&mut self, names: &[String], cancel: &CancelToken) -> Result<(), ExecError> {
        let keys = names
            .iter()
            .map(|name| parse_key(name))
            .collect::<Result<Vec<_>, _>>()?;
        let Some((last, held)) = keys.split_last() else {
            return Ok(());
        };

        let mut pressed: Vec<Key> = Vec::new();
        let outcome = (|| {
            for key in held {
                cancel.check()?;
                self.enigo
                    .key(*key, Direction::Press)
                    .map_err(input_error)?;
                pressed.push(*key);
            }
            cancel.check()?;
            self.enigo
                .key(*last, Direction::Click)
                .map_err(input_error)?;
            Ok(())
        })();

        for key in pressed.into_iter().rev() {
            let _ = self.enigo.key(key, Direction::Release);
        }
        outcome
    }
}

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
        single if single.chars().count() == 1 => {
            Key::Unicode(single.chars().next().unwrap_or_default())
        }
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
        assert!(TYPE_DELAY >= Duration::from_millis(10));
    }

    #[test]
    fn short_hops_are_quick_and_long_ones_are_capped() {
        assert_eq!(glide_duration(0.0), Duration::ZERO);
        assert_eq!(glide_duration(1.0), Duration::ZERO);
        assert!(glide_duration(40.0) < Duration::from_millis(150));
        assert_eq!(glide_duration(5_000.0), Duration::from_millis(450));
        assert!(glide_duration(300.0) < glide_duration(900.0));
    }

    #[test]
    fn the_glide_starts_slow_ends_exactly_and_never_goes_backwards() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!(ease(0.1) < 0.1, "should start slowly");
        let mut previous = 0.0;
        for step in 0..=100 {
            let value = ease(f64::from(step) / 100.0);
            assert!(value >= previous);
            previous = value;
        }
        assert_eq!(lerp(100, 900, 0.0), 100);
        assert_eq!(lerp(100, 900, 1.0), 900);
        assert_eq!(lerp(900, 100, 0.5), 500);
    }

    #[test]
    fn mouse_buttons_parse() {
        assert_eq!("LEFT".parse::<MouseButton>().ok(), Some(MouseButton::Left));
        assert_eq!(
            "right".parse::<MouseButton>().ok(),
            Some(MouseButton::Right)
        );
        assert!("wheel".parse::<MouseButton>().is_err());
    }

    #[test]
    fn stop_interrupts_a_sleep_immediately() {
        let cancel = CancelToken::default();
        cancel.cancel();
        assert!(matches!(
            cancel.sleep(Duration::from_secs(5)),
            Err(ExecError::Cancelled)
        ));
        assert!(
            CancelToken::default()
                .sleep(Duration::from_millis(5))
                .is_ok()
        );
    }
}
