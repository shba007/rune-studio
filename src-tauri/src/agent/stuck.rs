//! Noticing that the agent is going nowhere: it acts, and the screen stays the same.
//!
//! Each look is reduced to a tiny grey picture and compared with the one before. A caret
//! blinking or a clock ticking is too small to show at that size, a page changing is not.

/// After this many unchanged looks the model is told that what it is doing is not working.
pub const WARN_AFTER: u32 = 2;
/// After this many the run ends: more of the same will not help.
pub const GIVE_UP_AFTER: u32 = 5;
/// Average difference per pixel (0 to 255) below which two looks count as the same screen.
const SAME_BELOW: f64 = 1.5;

#[derive(Default)]
pub struct Watch {
    last: Option<Vec<u8>>,
    unchanged: u32,
}

impl Watch {
    /// Records the screen now. `acted` says whether an action ran since the last look; a look
    /// with no action in between (a rejected answer, a declined action) proves nothing either way.
    /// Returns how many actions in a row left the screen as it was.
    pub fn look(&mut self, signature: Vec<u8>, acted: bool) -> u32 {
        if acted {
            match &self.last {
                Some(before) if same(before, &signature) => self.unchanged += 1,
                _ => self.unchanged = 0,
            }
        }
        self.last = Some(signature);
        self.unchanged
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
    let total: u64 = a.iter().zip(b).map(|(x, y)| u64::from(x.abs_diff(*y))).sum();
    (total as f64 / a.len() as f64) < SAME_BELOW
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
        vec![level; 32 * 18]
    }

    #[test]
    fn a_screen_that_keeps_changing_is_never_stuck() {
        let mut watch = Watch::default();
        assert_eq!(watch.look(screen(10), false), 0);
        assert_eq!(watch.look(screen(60), true), 0);
        assert_eq!(watch.look(screen(120), true), 0);
    }

    #[test]
    fn actions_that_change_nothing_add_up_and_a_change_resets_it() {
        let mut watch = Watch::default();
        watch.look(screen(10), false);
        assert_eq!(watch.look(screen(10), true), 1);
        assert_eq!(watch.look(screen(11), true), 2, "a tiny change is still the same screen");
        assert_eq!(watch.look(screen(90), true), 0, "a real change starts over");
    }

    #[test]
    fn a_look_with_no_action_in_between_changes_nothing() {
        let mut watch = Watch::default();
        watch.look(screen(10), false);
        assert_eq!(watch.look(screen(10), true), 1);
        assert_eq!(watch.look(screen(10), false), 1);
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
