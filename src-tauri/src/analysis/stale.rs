use crate::{agent::stuck::changed_fraction, safety::foreground::WindowId};

pub const EVERY_MS: u64 = 1500;
pub const MOVED_SHARE: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    Fresh,
    WindowChanged,
    ScreenChanged,
}

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
        assert_eq!(
            decide(&screen(), &moved(N * 4 / 100, 20), Some(7), Some(7)),
            Staleness::Fresh
        );
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
