use tauri::AppHandle;

use crate::{
    model::decision::{self, Question},
    safety::Risk,
};

const SIMPLE_TASK: f64 = 0.9;
const FINISHED: f64 = 0.97;
const RISKY: f64 = 0.5;

pub async fn is_simple_task(app: &AppHandle, task: &str) -> bool {
    let question = Question::yes_no("simple", crate::prompts::quick_simple());
    let text = format!("Request: {}", task.trim());
    decision::ask(app, &text, &[], &[question])
        .await
        .is_some_and(|answers| answers.confident_yes("simple", SIMPLE_TASK))
}

pub async fn looks_finished(app: &AppHandle, task: &str, actions: &str, screenshot: &str) -> bool {
    let question = Question::yes_no("finished", crate::prompts::quick_finished());
    let text = format!(
        "Request: {}\nSteps taken so far:\n{}",
        task.trim(),
        actions.trim()
    );
    decision::ask(app, &text, &[screenshot.to_string()], &[question])
        .await
        .is_some_and(|answers| answers.confident_yes("finished", FINISHED))
}

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

pub const MAX_LOAD_WAITS: u32 = 4;
const LOADING: f64 = 0.9;
const BLOCKER: f64 = 0.8;
const BLOCKERS: &[&str] = &[
    "none",
    "cookie_banner",
    "popup",
    "login_required",
    "captcha",
    "error_page",
];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScreenNotes {
    pub loading: bool,
    pub hint: Option<&'static str>,
}

pub fn hint_for(blocker: &str) -> Option<&'static str> {
    crate::prompts::blocker_hint(blocker)
}

pub async fn read_screen(app: &AppHandle, screenshot: &str) -> Option<ScreenNotes> {
    let questions = [
        Question::yes_no("loading", crate::prompts::quick_loading()),
        Question::choice("blocker", crate::prompts::quick_blocker(), BLOCKERS),
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
