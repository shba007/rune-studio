//! Free time (cycle 3 of the Focus mode plan): how long the user can focus
//! before their next commitment. Pure functions over a sorted list, so the
//! store, the suggestion and the wrap-up warning all agree.

use serde::Serialize;

pub const MINUTE: u64 = 60_000;
/// Kept clear before a commitment so the user can arrive without rushing.
pub const BUFFER_MS: u64 = 5 * MINUTE;
/// How long before a commitment the running block is told to wrap up.
pub const WRAP_UP_MS: u64 = 5 * MINUTE;
/// What "Extend" adds.
pub const EXTEND_MS: u64 = 5 * MINUTE;

/// A fixed time the user must be somewhere. Entered by hand for now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commitment {
    pub id: i64,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// The first commitment that has not ended yet. `commitments` is ordered by
/// start time.
pub fn next(commitments: &[Commitment], now: u64) -> Option<&Commitment> {
    commitments.iter().find(|c| c.end_ms > now)
}

/// The latest moment a block may end: the next commitment's start minus the
/// buffer. `None` when nothing is coming up. Already inside a commitment, or
/// inside its buffer, it is `now`.
pub fn focus_limit(commitments: &[Commitment], now: u64) -> Option<u64> {
    next(commitments, now).map(|c| c.start_ms.saturating_sub(BUFFER_MS).max(now))
}

/// Free time before the next commitment, buffer included; `None` is unlimited.
pub fn free_ms(commitments: &[Commitment], now: u64) -> Option<u64> {
    focus_limit(commitments, now).map(|limit| limit - now)
}

/// The commitment a running block should wrap up for, once it is within
/// [`WRAP_UP_MS`] of starting (or has started).
pub fn wrap_up(commitments: &[Commitment], now: u64) -> Option<&Commitment> {
    next(commitments, now).filter(|c| c.start_ms <= now + WRAP_UP_MS)
}

/// Keeps a suggested length inside the free time. `None` means no room for
/// even one minute.
pub fn fit_minutes(minutes: u64, free: Option<u64>) -> Option<u64> {
    match free {
        None => Some(minutes),
        Some(free) => match (free / MINUTE).min(minutes) {
            0 => None,
            fitted => Some(fitted),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(id: i64, title: &str, start_min: u64, length_min: u64) -> Commitment {
        Commitment {
            id,
            title: title.into(),
            start_ms: start_min * MINUTE,
            end_ms: (start_min + length_min) * MINUTE,
        }
    }

    #[test]
    fn with_nothing_coming_up_free_time_is_unlimited() {
        assert_eq!(free_ms(&[], 0), None);
        assert_eq!(wrap_up(&[], 0), None);
        assert_eq!(fit_minutes(90, None), Some(90));
    }

    #[test]
    fn free_time_is_the_gap_to_the_next_commitment_minus_the_buffer() {
        let list = [at(1, "Meeting", 40, 30)];
        assert_eq!(free_ms(&list, 0), Some(35 * MINUTE));
        assert_eq!(free_ms(&list, 10 * MINUTE), Some(25 * MINUTE));
    }

    #[test]
    fn inside_the_buffer_or_a_commitment_there_is_no_free_time() {
        let list = [at(1, "Meeting", 40, 30)];
        assert_eq!(free_ms(&list, 37 * MINUTE), Some(0));
        assert_eq!(free_ms(&list, 50 * MINUTE), Some(0), "meeting in progress");
    }

    #[test]
    fn a_finished_commitment_is_skipped_and_the_following_one_counts() {
        let list = [at(1, "Standup", 10, 15), at(2, "Review", 100, 30)];
        assert_eq!(next(&list, 30 * MINUTE).unwrap().title, "Review");
        assert_eq!(free_ms(&list, 30 * MINUTE), Some(65 * MINUTE));
        assert_eq!(free_ms(&list, 200 * MINUTE), None, "all done");
    }

    #[test]
    fn the_wrap_up_warning_starts_five_minutes_before_the_commitment() {
        let list = [at(1, "Meeting", 40, 30)];
        assert_eq!(wrap_up(&list, 34 * MINUTE), None);
        assert_eq!(wrap_up(&list, 35 * MINUTE).unwrap().title, "Meeting");
        assert_eq!(wrap_up(&list, 45 * MINUTE).unwrap().title, "Meeting");
        assert_eq!(wrap_up(&list, 70 * MINUTE), None, "it is over");
    }

    #[test]
    fn a_suggested_length_never_goes_past_free_time() {
        assert_eq!(fit_minutes(45, Some(35 * MINUTE)), Some(35));
        assert_eq!(fit_minutes(20, Some(35 * MINUTE)), Some(20));
        assert_eq!(fit_minutes(25, Some(59_999)), None, "under a minute");
        assert_eq!(fit_minutes(25, Some(0)), None);
    }
}
