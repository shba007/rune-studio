//! Quick yes/no and pick-one questions for rune-infer's decision model (`POST /v1/decisions`).
//!
//! The decision model reads a text (and optionally screenshots) and answers a fixed list of
//! questions in a single forward pass: no tokens are generated, so an answer takes a fraction of
//! a chat completion. That makes it good for gating and skipping work, and the limits shape how
//! the app uses it:
//!
//! - It can only pick from what it is offered: yes or no, one of a few options, a level. It
//!   cannot write text, coordinates or tool arguments, so it never replaces the model that
//!   chooses the actions.
//! - It is a small model. Its answers come with a probability, and callers act only on a
//!   confident answer in the direction where a mistake is cheap: skipping work (a wrong yes
//!   costs a retry), or asking the user (a wrong yes costs a click). It never relaxes a safety
//!   check.
//! - It is optional. When it is off, missing, slow or failing, `ask` returns `None` and the
//!   caller does exactly what it did before; after a failure it is left alone for a minute.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use crate::{events::AppState, settings};

/// A text-only question is a single short pass; one with a screenshot reads a few hundred image tokens.
const TEXT_TIMEOUT: Duration = Duration::from_secs(4);
const IMAGE_TIMEOUT: Duration = Duration::from_secs(8);
/// After a failure the decision model is not asked again for this long.
const COOLDOWN: Duration = Duration::from_secs(60);

static FAILED_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// One question for the decision model.
pub struct Question {
    name: &'static str,
    instruction: String,
    options: Option<&'static [&'static str]>,
}

impl Question {
    /// A yes or no question. Word it so that "yes" is the answer you can act on.
    pub fn yes_no(name: &'static str, instruction: impl Into<String>) -> Self {
        Self { name, instruction: instruction.into(), options: None }
    }

    /// Pick one of `options`.
    pub fn choice(name: &'static str, instruction: impl Into<String>, options: &'static [&'static str]) -> Self {
        Self { name, instruction: instruction.into(), options: Some(options) }
    }

    fn to_json(&self) -> Value {
        // The server calls a yes/no question "noul".
        match self.options {
            Some(options) => json!({ "name": self.name, "type": "choice", "instruction": self.instruction, "criteria": options }),
            None => json!({ "name": self.name, "type": "noul", "instruction": self.instruction }),
        }
    }
}

/// The model's answers, by question name.
#[derive(Debug, Default)]
pub struct Answers(HashMap<String, Value>);

impl Answers {
    pub fn from_response(response: &Value) -> Self {
        let answers = response["answers"].as_object().map(|map| map.iter().map(|(name, value)| (name.clone(), value.clone())).collect());
        Self(answers.unwrap_or_default())
    }

    /// True only when the model said yes and was at least `min_probability` sure. Anything else,
    /// including a missing answer, is "not sure", never "no".
    pub fn confident_yes(&self, name: &str, min_probability: f64) -> bool {
        let Some(answer) = self.0.get(name) else { return false };
        answer["decision"].as_bool() == Some(true) && answer["probability"].as_f64().is_some_and(|p| p >= min_probability)
    }

    /// The option picked for a `choice` question and how confident the model is.
    pub fn choice(&self, name: &str) -> Option<(String, f64)> {
        let answer = self.0.get(name)?;
        Some((answer["decision"].as_str()?.to_string(), answer["confidence"].as_f64()?))
    }
}

fn request_body(model: &str, text: &str, images: &[String], questions: &[Question]) -> Value {
    let state = if images.is_empty() { json!(text) } else { json!({ "text": text, "images": images }) };
    json!({ "model": model, "state": state, "questions": questions.iter().map(Question::to_json).collect::<Vec<_>>() })
}

fn cooling_down() -> bool {
    FAILED_AT.lock().ok().and_then(|failed| *failed).is_some_and(|at| at.elapsed() < COOLDOWN)
}

fn note_failure() {
    if let Ok(mut failed) = FAILED_AT.lock() {
        *failed = Some(Instant::now());
    }
}

/// Asks the decision model. `None` means "no answer, do it the usual way": the feature is off,
/// the model is cooling down after a failure, or the call failed.
pub async fn ask(app: &AppHandle, text: &str, images: &[String], questions: &[Question]) -> Option<Answers> {
    let state = app.state::<AppState>();
    let (enabled, base_url, model) = {
        let settings = state.settings.lock().expect("settings lock poisoned");
        (settings.fast_checks, settings.base_url.clone(), settings.decision_model.clone())
    };
    if !enabled || model.trim().is_empty() || cooling_down() {
        return None;
    }

    let started = Instant::now();
    let mut request = state
        .http
        .post(format!("{}/v1/decisions", base_url.trim_end_matches('/')))
        .timeout(if images.is_empty() { TEXT_TIMEOUT } else { IMAGE_TIMEOUT })
        .json(&request_body(&model, text, images, questions));
    if let Some(key) = settings::api_key() {
        request = request.bearer_auth(key);
    }

    let outcome = async {
        let response = request.send().await.map_err(|err| err.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("status {status}"));
        }
        response.json::<Value>().await.map_err(|err| err.to_string())
    }
    .await;

    match outcome {
        Ok(value) => {
            let answers = Answers::from_response(&value);
            log::info!("decision model answered {} question(s) in {} ms", answers.0.len(), started.elapsed().as_millis());
            Some(answers)
        }
        Err(err) => {
            log::warn!("decision model unavailable ({err}); fast checks paused for a minute");
            note_failure();
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        json!({
            "id": "d1", "object": "decision", "created": 0, "model": "liquid-d1-3b",
            "answers": {
                "single": { "decision": true, "probability": 0.96 },
                "unsure": { "decision": true, "probability": 0.6 },
                "no": { "decision": false, "probability": 0.99 },
                "risk": { "decision": "high", "confidence": 0.88, "probabilities": { "low": 0.02, "medium": 0.1, "high": 0.88 } }
            },
            "usage": { "prompt_tokens": 40, "completion_tokens": 0 }
        })
    }

    #[test]
    fn only_a_confident_yes_counts() {
        let answers = Answers::from_response(&sample());
        assert!(answers.confident_yes("single", 0.9));
        assert!(!answers.confident_yes("unsure", 0.9), "a hesitant yes is not enough");
        assert!(!answers.confident_yes("no", 0.9), "a no is never a yes");
        assert!(!answers.confident_yes("missing", 0.0), "no answer is not a yes");
    }

    #[test]
    fn choices_come_back_with_their_confidence() {
        let answers = Answers::from_response(&sample());
        assert_eq!(answers.choice("risk"), Some(("high".to_string(), 0.88)));
        assert_eq!(answers.choice("single"), None, "a yes/no answer is not a choice");
    }

    #[test]
    fn a_reply_without_answers_is_empty_not_an_error() {
        let answers = Answers::from_response(&json!({ "error": { "message": "nope" } }));
        assert!(!answers.confident_yes("single", 0.0));
        assert_eq!(answers.choice("risk"), None);
    }

    #[test]
    fn the_request_uses_the_servers_names() {
        let questions = [Question::yes_no("done", "Is it done?"), Question::choice("risk", "How risky?", &["low", "high"])];
        let text_only = request_body("m", "state", &[], &questions);
        assert_eq!(text_only["state"], "state");
        assert_eq!(text_only["questions"][0]["type"], "noul");
        assert_eq!(text_only["questions"][1]["type"], "choice");
        assert_eq!(text_only["questions"][1]["criteria"], json!(["low", "high"]));

        let with_image = request_body("m", "state", &["data:image/jpeg;base64,AA".to_string()], &questions);
        assert_eq!(with_image["state"]["text"], "state");
        assert_eq!(with_image["state"]["images"][0], "data:image/jpeg;base64,AA");
    }
}
