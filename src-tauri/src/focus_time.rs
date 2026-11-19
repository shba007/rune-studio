use serde::Serialize;

pub const MINUTE: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub default_minutes: u64,
    pub cap_minutes: u64,
    pub buffer_minutes: u64,
    pub wrap_up_minutes: u64,
    pub extend_minutes: u64,
    pub done_when_label: &'static str,
}

pub const PRESETS: [Preset; 3] = [
    Preset {
        id: "engineer",
        label: "Engineer",
        summary: "Long, deep blocks; a short buffer.",
        default_minutes: 25,
        cap_minutes: 120,
        buffer_minutes: 5,
        wrap_up_minutes: 5,
        extend_minutes: 5,
        done_when_label: "Done when",
    },
    Preset {
        id: "student",
        label: "Student",
        summary: "Shorter blocks and time to get to class.",
        default_minutes: 25,
        cap_minutes: 50,
        buffer_minutes: 10,
        wrap_up_minutes: 10,
        extend_minutes: 5,
        done_when_label: "What I will finish",
    },
    Preset {
        id: "founder",
        label: "Founder",
        summary: "Short blocks between meetings.",
        default_minutes: 15,
        cap_minutes: 45,
        buffer_minutes: 5,
        wrap_up_minutes: 5,
        extend_minutes: 5,
        done_when_label: "Outcome",
    },
];

impl Preset {
    pub fn by_id(id: &str) -> Option<Preset> {
        PRESETS.iter().copied().find(|p| p.id == id)
    }
    pub fn buffer_ms(&self) -> u64 {
        self.buffer_minutes * MINUTE
    }
    pub fn wrap_up_ms(&self) -> u64 {
        self.wrap_up_minutes * MINUTE
    }
    pub fn extend_ms(&self) -> u64 {
        self.extend_minutes * MINUTE
    }
}

impl Default for Preset {
    fn default() -> Self {
        PRESETS[0]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commitment {
    pub id: i64,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source: String,
}

pub fn next(commitments: &[Commitment], now: u64) -> Option<&Commitment> {
    commitments.iter().find(|c| c.end_ms > now)
}

pub fn focus_limit(preset: &Preset, commitments: &[Commitment], now: u64) -> Option<u64> {
    next(commitments, now).map(|c| c.start_ms.saturating_sub(preset.buffer_ms()).max(now))
}

pub fn free_ms(preset: &Preset, commitments: &[Commitment], now: u64) -> Option<u64> {
    focus_limit(preset, commitments, now).map(|limit| limit - now)
}

pub fn wrap_up<'a>(
    preset: &Preset,
    commitments: &'a [Commitment],
    now: u64,
) -> Option<&'a Commitment> {
    next(commitments, now).filter(|c| c.start_ms <= now + preset.wrap_up_ms())
}

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

    const P: Preset = PRESETS[0];

    fn at(id: i64, title: &str, start_min: u64, length_min: u64) -> Commitment {
        Commitment {
            id,
            title: title.into(),
            start_ms: start_min * MINUTE,
            end_ms: (start_min + length_min) * MINUTE,
            source: "manual".into(),
        }
    }

    #[test]
    fn with_nothing_coming_up_free_time_is_unlimited() {
        assert_eq!(free_ms(&P, &[], 0), None);
        assert_eq!(wrap_up(&P, &[], 0), None);
        assert_eq!(fit_minutes(90, None), Some(90));
    }

    #[test]
    fn free_time_is_the_gap_to_the_next_commitment_minus_the_buffer() {
        let list = [at(1, "Meeting", 40, 30)];
        assert_eq!(free_ms(&P, &list, 0), Some(35 * MINUTE));
        assert_eq!(free_ms(&P, &list, 10 * MINUTE), Some(25 * MINUTE));
    }

    #[test]
    fn inside_the_buffer_or_a_commitment_there_is_no_free_time() {
        let list = [at(1, "Meeting", 40, 30)];
        assert_eq!(free_ms(&P, &list, 37 * MINUTE), Some(0));
        assert_eq!(
            free_ms(&P, &list, 50 * MINUTE),
            Some(0),
            "meeting in progress"
        );
    }

    #[test]
    fn a_finished_commitment_is_skipped_and_the_following_one_counts() {
        let list = [at(1, "Standup", 10, 15), at(2, "Review", 100, 30)];
        assert_eq!(next(&list, 30 * MINUTE).unwrap().title, "Review");
        assert_eq!(free_ms(&P, &list, 30 * MINUTE), Some(65 * MINUTE));
        assert_eq!(free_ms(&P, &list, 200 * MINUTE), None, "all done");
    }

    #[test]
    fn the_wrap_up_warning_starts_five_minutes_before_the_commitment() {
        let list = [at(1, "Meeting", 40, 30)];
        assert_eq!(wrap_up(&P, &list, 34 * MINUTE), None);
        assert_eq!(wrap_up(&P, &list, 35 * MINUTE).unwrap().title, "Meeting");
        assert_eq!(wrap_up(&P, &list, 45 * MINUTE).unwrap().title, "Meeting");
        assert_eq!(wrap_up(&P, &list, 70 * MINUTE), None, "it is over");
    }

    #[test]
    fn a_suggested_length_never_goes_past_free_time() {
        assert_eq!(fit_minutes(45, Some(35 * MINUTE)), Some(35));
        assert_eq!(fit_minutes(20, Some(35 * MINUTE)), Some(20));
        assert_eq!(fit_minutes(25, Some(59_999)), None, "under a minute");
        assert_eq!(fit_minutes(25, Some(0)), None);
    }

    #[test]
    fn presets_have_unique_ids_and_sane_numbers() {
        for (i, p) in PRESETS.iter().enumerate() {
            assert!(PRESETS[i + 1..].iter().all(|q| q.id != p.id));
            assert!(p.default_minutes <= p.cap_minutes);
            assert!(p.cap_minutes <= 240, "the store allows at most 240");
            assert!(p.buffer_minutes >= 1 && p.wrap_up_minutes >= 1 && p.extend_minutes >= 1);
        }
        assert_eq!(Preset::by_id("student").unwrap().label, "Student");
        assert!(Preset::by_id("nope").is_none());
        assert_eq!(Preset::default().id, "engineer");
    }

    #[test]
    fn a_bigger_buffer_shrinks_free_time_and_starts_the_wrap_up_earlier() {
        let list = [at(1, "Class", 40, 30)];
        let student = Preset::by_id("student").unwrap();
        assert_eq!(free_ms(&student, &list, 0), Some(30 * MINUTE));
        assert_eq!(free_ms(&P, &list, 0), Some(35 * MINUTE));
        assert!(wrap_up(&student, &list, 30 * MINUTE).is_some());
        assert!(wrap_up(&P, &list, 30 * MINUTE).is_none());
    }
}
