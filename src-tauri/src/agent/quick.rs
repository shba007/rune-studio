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
/// costs a click; not asking costs more, so this is lower than the others. The picked probability of a
/// real purchase click was 0.59 and of Shift+Delete 0.95 (field test), so 0.8 missed the purchase.
const RISKY: f64 = 0.5;

/// Can this task be done as one goal? Then the planning call (one full chat completion) is skipped.
/// A wrong yes only means the progress ring has one arc instead of several.
pub async fn is_simple_task(app: &AppHandle, task: &str) -> bool {
    let question = Question::yes_no(
        "simple",
        crate::prompts::quick_simple(),
    );
    let text = format!("Request: {}", task.trim());
    decision::ask(app, &text, &[], &[question])
        .await
        .is_some_and(|answers| answers.confident_yes("simple", SIMPLE_TASK))
}

/// Does the screen show that the whole task has been carried out? Then the loop ends without
/// asking the big model to say so. `actions` are the steps taken so far, in plain words.
///
/// A small model cannot read fine print, so this is only asked once every goal has had its steps.
/// If it says no or is unsure, the big model decides, as before.
pub async fn looks_finished(app: &AppHandle, task: &str, actions: &str, screenshot: &str) -> bool {
    let question = Question::yes_no(
        "finished",
        crate::prompts::quick_finished(),
    );
    let text = format!(
        "Request: {}\nSteps taken so far:\n{}",
        task.trim(),
        actions.trim()
    );
    decision::ask(app, &text, &[screenshot.to_string()], &[question])
        .await
        .is_some_and(|answers| answers.confident_yes("finished", FINISHED))
}

/// A second opinion on how risky an action is, from the action itself and the window it targets.
/// It can only raise the risk, and only to High, so a wrong answer costs the user one click.
pub async fn risk_opinion(
    app: &AppHandle,
    action: &str,
    reason: &str,
    window: &str,
) -> Option<Risk> {
    let question = Question::choice(
        "risk",
        crate::prompts::quick_risk(),
        &["low", "medium", "high"],
    );
    let text = format!("Action: {action}\nWhy: {reason}\nWindow in front: {window}");
    let answers = decision::ask(app, &text, &[], &[question]).await?;
    let (level, confidence) = answers.choice("risk")?;
    (level == "high" && confidence >= RISKY).then_some(Risk::High)
}

/// Most waits for a loading page before the big model is asked anyway.
pub const MAX_LOAD_WAITS: u32 = 4;
/// How sure the model must be that the screen is still loading before the loop waits.
const LOADING: f64 = 0.9;
/// How sure it must be about something in the way before the big model is told.
const BLOCKER: f64 = 0.8;
const BLOCKERS: &[&str] = &[
    "none",
    "cookie_banner",
    "popup",
    "login_required",
    "captcha",
    "error_page",
];

/// What the decision model notices on a screenshot. Both are only advice: a wait, or a hint in
/// the next prompt. The big model still chooses every action.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScreenNotes {
    /// The page or app still looks like it is loading (blank, half drawn, a spinner).
    pub loading: bool,
    /// A note for the big model about something in the way.
    pub hint: Option<&'static str>,
}

/// The note the big model gets for something in the way. A CAPTCHA or a login is never worked
/// around: it is the user's to handle.
pub fn hint_for(blocker: &str) -> Option<&'static str> {
    crate::prompts::blocker_hint(blocker)
}

/// Reads the screen in one quick pass: is it still loading, and is something in the way? `None`
/// when the decision model has nothing to say.
pub async fn read_screen(app: &AppHandle, screenshot: &str) -> Option<ScreenNotes> {
    let questions = [
        Question::yes_no(
            "loading",
            crate::prompts::quick_loading(),
        ),
        Question::choice(
            "blocker",
            crate::prompts::quick_blocker(),
            BLOCKERS,
        ),
    ];
    let answers = decision::ask(
        app,
        "A screenshot of the user's screen.",
        &[screenshot.to_string()],
        &questions,
    )
    .await?;
    Some(ScreenNotes {
        loading: answers.confident_yes("loading", LOADING),
        hint: answers
            .choice("blocker")
            .filter(|(_, confidence)| *confidence >= BLOCKER)
            .and_then(|(blocker, _)| hint_for(&blocker)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_blocker_but_none_has_a_hint_and_nothing_gets_worked_around() {
        for blocker in BLOCKERS {
            assert_eq!(hint_for(blocker).is_some(), *blocker != "none", "{blocker}");
        }
        assert!(
            hint_for("captcha")
                .unwrap()
                .contains("Never try to solve it")
        );
        assert!(
            hint_for("login_required")
                .unwrap()
                .contains("Never enter credentials")
        );
        assert!(hint_for("something else").is_none());
    }
}
