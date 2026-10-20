//! Keeping the boxes honest. They were drawn for the screen as it was when it was captured; once the
//! screen has changed they are wrong. So while boxes are on the real screen the live screen is
//! compared every 1.5 s with the small grey picture taken at capture, and the foreground window with
//! the one then. If the window changed, or more than 5% of the picture moved, the boxes fade out and
//! never come back for that analysis. **This is the only way the boxes disappear** (the findings text
//! stays), because they are false after the screen moved.

use crate::{agent::stuck::changed_fraction, safety::foreground::WindowId};

/// How often the live screen is looked at.
pub const EVERY_MS: u64 = 1500;
/// More than this share of the comparison picture moved: the screen is not the one that was analysed.
pub const MOVED_SHARE: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    Fresh,
    WindowChanged,
    ScreenChanged,
}

/// Decides from the picture at capture and now, and the foreground window then and now. Rune's own
/// windows are not "a different window" (`None` means the front window is Rune's own, or unknown):
/// with no window to compare only the pixels decide.
pub fn decide(
    thumbnail: &[u8],
    live: &[u8],
    window_then: Option<WindowId>,
    window_now: Option<WindowId>,
) -> Staleness {
    if let (Some(then), Some(now)) = (window_then, window_now)
        && then != now
    {
        return Staleness::WindowChanged;
    }
    if changed_fraction(thumbnail, live) > MOVED_SHARE {
        return Staleness::ScreenChanged;
    }
    Staleness::Fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 128 * 72;

    fn screen() -> Vec<u8> {
        vec![200; N]
    }

    /// The first `pixels` pixels set to `level`.
    fn moved(pixels: usize, level: u8) -> Vec<u8> {
        let mut screen = screen();
        screen[..pixels].fill(level);
        screen
    }

    #[test]
    fn the_same_screen_and_window_is_fresh() {
        assert_eq!(
            decide(&screen(), &screen(), Some(7), Some(7)),
            Staleness::Fresh
        );
        assert_eq!(decide(&screen(), &screen(), None, None), Staleness::Fresh);
    }

    #[test]
    fn a_caret_a_clock_or_a_few_words_are_not_a_change() {
        // 4 percent of the picture moved: under the 5 percent line.
        assert_eq!(
            decide(&screen(), &moved(N * 4 / 100, 20), Some(7), Some(7)),
            Staleness::Fresh
        );
        // A tiny colour drift everywhere stays under the per-pixel delta.
        assert_eq!(
            decide(&screen(), &vec![205; N], Some(7), Some(7)),
            Staleness::Fresh
        );
    }

    #[test]
    fn more_than_five_percent_moving_is_a_changed_screen() {
        assert_eq!(
            decide(&screen(), &moved(N * 6 / 100, 20), Some(7), Some(7)),
            Staleness::ScreenChanged
        );
        assert_eq!(
            decide(&screen(), &moved(N, 20), Some(7), Some(7)),
            Staleness::ScreenChanged
        );
    }

    #[test]
    fn a_different_foreground_window_is_a_change_even_with_identical_pixels() {
        assert_eq!(
            decide(&screen(), &screen(), Some(7), Some(9)),
            Staleness::WindowChanged
        );
    }

    #[test]
    fn rune_in_front_or_an_unknown_window_does_not_count_as_a_change() {
        assert_eq!(
            decide(&screen(), &screen(), Some(7), None),
            Staleness::Fresh,
            "the pill came to the front"
        );
        assert_eq!(
            decide(&screen(), &screen(), None, Some(9)),
            Staleness::Fresh,
            "no window to compare with"
        );
    }

    #[test]
    fn pictures_that_cannot_be_compared_count_as_changed() {
        assert_eq!(
            decide(&screen(), &[1, 2, 3], None, None),
            Staleness::ScreenChanged
        );
        assert_eq!(decide(&[], &[], None, None), Staleness::ScreenChanged);
    }
}
