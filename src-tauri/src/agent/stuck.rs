pub const WARN_AFTER: u32 = 2;
pub const GIVE_UP_AFTER: u32 = 5;
const PIXEL_DELTA: u8 = 10;
const CHANGED_FRACTION: f64 = 0.002;
pub const SIGNATURE_SIZE: (u32, u32) = (128, 72);

#[derive(Debug, PartialEq, Eq)]
struct ControlKey {
    window: isize,
    role: &'static str,
    name: String,
}

impl From<&crate::exec::uia::BoundTarget> for ControlKey {
    fn from(target: &crate::exec::uia::BoundTarget) -> Self {
        Self {
            window: target.window,
            role: target.element.role,
            name: target.element.name.clone(),
        }
    }
}

#[derive(Default)]
pub struct ControlWatch {
    last: Option<ControlKey>,
    consecutive: u32,
}

impl ControlWatch {
    pub fn observe_window(&mut self, window: Option<isize>) {
        if window.is_some()
            && self
                .last
                .as_ref()
                .is_some_and(|last| Some(last.window) != window)
        {
            self.record(None);
        }
    }

    pub fn record(&mut self, target: Option<&crate::exec::uia::BoundTarget>) {
        let next = target.map(ControlKey::from);
        self.consecutive = if next.is_some() && next == self.last {
            self.consecutive + 1
        } else if next.is_some() {
            1
        } else {
            0
        };
        self.last = next;
    }

    pub fn warning(&self) -> Option<&'static str> {
        (self.consecutive >= 2).then_some("You already clicked the same control twice. Review the action history and current screen before deciding what to do next; do not click it again without user clarification.")
    }

    pub fn should_stop(&self, target: &crate::exec::uia::BoundTarget) -> bool {
        self.consecutive >= 2 && self.last.as_ref() == Some(&ControlKey::from(target))
    }

    pub fn confirmation_question(target: &crate::exec::uia::BoundTarget) -> String {
        let name = target.element.name.trim();
        if name.is_empty() {
            "I've clicked the same control twice. Would you like me to click it again?".into()
        } else {
            let name: String = name.chars().take(80).collect();
            format!("I've clicked “{name}” twice. Would you like me to click it again?")
        }
    }
}

const WAIT_WARN_AFTER: u32 = 2;
const WAIT_STOP_AFTER: u32 = 3;

#[derive(Default)]
pub struct WaitWatch {
    consecutive: u32,
}

impl WaitWatch {
    pub fn record(&mut self, is_wait: bool) {
        self.consecutive = if is_wait { self.consecutive + 1 } else { 0 };
    }

    pub fn warning(&self) -> Option<&'static str> {
        (self.consecutive >= WAIT_WARN_AFTER).then_some(
            "You have waited repeatedly. Recheck the current window and controls; Rune's own status messages are not evidence that an application is loading. Choose a useful next action, or stop if you cannot proceed.",
        )
    }

    pub fn should_stop_wait(&self) -> bool {
        self.consecutive >= WAIT_STOP_AFTER
    }
}

pub struct Acted {
    pub key: String,
    pub quiet: bool,
}

impl Acted {
    pub fn new(tool: &str, key: String) -> Self {
        Self {
            key,
            quiet: matches!(tool, "type_text" | "press_keys" | "scroll" | "wait"),
        }
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
    pub fn look(&mut self, signature: Vec<u8>, acted: Option<&Acted>) -> u32 {
        if let Some(acted) = acted {
            self.repeats = if self.last_key.as_deref() == Some(acted.key.as_str()) {
                self.repeats + 1
            } else {
                1
            };
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

    pub fn should_give_up(&self) -> bool {
        self.unchanged >= GIVE_UP_AFTER && self.repeats >= 3
    }
}

pub const ELEVATED_GIVE_UP_AFTER: u32 = 2;

#[derive(Default)]
pub struct ElevatedWatch {
    streak: u32,
}

impl ElevatedWatch {
    pub fn look(&mut self, elevated: bool) -> u32 {
        self.streak = if elevated { self.streak + 1 } else { 0 };
        self.streak
    }
}

pub fn changed_fraction(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let changed = a
        .iter()
        .zip(b)
        .filter(|(x, y)| x.abs_diff(**y) > PIXEL_DELTA)
        .count();
    changed as f64 / a.len() as f64
}

fn same(a: &[u8], b: &[u8]) -> bool {
    changed_fraction(a, b) < CHANGED_FRACTION
}

pub fn warning(unchanged: u32) -> Option<String> {
    (unchanged >= WARN_AFTER).then(|| crate::prompts::agent_stuck(unchanged))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(level: u8) -> Vec<u8> {
        vec![level; (SIGNATURE_SIZE.0 * SIGNATURE_SIZE.1) as usize]
    }

    fn with_patch(base: u8, pixels: usize, level: u8) -> Vec<u8> {
        let mut screen = screen(base);
        screen[..pixels].fill(level);
        screen
    }

    fn click(n: u32) -> Acted {
        Acted::new("click", format!("click {n}"))
    }

    fn control(window: isize, id: &str, name: &str) -> crate::exec::uia::BoundTarget {
        crate::exec::uia::BoundTarget {
            window,
            element: crate::exec::uia::Element {
                id: id.into(),
                role: "button",
                name: name.into(),
                value: None,
                rect: (0, 0, 20, 20),
            },
        }
    }

    #[test]
    fn repeated_control_is_bound_to_window_role_and_name_not_snapshot_id() {
        let mut watch = ControlWatch::default();
        let first = control(42, "e634", "Add New Tab");
        let refreshed = control(42, "e661", "Add New Tab");
        watch.record(Some(&first));
        assert!(watch.warning().is_none());
        assert!(!watch.should_stop(&refreshed));
        watch.record(Some(&refreshed));
        assert!(watch.warning().is_some());
        assert!(watch.should_stop(&control(42, "e689", "Add New Tab")));
        assert!(!watch.should_stop(&control(43, "e689", "Add New Tab")));
        assert!(!watch.should_stop(&control(42, "e689", "Save")));
        let mut different_role = refreshed.clone();
        different_role.element.role = "menu item";
        assert!(!watch.should_stop(&different_role));
    }

    #[test]
    fn different_action_or_changed_window_clears_repeated_control_streak() {
        let mut watch = ControlWatch::default();
        let first = control(42, "e1", "Add New Tab");
        watch.record(Some(&first));
        watch.record(Some(&first));
        watch.record(None);
        assert!(!watch.should_stop(&first));
        watch.record(Some(&first));
        watch.record(Some(&first));
        watch.observe_window(Some(43));
        assert!(!watch.should_stop(&first));
        watch.record(Some(&first));
        watch.record(Some(&control(42, "e2", "Save")));
        assert!(!watch.should_stop(&first));
    }

    #[test]
    fn confirmation_names_the_control_and_reset_requires_a_new_streak() {
        let mut watch = ControlWatch::default();
        let target = control(42, "e1", "Add New Tab");
        watch.record(Some(&target));
        watch.record(Some(&target));
        assert_eq!(
            ControlWatch::confirmation_question(&target),
            "I've clicked “Add New Tab” twice. Would you like me to click it again?"
        );
        assert!(watch.should_stop(&target));
        watch.record(None);
        watch.record(Some(&target));
        assert!(!watch.should_stop(&target));
        watch.record(Some(&target));
        assert!(watch.should_stop(&target));
        assert!(
            ControlWatch::confirmation_question(&control(42, "e2", "")).contains("same control")
        );
    }

    #[test]
    fn consecutive_waits_warn_then_stop_and_other_actions_reset_the_count() {
        let mut watch = WaitWatch::default();
        watch.record(true);
        assert!(watch.warning().is_none());
        watch.record(true);
        assert!(watch.warning().is_some());
        assert!(!watch.should_stop_wait());
        watch.record(true);
        assert!(watch.should_stop_wait());
        watch.record(false);
        assert!(watch.warning().is_none());
        assert!(!watch.should_stop_wait());
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
        assert_eq!(
            watch.look(screen(11), Some(&click(1))),
            2,
            "a tiny change is still the same screen"
        );
        assert_eq!(
            watch.look(screen(90), Some(&click(1))),
            0,
            "a real change starts over"
        );
    }

    #[test]
    fn a_blinking_caret_is_not_a_change_but_a_menu_opening_is() {
        let mut watch = Watch::default();
        watch.look(screen(200), None);
        assert_eq!(
            watch.look(with_patch(200, 4, 20), Some(&click(1))),
            1,
            "4 of 9216 pixels is a caret"
        );
        assert_eq!(watch.look(with_patch(200, 460, 20), Some(&click(2))), 0);
    }

    #[test]
    fn a_small_real_change_that_the_old_32_by_18_average_could_not_see_now_counts() {
        let mut watch = Watch::default();
        watch.look(screen(200), None);
        assert_eq!(watch.look(with_patch(200, 46, 20), Some(&click(1))), 0);
    }

    #[test]
    fn typing_keys_and_scrolling_that_change_little_are_not_counted_as_stuck() {
        let mut watch = Watch::default();
        watch.look(screen(10), None);
        for (tool, key) in [
            ("type_text", "type hello"),
            ("press_keys", "press Enter"),
            ("scroll", "scroll down"),
            ("type_text", "type rune"),
        ] {
            assert_eq!(
                watch.look(screen(10), Some(&Acted::new(tool, key.into()))),
                0,
                "{tool}"
            );
        }
        assert_eq!(
            watch.look(screen(10), Some(&click(1))),
            1,
            "a click that does nothing still counts"
        );
    }

    #[test]
    fn the_run_ends_only_when_the_same_action_is_repeated_without_effect() {
        let mut watch = Watch::default();
        watch.look(screen(10), None);
        for n in 1..=5 {
            watch.look(screen(10), Some(&click(n)));
        }
        assert!(watch.unchanged >= GIVE_UP_AFTER);
        assert!(!watch.should_give_up(), "different actions are not a loop");
        for _ in 0..3 {
            watch.look(screen(10), Some(&click(9)));
        }
        assert!(watch.should_give_up());
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

    const _: () = assert!(GIVE_UP_AFTER > WARN_AFTER);

    #[test]
    fn the_warning_starts_at_two() {
        assert!(warning(1).is_none());
        assert!(warning(WARN_AFTER).unwrap().contains("last 2 actions"));
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
