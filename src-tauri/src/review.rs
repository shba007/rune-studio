use std::{path::Path, time::Duration};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    events::{self, AppState},
    settings::OfferReview,
    sync::LockExt,
    trace::{self, store},
};

pub const OFFER: &str = "review_offer";
const EVERY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub run_id: String,
    pub task: String,
    pub outcome: String,
    pub steps: i64,
    pub duration_secs: i64,
    pub source: String,
}

pub fn outcome_word(label: &str) -> &'static str {
    match label {
        "succeeded" => "succeeded",
        "stopped" | "lease_expired" | "replaced" => "stopped",
        _ => "failed",
    }
}

pub fn should_offer(setting: OfferReview, outcome: &str, steps: i64, busy: bool) -> bool {
    if busy || steps <= 0 {
        return false;
    }
    match setting {
        OfferReview::Always => true,
        OfferReview::Failures => outcome != "succeeded",
        OfferReview::Never => false,
    }
}

pub fn offer_for(detail: &store::RunDetail) -> Option<Offer> {
    detail.run.ended_utc.as_ref()?;
    let started = trace::parse_iso(&detail.run.info.started_utc)?;
    let ended = detail.run.ended_utc.as_deref().and_then(trace::parse_iso)?;
    Some(Offer {
        run_id: detail.run.info.id.clone(),
        task: trace::cap(detail.run.info.task.trim(), 200),
        outcome: outcome_word(detail.run.outcome.as_deref().unwrap_or("failed")).to_string(),
        steps: i64::try_from(detail.steps.len()).unwrap_or(i64::MAX),
        duration_secs: i64::try_from(ended.saturating_sub(started)).unwrap_or(0),
        source: match detail.run.info.source.as_str() {
            "" | "built-in" => "Rune".to_string(),
            other => other.to_string(),
        },
    })
}

fn latest_reviewable(rows: Vec<store::RunSummary>) -> Option<store::RunSummary> {
    rows.into_iter().find(|run| {
        !matches!(
            run.source.as_deref(),
            Some("task-request" | "background-task")
        )
    })
}

pub fn start_watcher(app: &AppHandle) {
    let app = app.clone();
    let since = trace::now_utc();
    std::thread::Builder::new()
        .name("review-offer".into())
        .spawn(move || {
            let mut announced = String::new();
            loop {
                std::thread::sleep(EVERY);
                let Some(db) = app
                    .state::<AppState>()
                    .trace
                    .db_path()
                    .map(Path::to_path_buf)
                else {
                    continue;
                };
                let Some(newest) = store::list_runs(&db, 100).ok().and_then(latest_reviewable)
                else {
                    continue;
                };
                if newest.ended_utc.is_none()
                    || newest.id == announced
                    || newest
                        .started_utc
                        .as_deref()
                        .is_none_or(|s| s < since.as_str())
                {
                    continue;
                }
                let state = app.state::<AppState>();
                let busy = events::agent_state(&app).is_running() || state.lease.locked().is_held();
                if busy {
                    continue;
                }
                announced = newest.id.clone();
                let Some(offer) = store::get_run(&db, &newest.id)
                    .ok()
                    .flatten()
                    .as_ref()
                    .and_then(offer_for)
                else {
                    continue;
                };
                let setting = state.settings.locked().offer_review;
                if should_offer(setting, &offer.outcome, offer.steps, busy) {
                    let _ = app.emit(OFFER, offer);
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{RunInfo, StepRecord, store::RunRow};

    #[test]
    fn internal_task_history_does_not_hide_the_desktop_session_review() {
        let row = |source: &str| store::RunSummary {
            id: source.into(),
            source: Some(source.into()),
            started_utc: None,
            ended_utc: None,
            outcome: None,
            task: None,
            steps: 0,
            verdict: None,
            session_dir: None,
            last_shot: None,
            shot_run: None,
            shot_dir: None,
        };
        let selected = latest_reviewable(vec![
            row("background-task"),
            row("task-request"),
            row("built-in"),
        ])
        .unwrap();
        assert_eq!(selected.id, "built-in");
        assert!(latest_reviewable(vec![row("background-task")]).is_none());
    }

    fn detail(outcome: &str, source: &str, steps: usize, ended: Option<&str>) -> store::RunDetail {
        store::RunDetail {
            run: RunRow {
                info: RunInfo {
                    id: "r".into(),
                    task: "  open notepad  ".into(),
                    source: source.into(),
                    started_utc: "2026-10-20T10:00:00Z".into(),
                    ..RunInfo::default()
                },
                ended_utc: ended.map(String::from),
                outcome: Some(outcome.into()),
                summary: None,
            },
            steps: (1..=steps as u32)
                .map(|n| StepRecord::new("r", n))
                .collect(),
            events: vec![],
            verdict: None,
        }
    }

    #[test]
    fn the_card_shows_the_task_outcome_steps_duration_and_who() {
        let offer = offer_for(&detail(
            "succeeded",
            "built-in",
            6,
            Some("2026-10-20T10:00:55Z"),
        ))
        .unwrap();
        assert_eq!(
            (
                offer.task.as_str(),
                offer.outcome.as_str(),
                offer.steps,
                offer.duration_secs,
                offer.source.as_str()
            ),
            ("open notepad", "succeeded", 6, 55, "Rune")
        );
        let outside = offer_for(&detail(
            "reported_failure",
            "Claude",
            3,
            Some("2026-10-20T10:01:00Z"),
        ))
        .unwrap();
        assert_eq!(
            (outside.outcome.as_str(), outside.source.as_str()),
            ("failed", "Claude")
        );
        assert!(
            offer_for(&detail("succeeded", "", 6, None)).is_none(),
            "a run that has not ended is not offered"
        );
    }

    #[test]
    fn endings_fold_into_succeeded_failed_and_stopped() {
        for (label, word) in [
            ("succeeded", "succeeded"),
            ("stopped", "stopped"),
            ("lease_expired", "stopped"),
            ("failed", "failed"),
            ("reported_failure", "failed"),
            ("something new", "failed"),
        ] {
            assert_eq!(outcome_word(label), word, "{label}");
        }
    }

    #[test]
    fn the_setting_decides_and_nothing_is_offered_for_an_empty_or_a_busy_moment() {
        assert!(should_offer(OfferReview::Always, "succeeded", 3, false));
        assert!(should_offer(OfferReview::Failures, "failed", 3, false));
        assert!(should_offer(OfferReview::Failures, "stopped", 3, false));
        assert!(!should_offer(OfferReview::Failures, "succeeded", 3, false));
        assert!(!should_offer(OfferReview::Never, "failed", 3, false));
        assert!(
            !should_offer(OfferReview::Always, "succeeded", 0, false),
            "zero steps"
        );
        assert!(
            !should_offer(OfferReview::Always, "succeeded", 3, true),
            "another task runs"
        );
    }

    #[test]
    fn the_offer_is_sent_in_camel_case() {
        let json = serde_json::to_value(
            offer_for(&detail("succeeded", "", 2, Some("2026-10-20T10:00:05Z"))).unwrap(),
        )
        .unwrap();
        assert!(json.get("runId").is_some() && json.get("durationSecs").is_some());
    }
}
