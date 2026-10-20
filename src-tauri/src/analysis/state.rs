//! What the analysis cards and the screen markers show: the latest screenshot (kept for follow-up
//! questions, never sent to the webview), the question, where the call is and the parsed result. The rules (when an analysis may start, which status may follow which) are pure
//! functions; nothing here touches Tauri.

use std::sync::Arc;

use serde::Serialize;

use super::report::{Finding, Status};
use crate::{exec::CancelToken, model::coords::Monitor, safety::foreground::WindowId};

/// The screenshot an analysis is about, kept so a follow-up question can use it again.
#[derive(Debug, Clone)]
pub struct StoredFrame {
    pub jpeg: Arc<Vec<u8>>,
    pub monitor: Monitor,
    /// The small grey picture taken with it, to tell later whether the screen has moved (`stale`).
    pub signature: Vec<u8>,
    /// The foreground window then (`None`: Rune's own, or unknown).
    pub front: Option<WindowId>,
}

pub struct AnalysisState {
    pub status: Status,
    /// The archive session id of the analysis in flight or finished.
    pub id: String,
    pub question: String,
    pub answer: String,
    pub findings: Vec<Finding>,
    pub model: String,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub created_utc: String,
    pub frame: Option<StoredFrame>,
    /// The screen changed after the capture: the boxes are gone and never come back for this analysis.
    pub stale: bool,
    /// How many boxes the quality guard dropped (`analysis::boxes`).
    pub boxes_rejected: u32,
    /// Cancels the call in flight (Stop, Cancel, closing the cards).
    pub cancel: CancelToken,
}

impl Default for AnalysisState {
    fn default() -> Self {
        Self {
            status: Status::Idle,
            id: String::new(),
            question: String::new(),
            answer: String::new(),
            findings: Vec::new(),
            model: String::new(),
            latency_ms: 0,
            error: None,
            created_utc: String::new(),
            frame: None,
            stale: false,
            boxes_rejected: 0,
            cancel: CancelToken::default(),
        }
    }
}

/// What the UI gets: everything it needs to draw. The screenshot itself never goes to the webview.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub status: Status,
    pub id: String,
    pub question: String,
    pub answer: String,
    pub findings: Vec<Finding>,
    pub model: String,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub created_utc: String,
    /// The monitor that was captured (its box coordinates are relative to it); `None` before a capture.
    pub monitor: Option<Monitor>,
    /// The screen changed since the capture: no boxes are drawn.
    pub stale: bool,
    pub boxes_rejected: u32,
}

impl AnalysisState {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            status: self.status,
            id: self.id.clone(),
            question: self.question.clone(),
            answer: self.answer.clone(),
            findings: self.findings.clone(),
            model: self.model.clone(),
            latency_ms: self.latency_ms,
            error: self.error.clone(),
            created_utc: self.created_utc.clone(),
            monitor: self.frame.as_ref().map(|frame| frame.monitor),
            stale: self.stale,
            boxes_rejected: self.boxes_rejected,
        }
    }

    /// Closes the cards: whatever runs is cancelled and the findings, and with them the boxes on the
    /// screen, are cleared. The captured frame stays, so a follow-up question still works.
    pub fn close(&mut self) {
        self.cancel.cancel();
        self.status = Status::Idle;
        self.answer.clear();
        self.findings.clear();
        self.error = None;
        self.stale = false;
        self.boxes_rejected = 0;
    }

    /// Starts an analysis: clears the last result and gives the new call its own cancel token.
    /// `fresh` takes a new capture first; otherwise the stored frame is asked about again.
    pub fn begin(&mut self, question: &str, fresh: bool) {
        self.status = if fresh {
            Status::Capturing
        } else {
            Status::Asking
        };
        self.question = question.to_string();
        self.answer.clear();
        self.findings.clear();
        self.model.clear();
        self.latency_ms = 0;
        self.error = None;
        self.id.clear();
        self.created_utc.clear();
        self.stale = false;
        self.boxes_rejected = 0;
        self.cancel = CancelToken::default();
    }
}

/// Whether the screen overlay should be up for an analysis: while it works, and while its boxes are on
/// the screen (a finished analysis with at least one box that has not gone stale).
pub fn wants_overlay(status: Status, boxes_on_screen: bool, stale: bool) -> bool {
    in_flight(status) || (status == Status::Done && boxes_on_screen && !stale)
}

/// Whether an analysis is under way (the screen is being captured or the model is answering).
pub fn in_flight(status: Status) -> bool {
    matches!(status, Status::Capturing | Status::Asking)
}

/// Why an analysis may not start now, if it may not. The text is shown to the user as it is.
pub fn refusal(status: Status, task_running: bool, outside_agent: bool) -> Option<&'static str> {
    if task_running {
        Some("A task is running. Wait for it to finish, or stop it, before analysing the screen.")
    } else if outside_agent {
        Some("An outside agent is in control of this PC. Analyse the screen when it is done.")
    } else if in_flight(status) {
        Some("Another analysis is still running. Cancel it or wait for it.")
    } else {
        None
    }
}

/// Whether `to` may follow `from`:
/// `idle -> capturing -> asking -> done | error | cancelled`, and a finished analysis may be asked
/// again (`asking`) or replaced by a new capture.
pub fn may_follow(from: Status, to: Status) -> bool {
    use Status::*;
    matches!(
        (from, to),
        (Idle | Done | Error | Cancelled, Capturing | Asking)
            | (Capturing, Asking | Error | Cancelled)
            | (Asking, Done | Error | Cancelled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use Status::*;

    #[test]
    fn the_status_only_moves_forward_through_the_steps() {
        assert!(may_follow(Idle, Capturing));
        assert!(may_follow(Capturing, Asking));
        for end in [Done, Error, Cancelled] {
            assert!(may_follow(Asking, end));
            assert!(may_follow(end, Capturing), "a new capture after {end:?}");
            assert!(may_follow(end, Asking), "asking again after {end:?}");
        }
        assert!(may_follow(Capturing, Error) && may_follow(Capturing, Cancelled));
        assert!(!may_follow(Idle, Done));
        assert!(
            !may_follow(Capturing, Done),
            "a result needs a model call first"
        );
        assert!(
            !may_follow(Asking, Capturing),
            "a capture cannot start in the middle of a call"
        );
        assert!(!may_follow(Done, Error));
    }

    #[test]
    fn a_running_task_an_outside_agent_or_another_analysis_refuses_with_a_message() {
        assert_eq!(refusal(Idle, false, false), None);
        assert_eq!(refusal(Done, false, false), None);
        assert_eq!(refusal(Cancelled, false, false), None);
        assert!(
            refusal(Idle, true, false)
                .unwrap()
                .starts_with("A task is running")
        );
        assert!(
            refusal(Idle, false, true)
                .unwrap()
                .starts_with("An outside agent")
        );
        assert!(
            refusal(Capturing, false, false)
                .unwrap()
                .starts_with("Another analysis")
        );
        assert!(
            refusal(Asking, false, false)
                .unwrap()
                .starts_with("Another analysis")
        );
        assert!(
            refusal(Asking, true, true)
                .unwrap()
                .starts_with("A task is running"),
            "the task is named first"
        );
    }

    #[test]
    fn beginning_clears_the_last_result_and_uses_a_fresh_token() {
        let mut state = AnalysisState {
            answer: "old".into(),
            error: Some("old".into()),
            status: Done,
            ..AnalysisState::default()
        };
        state.findings.push(Finding {
            n: 1,
            label: "x".into(),
            kind: super::super::report::Kind::Other,
            bbox: None,
            description: String::new(),
            evidence: String::new(),
        });
        let old = state.cancel.clone();
        old.cancel();
        state.begin("what?", true);
        assert_eq!(
            (state.status, state.question.as_str(), state.answer.as_str()),
            (Capturing, "what?", "")
        );
        assert!(state.findings.is_empty() && state.error.is_none());
        assert!(
            !state.cancel.is_cancelled(),
            "the earlier cancel does not reach the new call"
        );
        state.begin("again", false);
        assert_eq!(state.status, Asking);
    }

    #[test]
    fn the_snapshot_is_camel_case_and_never_carries_the_screenshot() {
        let mut state = AnalysisState::default();
        let json = serde_json::to_value(state.snapshot()).unwrap();
        assert_eq!(json["status"], "idle");
        assert!(
            json.get("image").is_none(),
            "the screenshot does not go to the webview"
        );
        assert!(json["monitor"].is_null());
        assert!(json.get("latencyMs").is_some() && json.get("createdUtc").is_some());
        assert_eq!(json["stale"], false);
        assert_eq!(json["boxesRejected"], 0);
        state.frame = Some(StoredFrame {
            jpeg: Arc::new(vec![0xFF, 0xD8]),
            monitor: Monitor {
                id: 0,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.25,
                primary: true,
            },
            signature: vec![0; 4],
            front: Some(5),
        });
        let json = serde_json::to_value(state.snapshot()).unwrap();
        assert_eq!(json["monitor"]["width"], 1920);
        assert_eq!(json["monitor"]["scaleFactor"], 1.25);
        assert_eq!(json["monitor"]["primary"], true);
    }

    #[test]
    fn closing_clears_the_findings_and_cancels_but_keeps_the_frame() {
        let mut state = AnalysisState {
            status: Done,
            answer: "a".into(),
            stale: true,
            boxes_rejected: 2,
            ..AnalysisState::default()
        };
        state.frame = Some(StoredFrame {
            jpeg: Arc::new(vec![1]),
            monitor: Monitor {
                id: 0,
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                scale_factor: 1.0,
                primary: true,
            },
            signature: vec![],
            front: None,
        });
        let token = state.cancel.clone();
        state.close();
        assert!(token.is_cancelled());
        assert_eq!(
            (
                state.status,
                state.answer.as_str(),
                state.stale,
                state.boxes_rejected
            ),
            (Idle, "", false, 0)
        );
        assert!(
            state.frame.is_some(),
            "a follow-up question can still use the frame"
        );
    }

    #[test]
    fn the_overlay_is_wanted_while_working_and_while_boxes_are_on_the_screen() {
        assert!(wants_overlay(Capturing, false, false));
        assert!(wants_overlay(Asking, false, false));
        assert!(wants_overlay(Done, true, false), "boxes stay on the screen");
        assert!(!wants_overlay(Done, true, true), "stale boxes are gone");
        assert!(!wants_overlay(Done, false, false), "nothing to draw");
        for ended in [Idle, Error, Cancelled] {
            assert!(!wants_overlay(ended, true, false), "{ended:?}");
        }
    }
}
