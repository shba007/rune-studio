use crate::focus_time::MINUTE;

pub const DEFAULT_ESTIMATE_MINUTES: u64 = 30;
const MIN_LEFT_MINUTES: u64 = 5;
const MIN_UNTIL_DUE_MINUTES: f64 = 1.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub due_ms: u64,
    pub estimate_minutes: Option<u32>,
    pub credited_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub id: String,
    pub title: String,
    pub due_ms: u64,
    pub minutes_left: u64,
    pub urgency: f64,
    pub overdue: bool,
}

pub fn minutes_left(candidate: &Candidate) -> u64 {
    let estimate = candidate
        .estimate_minutes
        .map_or(DEFAULT_ESTIMATE_MINUTES, u64::from);
    estimate
        .saturating_sub(candidate.credited_ms / MINUTE)
        .max(MIN_LEFT_MINUTES)
}

pub fn rank(candidates: &[Candidate], now: u64, limit: usize) -> Vec<Ranked> {
    let mut ranked: Vec<Ranked> = candidates
        .iter()
        .map(|c| {
            let left = minutes_left(c);
            let until_due = (c.due_ms as f64 - now as f64) / MINUTE as f64;
            Ranked {
                id: c.id.clone(),
                title: c.title.clone(),
                due_ms: c.due_ms,
                minutes_left: left,
                urgency: left as f64 / until_due.max(MIN_UNTIL_DUE_MINUTES),
                overdue: c.due_ms <= now,
            }
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.urgency
            .total_cmp(&a.urgency)
            .then(a.due_ms.cmp(&b.due_ms))
            .then(a.id.cmp(&b.id))
    });
    ranked.truncate(limit);
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, due_min: u64, estimate: Option<u32>, credited_min: u64) -> Candidate {
        Candidate {
            id: id.into(),
            title: id.to_uppercase(),
            due_ms: due_min * MINUTE,
            estimate_minutes: estimate,
            credited_ms: credited_min * MINUTE,
        }
    }
    fn order(ranked: &[Ranked]) -> Vec<&str> {
        ranked.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn effort_left_is_the_estimate_minus_time_already_credited() {
        assert_eq!(minutes_left(&task("a", 100, Some(60), 25)), 35);
        assert_eq!(
            minutes_left(&task("a", 100, None, 0)),
            30,
            "default estimate"
        );
        assert_eq!(
            minutes_left(&task("a", 100, Some(30), 30)),
            5,
            "used up, still listed"
        );
        assert_eq!(minutes_left(&task("a", 100, Some(30), 90)), 5);
    }

    #[test]
    fn a_big_task_due_soon_outranks_a_small_task_due_later() {
        let list = [
            task("small", 24 * 60, Some(30), 0),
            task("big", 180, Some(120), 0),
        ];
        assert_eq!(order(&rank(&list, 0, 3)), ["big", "small"]);
    }

    #[test]
    fn time_already_spent_lowers_urgency() {
        let list = [task("a", 120, Some(60), 0), task("b", 120, Some(60), 50)];
        let ranked = rank(&list, 0, 3);
        assert_eq!(order(&ranked), ["a", "b"]);
        assert_eq!(ranked[1].minutes_left, 10);
    }

    #[test]
    fn an_overdue_task_comes_first_and_is_marked() {
        let list = [task("soon", 30, Some(30), 0), task("late", 10, Some(30), 0)];
        let ranked = rank(&list, 20 * MINUTE, 3);
        assert_eq!(order(&ranked), ["late", "soon"]);
        assert!(ranked[0].overdue && !ranked[1].overdue);
        assert!(ranked[0].urgency.is_finite());
    }

    #[test]
    fn equal_urgency_goes_to_the_task_due_sooner() {
        let list = [
            task("later", 120, Some(60), 0),
            task("sooner", 60, Some(30), 0),
        ];
        assert_eq!(order(&rank(&list, 0, 3)), ["sooner", "later"]);
    }

    #[test]
    fn only_the_top_few_are_returned() {
        let list: Vec<_> = (1..=5)
            .map(|n| task(&format!("t{n}"), n * 100, Some(60), 0))
            .collect();
        let ranked = rank(&list, 0, 3);
        assert_eq!(order(&ranked), ["t1", "t2", "t3"]);
        assert!(rank(&[], 0, 3).is_empty());
    }
}
