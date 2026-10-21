use std::time::Duration;

use crate::exec::CancelToken;

pub const STAGES: [&str; 3] = [
    "Reading recent tasks",
    "Looking for patterns",
    "Writing notes",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Rehearsed,
    Preempted { stages: u32 },
}

pub async fn run(
    cancel: &CancelToken,
    stage_time: Duration,
    mut report: impl FnMut(&str, u32, u32),
) -> Outcome {
    let total = STAGES.len() as u32;
    for (index, stage) in STAGES.iter().enumerate() {
        let done = index as u32;
        report(stage, done, total);
        let until = tokio::time::Instant::now() + stage_time;
        while tokio::time::Instant::now() < until {
            if cancel.is_cancelled() {
                return Outcome::Preempted { stages: done };
            }
            tokio::time::sleep(Duration::from_millis(10).min(stage_time)).await;
        }
        if cancel.is_cancelled() {
            return Outcome::Preempted { stages: done };
        }
        report(stage, done + 1, total);
    }
    Outcome::Rehearsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_goes_through_every_stage_and_reports_progress() {
        let mut seen = Vec::new();
        let outcome = run(
            &CancelToken::default(),
            Duration::from_millis(5),
            |stage, done, total| seen.push((stage.to_string(), done, total)),
        )
        .await;
        assert_eq!(outcome, Outcome::Rehearsed);
        assert_eq!(
            seen.first(),
            Some(&("Reading recent tasks".to_string(), 0, 3))
        );
        assert_eq!(seen.last(), Some(&("Writing notes".to_string(), 3, 3)));
        assert_eq!(seen.len(), 6);
    }

    #[tokio::test]
    async fn a_cancel_is_noticed_within_a_fraction_of_a_stage() {
        let cancel = CancelToken::default();
        let remote = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            remote.cancel();
        });
        let started = std::time::Instant::now();
        let outcome = run(&cancel, Duration::from_secs(10), |_, _, _| {}).await;
        assert_eq!(outcome, Outcome::Preempted { stages: 0 });
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn a_run_that_was_cancelled_before_it_began_does_nothing() {
        let cancel = CancelToken::default();
        cancel.cancel();
        let mut reports = 0;
        let outcome = run(&cancel, Duration::from_secs(10), |_, _, _| reports += 1).await;
        assert_eq!(outcome, Outcome::Preempted { stages: 0 });
        assert_eq!(reports, 1, "only the first stage's start was reported");
    }
}
