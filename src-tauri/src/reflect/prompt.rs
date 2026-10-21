pub const ANSWER_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Hidden,
    Shown { deadline: u64 },
    Reflecting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Do {
    Nothing,
    ShowPrompt { deadline: u64 },
    Denied,
    Start,
    Preempted,
    Finished,
}

impl Phase {
    pub fn show(self, now: u64) -> (Phase, Do) {
        match self {
            Phase::Hidden => {
                let deadline = now + ANSWER_SECS * 1000;
                (Phase::Shown { deadline }, Do::ShowPrompt { deadline })
            }
            other => (other, Do::Nothing),
        }
    }

    pub fn answer(self, start: bool) -> (Phase, Do) {
        match self {
            Phase::Shown { .. } if start => (Phase::Reflecting, Do::Start),
            Phase::Shown { .. } => (Phase::Hidden, Do::Denied),
            other => (other, Do::Nothing),
        }
    }

    pub fn tick(self, now: u64) -> (Phase, Do) {
        match self {
            Phase::Shown { deadline } if now >= deadline => (Phase::Reflecting, Do::Start),
            other => (other, Do::Nothing),
        }
    }

    pub fn preempt(self) -> (Phase, Do) {
        match self {
            Phase::Reflecting => (Phase::Hidden, Do::Preempted),
            Phase::Shown { .. } => (Phase::Hidden, Do::Denied),
            Phase::Hidden => (Phase::Hidden, Do::Nothing),
        }
    }

    pub fn finish(self) -> (Phase, Do) {
        match self {
            Phase::Reflecting => (Phase::Hidden, Do::Finished),
            other => (other, Do::Nothing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_is_shown_once_with_a_thirty_second_deadline() {
        let (phase, action) = Phase::Hidden.show(1_000);
        assert_eq!(
            (phase, action),
            (
                Phase::Shown { deadline: 31_000 },
                Do::ShowPrompt { deadline: 31_000 }
            )
        );
        assert_eq!(phase.show(2_000), (phase, Do::Nothing), "not shown twice");
        assert_eq!(
            Phase::Reflecting.show(2_000),
            (Phase::Reflecting, Do::Nothing)
        );
    }

    #[test]
    fn start_begins_the_job_and_deny_hides_the_prompt() {
        let shown = Phase::Shown { deadline: 31_000 };
        assert_eq!(shown.answer(true), (Phase::Reflecting, Do::Start));
        assert_eq!(shown.answer(false), (Phase::Hidden, Do::Denied));
        assert_eq!(
            Phase::Hidden.answer(true),
            (Phase::Hidden, Do::Nothing),
            "a late answer does nothing"
        );
        assert_eq!(
            Phase::Reflecting.answer(false),
            (Phase::Reflecting, Do::Nothing)
        );
    }

    #[test]
    fn the_timeout_starts_reflection_but_not_a_second_early() {
        let shown = Phase::Shown { deadline: 31_000 };
        assert_eq!(shown.tick(30_999), (shown, Do::Nothing));
        assert_eq!(shown.tick(31_000), (Phase::Reflecting, Do::Start));
        assert_eq!(Phase::Hidden.tick(99_999), (Phase::Hidden, Do::Nothing));
    }

    #[test]
    fn a_task_or_stop_ends_reflection_and_withdraws_a_prompt() {
        assert_eq!(Phase::Reflecting.preempt(), (Phase::Hidden, Do::Preempted));
        assert_eq!(
            Phase::Shown { deadline: 5 }.preempt(),
            (Phase::Hidden, Do::Denied)
        );
        assert_eq!(Phase::Hidden.preempt(), (Phase::Hidden, Do::Nothing));
    }

    #[test]
    fn a_job_that_ends_by_itself_frees_the_state_once() {
        assert_eq!(Phase::Reflecting.finish(), (Phase::Hidden, Do::Finished));
        assert_eq!(
            Phase::Hidden.finish(),
            (Phase::Hidden, Do::Nothing),
            "already pre-empted"
        );
    }
}
