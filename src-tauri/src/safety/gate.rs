//! Asking the user. An action that needs a yes waits here until the user answers, presses Stop,
//! or the question times out; the answer comes back through the `confirm` command.

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
/// An unanswered question is a no, so a forgotten prompt cannot approve anything later.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(120);

/// A question waiting for its answer.
pub struct Pending {
    pub id: String,
    sender: oneshot::Sender<bool>,
}

#[derive(Clone, Serialize)]
pub struct ConfirmRequest {
    pub id: String,
    /// Who wants to do it: "Rune" or the outside agent's name.
    pub source: String,
    /// What it wants to do, in a short line.
    pub text: String,
    /// Why, in the agent's words.
    pub reason: String,
    pub risk: Risk,
    /// What made this risky, in plain words.
    pub reasons: Vec<String>,
    pub timeout_secs: u64,
    /// "approval" (an action wants a yes) or "question" (the agent asks something).
    pub kind: &'static str,
}

#[derive(Clone, Serialize)]
pub struct ConfirmResolved {
    pub id: String,
    pub approved: bool,
}

/// What the gate decided about one action.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    /// The user said no, or did not answer in time.
    Denied(&'static str),
    /// The user pressed Stop while we were asking.
    Stopped,
}

/// Completes the waiting question with the user's answer. Returns false if there is none (or the
/// id is stale), so an old button press cannot approve a newer action.
pub fn answer(app: &AppHandle, id: &str, approved: bool) -> bool {
    let state = app.state::<AppState>();
    let mut slot = state.confirm.lock().expect("confirm lock poisoned");
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

fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    format!("q{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Weighs an action and, if it needs a yes, asks for one. `source` is who chose the action.
/// `stated` is the risk the agent itself declared, if any.
pub async fn check(app: &AppHandle, cancel: &CancelToken, source: &str, action: &Action, reason: &str, stated: Option<Risk>) -> Verdict {
    let (ask_every_action, assessment) = {
        let state = app.state::<AppState>();
        let ask = state.settings.lock().expect("settings lock poisoned").ask_every_action;
        (ask, rules::assess(action, reason, &foreground::current()))
    };
    let Assessment { risk: found, mut reasons } = assessment;
    let mut risk = found.max(stated.unwrap_or_default());
    if stated == Some(Risk::High) && found != Risk::High {
        reasons.push("The agent marked it as high risk".into());
    }
    let text = action.describe();
    // A quick second opinion from the decision model, only when nothing has said "high" yet.
    // It can raise the risk, never lower it, and Stop still works while it thinks.
    if risk != Risk::High && !rules::is_harmless(action) {
        let window = foreground::current();
        let window = format!("{} ({})", window.title, window.process);
        let opinion = tokio::select! {
            opinion = crate::agent::quick::risk_opinion(app, &text, reason, &window) => opinion,
            () = until_stopped(cancel) => return Verdict::Stopped,
        };
        if let Some(opinion) = opinion {
            risk = risk.max(opinion);
            reasons.push("A quick safety check rated it high risk".into());
        }
    }
    let ask = needs_confirmation(risk, ask_every_action, rules::is_harmless(action));

    let verdict = if ask { ask_user(app, cancel, source, &text, reason, risk, &reasons, "approval").await } else { Verdict::Allow };

    if !rules::is_harmless(action) {
        let decision = match (&verdict, ask) {
            (Verdict::Allow, false) => "allowed",
            (Verdict::Allow, true) => "approved",
            (Verdict::Denied(why), _) => why,
            (Verdict::Stopped, _) => "stopped",
        };
        audit::record(app, json!({ "source": source, "action": text, "reason": reason, "risk": risk, "why": reasons, "decision": decision }));
    }
    verdict
}

/// Resolves when the user presses Stop.
async fn until_stopped(cancel: &CancelToken) {
    while !cancel.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// The agent asks the user a yes/no question. Yes comes back as `Allow`, no and no answer as
/// `Denied`. It is not an action, so it is neither assessed nor written to the audit log.
pub async fn ask_question(app: &AppHandle, cancel: &CancelToken, source: &str, question: &str) -> Verdict {
    ask_user(app, cancel, source, question, "", Risk::Low, &[], "question").await
}

#[allow(clippy::too_many_arguments)]
async fn ask_user(app: &AppHandle, cancel: &CancelToken, source: &str, text: &str, reason: &str, risk: Risk, reasons: &[String], kind: &'static str) -> Verdict {
    let id = new_id();
    let (sender, receiver) = oneshot::channel();
    *app.state::<AppState>().confirm.lock().expect("confirm lock poisoned") = Some(Pending { id: id.clone(), sender });

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
    // The pill may be hidden from the tray; a question must be seen.
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

    // Whatever happened, nothing may stay waiting.
    app.state::<AppState>().confirm.lock().expect("confirm lock poisoned").take();
    let _ = app.emit(CONFIRM_RESOLVED, ConfirmResolved { id, approved: outcome == Verdict::Allow });
    outcome
}
