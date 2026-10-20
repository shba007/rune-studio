//! When is the user idle? One clock with the time passed in, so tests need no real waiting.
//!
//! Idle means: nothing has happened for `idle_ms` and nothing is running. Every kind of input
//! resets it; time during which a task runs, an outside agent holds the lease, the analysis window
//! is open or a prompt is up does not count at all (the clock restarts when that ends).

/// What counts as the user being there (each resets the clock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// A task started (typed, voice, hotkey or an outside agent's `rune_begin`).
    Task,
    /// An utterance was heard.
    Voice,
    /// The pill was clicked, typed in or opened.
    Pill,
    /// A screen analysis was asked for.
    Analysis,
}

#[derive(Debug, Clone, Copy)]
pub struct IdleClock {
    /// Milliseconds on whatever clock the caller uses (a monotonic one in the app).
    last_activity: u64,
    /// A prompt was already shown for this stretch of idleness.
    prompted: bool,
}

impl IdleClock {
    pub fn new(now: u64) -> Self {
        Self {
            last_activity: now,
            prompted: false,
        }
    }

    /// Something happened: the idle stretch starts again.
    pub fn touch(&mut self, now: u64, _input: Input) {
        self.last_activity = now;
        self.prompted = false;
    }

    /// Called every few seconds. `busy` means something that is not idleness is going on. Returns
    /// whether the prompt is due now; it returns true once per idle stretch.
    pub fn tick(&mut self, now: u64, idle_ms: u64, busy: bool) -> bool {
        if busy {
            self.last_activity = now;
            self.prompted = false;
            return false;
        }
        if self.prompted || now.saturating_sub(self.last_activity) < idle_ms {
            return false;
        }
        self.prompted = true;
        true
    }

    /// The user said no (or a reflection ended): the next prompt comes after another full idle period.
    pub fn restart(&mut self, now: u64) {
        self.last_activity = now;
        self.prompted = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn it_fires_once_after_the_idle_time_and_not_again_for_the_same_stretch() {
        let mut clock = IdleClock::new(0);
        assert!(!clock.tick(9 * MIN, 10 * MIN, false));
        assert!(clock.tick(10 * MIN, 10 * MIN, false));
        assert!(!clock.tick(11 * MIN, 10 * MIN, false), "already prompted");
        assert!(!clock.tick(60 * MIN, 10 * MIN, false));
    }

    #[test]
    fn every_kind_of_input_resets_it() {
        for input in [Input::Task, Input::Voice, Input::Pill, Input::Analysis] {
            let mut clock = IdleClock::new(0);
            clock.touch(9 * MIN, input);
            assert!(
                !clock.tick(10 * MIN, 10 * MIN, false),
                "{input:?} at 9 minutes"
            );
            assert!(
                clock.tick(19 * MIN, 10 * MIN, false),
                "{input:?}: ten idle minutes after it"
            );
        }
    }

    #[test]
    fn busy_time_does_not_count_and_the_clock_restarts_when_it_ends() {
        let mut clock = IdleClock::new(0);
        // A task (or a held lease, or an open analysis window) for 30 minutes.
        for minute in 0..30 {
            assert!(!clock.tick(minute * MIN, 10 * MIN, true));
        }
        assert!(
            !clock.tick(35 * MIN, 10 * MIN, false),
            "only 5 idle minutes since the task ended"
        );
        assert!(clock.tick(40 * MIN, 10 * MIN, false));
    }

    #[test]
    fn a_denied_prompt_waits_another_full_idle_period() {
        let mut clock = IdleClock::new(0);
        assert!(clock.tick(10 * MIN, 10 * MIN, false));
        clock.restart(10 * MIN);
        assert!(!clock.tick(19 * MIN, 10 * MIN, false));
        assert!(clock.tick(20 * MIN, 10 * MIN, false));
    }

    #[test]
    fn a_clock_that_goes_backwards_does_not_fire() {
        let mut clock = IdleClock::new(100);
        assert!(!clock.tick(50, 10, false));
    }
}
