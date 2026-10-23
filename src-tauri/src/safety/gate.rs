use crate::sync::LockExt;
use std::time::Duration;

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;

use super::{Assessment, Risk, audit, foreground, needs_confirmation, rules};
use crate::{
    agent::protocol::Action,
    events::{self, AgentState, AppState},
    exec::CancelToken,
};

pub const CONFIRM_REQUEST: &str = "confirm_request";
pub const CONFIRM_RESOLVED: &str = "confirm_resolved";
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Pending {
    pub id: String,
    pub kind: &'static str,
    #[cfg(feature = "eval")]
    text: String,
    sender: oneshot::Sender<bool>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRequest {
    pub id: String,
    pub source: String,
    pub text: String,
    pub reason: String,
    pub risk: Risk,
    pub reasons: Vec<String>,
    pub timeout_secs: u64,
    pub kind: &'static str,
}

#[derive(Clone, Serialize)]
pub struct ConfirmResolved {
    pub id: String,
    pub approved: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Denied(&'static str),
    Stopped,
}

#[derive(Debug, PartialEq, Eq)]
pub enum VoiceAnswer {
    Answered,
    NeedsClick,
    NothingWaiting,
}

pub fn answer_by_voice(app: &AppHandle, approved: bool) -> VoiceAnswer {
    let state = app.state::<AppState>();
    let mut slot = state.confirm.locked();
    answer_pending_by_voice(&mut slot, approved)
}

fn answer_pending_by_voice(slot: &mut Option<Pending>, approved: bool) -> VoiceAnswer {
    let Some(pending) = slot.take() else {
        return VoiceAnswer::NothingWaiting;
    };
    if approved && matches!(pending.kind, "approval" | "restart") {
        *slot = Some(pending);
        return VoiceAnswer::NeedsClick;
    }
    let _ = pending.sender.send(approved);
    VoiceAnswer::Answered
}

pub fn answer(app: &AppHandle, id: &str, approved: bool) -> bool {
    let state = app.state::<AppState>();
    let mut slot = state.confirm.locked();
    match slot.take() {
        Some(pending) if pending.id == id => {
            let _ = pending.sender.send(approved);
            true
        }
        other => {
            *slot = other;
            false
        }
    }
}

#[cfg(feature = "eval")]
pub fn pending_question(app: &AppHandle) -> Option<(String, &'static str, String)> {
    app.state::<AppState>()
        .confirm
        .locked()
        .as_ref()
        .map(|pending| (pending.id.clone(), pending.kind, pending.text.clone()))
}

fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    format!("q{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn clear_pending(slot: &mut Option<Pending>, id: &str) {
    if slot.as_ref().is_some_and(|pending| pending.id == id) {
        slot.take();
    }
}

#[derive(Debug)]
pub struct CheckResult {
    pub verdict: Verdict,
    pub risk_final: Risk,
    pub reasons: Vec<String>,
    pub decision: &'static str,
}

#[allow(dead_code)]
pub async fn check(
    app: &AppHandle,
    cancel: &CancelToken,
    source: &str,
    action: &Action,
    reason: &str,
    stated: Option<Risk>,
) -> Verdict {
    check_detailed(app, cancel, source, action, reason, stated)
        .await
        .verdict
}

pub async fn check_detailed(
    app: &AppHandle,
    cancel: &CancelToken,
    source: &str,
    action: &Action,
    reason: &str,
    stated: Option<Risk>,
) -> CheckResult {
    let (ask_every_action, assessment) = {
        let state = app.state::<AppState>();
        let ask = state.settings.locked().ask_every_action;
        (ask, rules::assess(action, reason, &foreground::current()))
    };
    let Assessment {
        risk: found,
        mut reasons,
    } = assessment;
    let mut risk = found.max(stated.unwrap_or_default());
    if stated == Some(Risk::High) && found != Risk::High {
        reasons.push("The agent marked it as high risk".into());
    }
    let text = action.describe();
    if risk != Risk::High
        && !rules::is_harmless(action)
        && !matches!(action, Action::External { .. })
    {
        let window = foreground::current();
        let window = format!("{} ({})", window.title, window.process);
        let opinion = tokio::select! {
            opinion = crate::agent::quick::risk_opinion(app, &text, reason, &window) => opinion,
            () = until_stopped(cancel) => return CheckResult { verdict: Verdict::Stopped, risk_final: risk, reasons, decision: "stopped" },
        };
        if let Some(opinion) = opinion {
            risk = risk.max(opinion);
            reasons.push("A quick safety check rated it high risk".into());
        }
    }
    let ask = needs_confirmation(risk, ask_every_action, rules::is_harmless(action));

    let verdict = if ask {
        ask_user(
            app, cancel, source, &text, reason, risk, &reasons, "approval",
        )
        .await
    } else {
        Verdict::Allow
    };

    let decision = match (&verdict, ask) {
        (Verdict::Allow, false) => "allowed",
        (Verdict::Allow, true) => "approved",
        (Verdict::Denied(why), _) => why,
        (Verdict::Stopped, _) => "stopped",
    };
    if !rules::is_harmless(action) {
        audit::record(
            app,
            json!({ "source": source, "action": text, "reason": reason, "risk": risk, "why": reasons, "decision": decision }),
        );
    }
    let decision = if rules::is_harmless(action) && decision == "allowed" {
        "none"
    } else {
        decision
    };
    CheckResult {
        verdict,
        risk_final: risk,
        reasons,
        decision,
    }
}

async fn until_stopped(cancel: &CancelToken) {
    while !cancel.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub async fn ask_question(
    app: &AppHandle,
    cancel: &CancelToken,
    source: &str,
    question: &str,
) -> Verdict {
    ask_user(
        app,
        cancel,
        source,
        question,
        "",
        Risk::Low,
        &[],
        "question",
    )
    .await
}

pub async fn ask_restart(
    app: &AppHandle,
    cancel: &CancelToken,
    source: &str,
    task: &str,
) -> Verdict {
    ask_user(
        app,
        cancel,
        source,
        "restart the stopped external-agent session",
        task,
        Risk::Medium,
        &["You pressed Stop. Allowing starts a new session; old actions are not replayed.".into()],
        "restart",
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn ask_user(
    app: &AppHandle,
    cancel: &CancelToken,
    source: &str,
    text: &str,
    reason: &str,
    risk: Risk,
    reasons: &[String],
    kind: &'static str,
) -> Verdict {
    let id = new_id();
    let target = foreground::remember();
    let (sender, receiver) = oneshot::channel();
    *app.state::<AppState>().confirm.locked() = Some(Pending {
        id: id.clone(),
        kind,
        #[cfg(feature = "eval")]
        text: text.to_string(),
        sender,
    });

    let _ = app.emit(
        CONFIRM_REQUEST,
        ConfirmRequest {
            id: id.clone(),
            source: source.to_string(),
            text: text.to_string(),
            reason: reason.to_string(),
            risk,
            reasons: reasons.to_vec(),
            timeout_secs: ANSWER_TIMEOUT.as_secs(),
            kind,
        },
    );
    if let Some(pill) = app.get_webview_window(crate::window::MAIN) {
        let _ = pill.show();
    }
    events::set_agent_state(app, AgentState::AwaitingConfirm);

    let outcome = tokio::select! {
        answer = receiver => match answer {
            Ok(true) => Verdict::Allow,
            Ok(false) | Err(_) => Verdict::Denied("declined"),
        },
        () = async { while !cancel.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await } } => Verdict::Stopped,
        () = tokio::time::sleep(ANSWER_TIMEOUT) => Verdict::Denied("no answer in time"),
    };

    {
        let state = app.state::<AppState>();
        let mut pending = state.confirm.locked();
        clear_pending(&mut pending, &id);
    }
    let outcome = match outcome {
        Verdict::Allow if matches!(kind, "approval" | "restart") => {
            match foreground::give_back(target).await {
                Ok(()) => Verdict::Allow,
                Err(why) => Verdict::Denied(why),
            }
        }
        other => {
            if !matches!(other, Verdict::Stopped) {
                let _ = foreground::give_back(target).await;
            }
            other
        }
    };
    let _ = app.emit(
        CONFIRM_RESOLVED,
        ConfirmResolved {
            id,
            approved: outcome == Verdict::Allow,
        },
    );
    outcome
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn resolving_a_cancelled_prompt_cannot_remove_a_new_restart_request() {
        let (sender, _receiver) = oneshot::channel();
        let mut pending = Some(Pending {
            id: "new-restart".into(),
            kind: "restart",
            #[cfg(feature = "eval")]
            text: "restart".into(),
            sender,
        });
        clear_pending(&mut pending, "old-action");
        assert_eq!(pending.as_ref().unwrap().id, "new-restart");
        clear_pending(&mut pending, "new-restart");
        assert!(pending.is_none());
    }

    #[test]
    fn restart_acceptance_needs_a_click_but_voice_can_keep_it_stopped() {
        let (sender, mut receiver) = oneshot::channel();
        let mut pending = Some(Pending {
            id: "restart".into(),
            kind: "restart",
            #[cfg(feature = "eval")]
            text: "restart".into(),
            sender,
        });
        assert_eq!(
            answer_pending_by_voice(&mut pending, true),
            VoiceAnswer::NeedsClick
        );
        assert!(pending.is_some());
        assert!(
            receiver.try_recv().is_err(),
            "voice yes must not approve restart"
        );
        assert_eq!(
            answer_pending_by_voice(&mut pending, false),
            VoiceAnswer::Answered
        );
        assert!(!receiver.try_recv().unwrap());
        assert!(pending.is_none());
    }

    #[test]
    fn the_confirm_request_is_sent_with_the_camel_case_names_the_card_reads() {
        let request = ConfirmRequest {
            id: "a".into(),
            source: "Rune".into(),
            text: "t".into(),
            reason: String::new(),
            risk: Risk::High,
            reasons: vec![],
            timeout_secs: 60,
            kind: "approval",
        };
        let json = serde_json::to_value(request).unwrap();
        assert_eq!(
            json["timeoutSecs"], 60,
            "the card's countdown reads timeoutSecs (it showed 0:00 when this was timeout_secs)"
        );
        assert!(json.get("timeout_secs").is_none());
    }
}
