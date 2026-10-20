//! Noticing that the agent is going nowhere: it acts, and the screen stays the same.
//!
//! Each look is reduced to a tiny grey picture and compared with the one before. A caret
//! blinking or a clock ticking is too small to show at that size, a page changing is not.

/// After this many unchanged looks the model is told that what it is doing is not working.
pub const WARN_AFTER: u32 = 2;
/// After this many the run ends: more of the same will not help.
pub const GIVE_UP_AFTER: u32 = 5;
/// A pixel of the small grey picture counts as changed when it differs by more than this (0 to 255).
const PIXEL_DELTA: u8 = 10;
/// Two looks are the same screen when fewer than this fraction of the pixels changed. A blinking
/// caret or a ticking clock touches a few pixels; a menu, a page or a window changing touches many.
const CHANGED_FRACTION: f64 = 0.002;
/// Width and height of the grey picture `Frame::signature` makes.
pub const SIGNATURE_SIZE: (u32, u32) = (128, 72);

/// What the agent just did, as far as the stuck check cares.
pub struct Acted {
    /// The action's one-line description: the same line twice in a row is a repeat.
    pub key: String,
    /// Typing, a key press, a scroll or a wait can change very little of the screen and still be right.
    pub quiet: bool,
}

impl Acted {
    pub fn new(tool: &str, key: String) -> Self {
        Self { key, quiet: matches!(tool, "type_text" | "press_keys" | "scroll" | "wait") }
    }
}

#[derive(Default)]
pub struct Watch {
    last: Option<Vec<u8>>,
    unchanged: u32,
    last_key: Option<String>,
    repeats: u32,
}

impl Watch {
    /// Records the screen now. `acted` is what ran since the last look; a look with no action in
    /// between (a rejected answer, a declined action) proves nothing either way. An unchanged
    /// screen after a quiet action (typing, a key, a scroll) is not counted: it may be right.
    /// Returns how many actions in a row left the screen as it was.
    pub fn look(&mut self, signature: Vec<u8>, acted: Option<&Acted>) -> u32 {
        if let Some(acted) = acted {
            self.repeats = if self.last_key.as_deref() == Some(acted.key.as_str()) { self.repeats + 1 } else { 1 };
            self.last_key = Some(acted.key.clone());
            match &self.last {
                Some(before) if same(before, &signature) => {
                    if !acted.quiet {
                        self.unchanged += 1;
                    }
                }
                _ => self.unchanged = 0,
            }
        }
        self.last = Some(signature);
        self.unchanged
    }

    /// Whether the run should end: the screen has not changed for `GIVE_UP_AFTER` actions **and**
    /// the last three were the same action. Trying different things is not being stuck.
    pub fn should_give_up(&self) -> bool {
        self.unchanged >= GIVE_UP_AFTER && self.repeats >= 3
    }
}

/// After this many looks in a row at an elevated front window the run ends: it cannot be controlled.
pub const ELEVATED_GIVE_UP_AFTER: u32 = 2;

/// Counts consecutive looks at a front window that runs as administrator.
#[derive(Default)]
pub struct ElevatedWatch {
    streak: u32,
}

impl ElevatedWatch {
    /// Records one look and returns how many in a row were elevated; a normal look resets it.
    pub fn look(&mut self, elevated: bool) -> u32 {
        self.streak = if elevated { self.streak + 1 } else { 0 };
        self.streak
    }
}

fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    let changed = a.iter().zip(b).filter(|(x, y)| x.abs_diff(**y) > PIXEL_DELTA).count();
    (changed as f64 / a.len() as f64) < CHANGED_FRACTION
}

/// What the model is told once it has been stuck for a while.
pub fn warning(unchanged: u32) -> Option<String> {
    (unchanged >= WARN_AFTER).then(|| {
        format!(
            "The screen has not changed after your last {unchanged} actions, so they are not working. Do not repeat them: try another control, a keyboard shortcut or scrolling, or ask the user."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(level: u8) -> Vec<u8> {
        vec![level; (SIGNATURE_SIZE.0 * SIGNATURE_SIZE.1) as usize]
    }

    /// A screen with `pixels` of the grey picture changed to `level`.
    fn with_patch(base: u8, pixels: usize, level: u8) -> Vec<u8> {
        let mut screen = screen(base);
        screen[..pixels].fill(level);
        screen
    }

    fn click(n: u32) -> Acted {
        Acted::new("click", format!("click {n}"))
    }

    #[test]
    fn a_screen_that_keeps_changing_is_never_stuck() {
        let mut watch = Watch::default();
        assert_eq!(watch.look(screen(10), None), 0);
        assert_eq!(watch.look(screen(60), Some(&click(1))), 0);
        assert_eq!(watch.look(screen(120), Some(&click(2))), 0);
    }

    #[test]
    fn clicks_that_change_nothing_add_up_and_a_change_resets_it() {
        let mut watch = Watch::default();
        watch.look(screen(10), None);
        assert_eq!(watch.look(screen(10), Some(&click(1))), 1);
        assert_eq!(watch.look(screen(11), Some(&click(1))), 2, "a tiny change is still the same screen");
        assert_eq!(watch.look(screen(90), Some(&click(1))), 0, "a real change starts over");
    }

    #[test]
    fn a_blinking_caret_is_not_a_change_but_a_menu_opening_is() {
        let mut watch = Watch::default();
        watch.look(screen(200), None);
        assert_eq!(watch.look(with_patch(200, 4, 20), Some(&click(1))), 1, "4 of 9216 pixels is a caret");
        // 5 percent of the picture: a menu, a dialog, a page.
        assert_eq!(watch.look(with_patch(200, 460, 20), Some(&click(2))), 0);
    }

    #[test]
    fn a_small_real_change_that_the_old_32_by_18_average_could_not_see_now_counts() {
        // 0.5 percent of the pixels, far below the mean difference the old check needed (1.5 of 255).
        let mut watch = Watch::default();
        watch.look(screen(200), None);
        assert_eq!(watch.look(with_patch(200, 46, 20), Some(&click(1))), 0);
    }

    #[test]
    fn typing_keys_and_scrolling_that_change_little_are_not_counted_as_stuck() {
        let mut watch = Watch::default();
        watch.look(screen(10), None);
        for (tool, key) in [("type_text", "type hello"), ("press_keys", "press Enter"), ("scroll", "scroll down"), ("type_text", "type rune")] {
            assert_eq!(watch.look(screen(10), Some(&Acted::new(tool, key.into()))), 0, "{tool}");
        }
        assert_eq!(watch.look(screen(10), Some(&click(1))), 1, "a click that does nothing still counts");
    }

    #[test]
    fn the_run_ends_only_when_the_same_action_is_repeated_without_effect() {
        let mut watch = Watch::default();
        watch.look(screen(10), None);
        // Five different clicks that did nothing: warned, but not ended.
        for n in 1..=5 {
            watch.look(screen(10), Some(&click(n)));
        }
        assert!(watch.unchanged >= GIVE_UP_AFTER);
        assert!(!watch.should_give_up(), "different actions are not a loop");
        // The same click three more times: now it is a loop.
        for _ in 0..3 {
            watch.look(screen(10), Some(&click(9)));
        }
        assert!(watch.should_give_up());
        // A change clears it.
        watch.look(screen(100), Some(&click(9)));
        assert!(!watch.should_give_up());
    }

    #[test]
    fn a_look_with_no_action_in_between_changes_nothing() {
        let mut watch = Watch::default();
        watch.look(screen(10), None);
        assert_eq!(watch.look(screen(10), Some(&click(1))), 1);
        assert_eq!(watch.look(screen(10), None), 1);
    }

    #[test]
    fn the_warning_starts_at_two() {
        assert!(warning(1).is_none());
        assert!(warning(WARN_AFTER).unwrap().contains("last 2 actions"));
        assert!(GIVE_UP_AFTER > WARN_AFTER);
    }

    #[test]
    fn elevated_looks_are_counted_in_a_row_and_a_normal_look_resets() {
        let mut watch = ElevatedWatch::default();
        assert_eq!(watch.look(false), 0);
        assert_eq!(watch.look(true), 1);
        assert!(watch.look(true) >= ELEVATED_GIVE_UP_AFTER);
        assert_eq!(watch.look(false), 0, "a normal look starts the count again");
        assert_eq!(watch.look(true), 1);
    }
}
