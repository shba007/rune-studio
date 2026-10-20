//! What the analysis window shows: the latest screenshot, the question, where the call is and the
//! parsed result. The rules (when an analysis may start, which status may follow which) are pure
//! functions; nothing here touches Tauri.

use std::sync::Arc;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Serialize;

use super::report::{Finding, Status};
use crate::{exec::CancelToken, model::coords::Monitor};

/// The screenshot an analysis is about, kept so a follow-up question can use it again.
#[derive(Debug, Clone)]
pub struct StoredFrame {
    pub jpeg: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub monitor: Monitor,
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
    /// Cancels the call in flight (Stop, Cancel, closing the window).
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
            cancel: CancelToken::default(),
        }
    }
}

/// What the UI gets: everything it needs to draw, with the screenshot as a data URI.
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
    pub image: Option<String>,
    pub width: u32,
    pub height: u32,
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
            image: self.frame.as_ref().map(|frame| {
                format!(
                    "data:image/jpeg;base64,{}",
                    BASE64.encode(frame.jpeg.as_slice())
                )
            }),
            width: self.frame.as_ref().map_or(0, |frame| frame.width),
            height: self.frame.as_ref().map_or(0, |frame| frame.height),
        }
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
        self.cancel = CancelToken::default();
    }
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
    fn the_snapshot_is_camel_case_and_carries_the_screenshot_as_a_data_uri() {
        let mut state = AnalysisState::default();
        let json = serde_json::to_value(state.snapshot()).unwrap();
        assert_eq!(json["status"], "idle");
        assert!(json["image"].is_null());
        assert!(json.get("latencyMs").is_some() && json.get("createdUtc").is_some());
        state.frame = Some(StoredFrame {
            jpeg: Arc::new(vec![0xFF, 0xD8]),
            width: 640,
            height: 360,
            monitor: Monitor {
                id: 0,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
                primary: true,
            },
        });
        let snapshot = state.snapshot();
        assert_eq!(
            snapshot.image.as_deref(),
            Some("data:image/jpeg;base64,/9g=")
        );
        assert_eq!((snapshot.width, snapshot.height), (640, 360));
    }
}
