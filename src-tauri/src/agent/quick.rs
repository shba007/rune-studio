//! Quick checks with the decision model (see `model::decision`): the places where a fast yes/no
//! saves a slow chat call, or adds a cheap second opinion.
//!
//! The rule for every check here is the same: act only on a *confident* answer, and only in the
//! direction where being wrong is cheap. Each one returns plain `false` / `None` when the decision
//! model has nothing useful to say, and the caller carries on exactly as it would without it.

use tauri::AppHandle;

use crate::{
    model::decision::{self, Question},
    safety::Risk,
};

/// How sure the model must be that a task is simple before the planning call is skipped.
const SIMPLE_TASK: f64 = 0.9;
/// How sure it must be that the task is finished before the next chat call is skipped. Higher,
/// because stopping early is the one mistake here the user would notice.
const FINISHED: f64 = 0.97;
/// How sure it must be that an action is high risk before the user is asked. Asking too often
/// costs a click; not asking costs more, so this is lower than the others.
const RISKY: f64 = 0.8;

/// Can this task be done as one goal? Then the planning call (one full chat completion) is skipped.
/// A wrong yes only means the progress ring has one arc instead of several.
pub async fn is_simple_task(app: &AppHandle, task: &str) -> bool {
    let question = Question::yes_no(
        "simple",
        "Can this request be done as one short step, such as opening a single app, or typing one short line, without separate stages?",
    );
    let text = format!("Request: {}", task.trim());
    decision::ask(app, &text, &[], &[question]).await.is_some_and(|answers| answers.confident_yes("simple", SIMPLE_TASK))
}

/// Does the screen show that the whole task has been carried out? Then the loop ends without
/// asking the big model to say so. `actions` are the steps taken so far, in plain words.
///
/// A small model cannot read fine print, so this is only asked once every goal has had its steps.
/// If it says no or is unsure, the big model decides, as before.
pub async fn looks_finished(app: &AppHandle, task: &str, actions: &str, screenshot: &str) -> bool {
    let question = Question::yes_no(
        "finished",
        "Looking at the screen and the steps taken, has the whole request been completely carried out, with nothing left to do?",
    );
    let text = format!("Request: {}\nSteps taken so far:\n{}", task.trim(), actions.trim());
    decision::ask(app, &text, &[screenshot.to_string()], &[question])
        .await
        .is_some_and(|answers| answers.confident_yes("finished", FINISHED))
}

/// A second opinion on how risky an action is, from the action itself and the window it targets.
/// It can only raise the risk, and only to High, so a wrong answer costs the user one click.
pub async fn risk_opinion(app: &AppHandle, action: &str, reason: &str, window: &str) -> Option<Risk> {
    let question = Question::choice(
        "risk",
        "How much harm could this action do on the user's computer? High means it could delete or overwrite data, spend money, send a message, change security settings or install something.",
        &["low", "medium", "high"],
    );
    let text = format!("Action: {action}\nWhy: {reason}\nWindow in front: {window}");
    let answers = decision::ask(app, &text, &[], &[question]).await?;
    let (level, confidence) = answers.choice("risk")?;
    (level == "high" && confidence >= RISKY).then_some(Risk::High)
}
