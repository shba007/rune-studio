use crate::sync::LockExt;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use crate::{events::AppState, settings};

const TEXT_TIMEOUT: Duration = Duration::from_secs(4);
const IMAGE_TIMEOUT: Duration = Duration::from_secs(8);
const COOLDOWN: Duration = Duration::from_secs(60);

static FAILED_AT: Mutex<Option<Instant>> = Mutex::new(None);

pub struct Question {
    name: &'static str,
    instruction: String,
    options: Option<&'static [&'static str]>,
}

impl Question {
    pub fn yes_no(name: &'static str, instruction: impl Into<String>) -> Self {
        Self {
            name,
            instruction: instruction.into(),
            options: None,
        }
    }

    pub fn choice(
        name: &'static str,
        instruction: impl Into<String>,
        options: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            instruction: instruction.into(),
            options: Some(options),
        }
    }

    fn to_json(&self) -> Value {
        match self.options {
            Some(options) => {
                json!({ "name": self.name, "type": "choice", "instruction": self.instruction, "criteria": options })
            }
            None => json!({ "name": self.name, "type": "noul", "instruction": self.instruction }),
        }
    }
}

#[derive(Debug, Default)]
pub struct Answers(HashMap<String, Value>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Source,
    Spec,
}

fn shape_of(answer: &Value) -> Shape {
    if answer["type"].is_string() && !answer["value"].is_null() {
        Shape::Spec
    } else {
        Shape::Source
    }
}

impl Answers {
    pub fn from_response(response: &Value) -> Self {
        let answers: Option<HashMap<String, Value>> = response["answers"].as_object().map(|map| {
            map.iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect()
        });
        let answers = answers.unwrap_or_default();
        if let Some(first) = answers.values().next() {
            static LOGGED: [std::sync::atomic::AtomicBool; 2] = [
                std::sync::atomic::AtomicBool::new(false),
                std::sync::atomic::AtomicBool::new(false),
            ];
            let shape = shape_of(first);
            if !LOGGED[shape as usize].swap(true, std::sync::atomic::Ordering::Relaxed) {
                log::info!("decision model answers in the {shape:?} shape");
            }
        }
        let answers = Self(answers);
        let mut names: Vec<&String> = answers.0.keys().collect();
        names.sort();
        for name in names {
            let answer = &answers.0[name];
            let shape = match shape_of(answer) {
                Shape::Source => "source",
                Shape::Spec => "spec",
            };
            let value = if answer["decision"].is_null() {
                &answer["value"]
            } else {
                &answer["decision"]
            };
            let number = |number: Option<f64>| {
                number.map_or_else(|| "-".to_string(), |number| format!("{number:.4}"))
            };
            log::info!(
                "decision answer {name}: shape={shape} decision={value} confidence={} probability={} p(picked)={}",
                number(answer["confidence"].as_f64()),
                number(answer["probability"].as_f64()),
                number(answers.picked_probability(name)),
            );
        }
        answers
    }

    pub fn confident_yes(&self, name: &str, min_probability: f64) -> bool {
        let Some(answer) = self.0.get(name) else {
            return false;
        };
        match shape_of(answer) {
            Shape::Source => {
                answer["decision"].as_bool() == Some(true)
                    && answer["probability"]
                        .as_f64()
                        .is_some_and(|p| p >= min_probability)
            }
            Shape::Spec => {
                let yes = answer["type"] == "noul"
                    && (answer["value"].as_bool() == Some(true)
                        || answer["value"]
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case("yes")));
                yes && answer["confidence"]
                    .as_f64()
                    .is_some_and(|p| p >= min_probability)
            }
        }
    }

    pub fn picked_probability(&self, name: &str) -> Option<f64> {
        let answer = self.0.get(name)?;
        answer["probabilities"][answer["decision"].as_str()?].as_f64()
    }

    pub fn choice(&self, name: &str) -> Option<(String, f64)> {
        let answer = self.0.get(name)?;
        match shape_of(answer) {
            Shape::Source => Some((
                answer["decision"].as_str()?.to_string(),
                self.picked_probability(name)
                    .or_else(|| answer["confidence"].as_f64())?,
            )),
            Shape::Spec if answer["type"] == "choice" => Some((
                answer["value"].as_str()?.to_string(),
                answer["confidence"].as_f64()?,
            )),
            Shape::Spec => None,
        }
    }
}

fn request_body(model: &str, text: &str, images: &[String], questions: &[Question]) -> Value {
    let state = if images.is_empty() {
        json!(text)
    } else {
        json!({ "text": text, "images": images })
    };
    json!({ "model": model, "state": state, "questions": questions.iter().map(Question::to_json).collect::<Vec<_>>() })
}

fn endpoint(decision_url: &str, base_url: &str) -> String {
    let url = if decision_url.trim().is_empty() {
        base_url
    } else {
        decision_url
    };
    format!("{}/v1/decisions", url.trim().trim_end_matches('/'))
}

fn cooling_down() -> bool {
    FAILED_AT
        .lock()
        .ok()
        .and_then(|failed| *failed)
        .is_some_and(|at| at.elapsed() < COOLDOWN)
}

fn note_failure() {
    if let Ok(mut failed) = FAILED_AT.lock() {
        *failed = Some(Instant::now());
    }
}

pub async fn ask(
    app: &AppHandle,
    text: &str,
    images: &[String],
    questions: &[Question],
) -> Option<Answers> {
    if crate::workspace_tasks::protected_execution_context(app) {
        return None;
    }
    let state = app.state::<AppState>();
    let (enabled, url, model, skip_swap, allowance_secs, api_key) = {
        let settings = state.settings.locked();
        (
            settings.fast_checks,
            endpoint(&settings.decision_url, &settings.base_url),
            settings
                .model_for_role(crate::model::role::ModelRole::Decision)
                .to_string(),
            settings.skip_swap_for_optional_roles,
            settings.first_call_idle_allowance_secs,
            settings::api_key_for(&settings),
        )
    };
    if !enabled || url.starts_with("/v1") || model.trim().is_empty() || cooling_down() {
        return None;
    }

    if skip_swap
        && state
            .capabilities
            .get_model(&model)
            .is_some_and(|caps| caps.resident == crate::model::capabilities::TriState::No)
    {
        log::info!("skipping optional decision call: model '{model}' is not resident");
        return None;
    }

    let is_first_or_idle = crate::model::swap::is_idle_or_first_call(Duration::from_secs(60));
    let timeout = if is_first_or_idle {
        Duration::from_secs(allowance_secs.max(30) as u64)
    } else if images.is_empty() {
        TEXT_TIMEOUT
    } else {
        IMAGE_TIMEOUT
    };

    let started = Instant::now();
    let mut request = state
        .http
        .post(url)
        .timeout(timeout)
        .json(&request_body(&model, text, images, questions));
    request = with_optional_swap_policy(request, skip_swap);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }

    let outcome = send_optional_decision(request).await;

    match outcome {
        Ok(Value::Null) => None,
        Ok(value) => {
            let answers = Answers::from_response(&value);
            log::info!(
                "decision model answered {} question(s) in {} ms",
                answers.0.len(),
                started.elapsed().as_millis()
            );
            crate::model::swap::record_call_activity();
            Some(answers)
        }
        Err(err) => {
            log::warn!("decision model unavailable ({err}); fast checks paused for a minute");
            note_failure();
            None
        }
    }
}

fn with_optional_swap_policy(
    request: reqwest::RequestBuilder,
    skip_swap: bool,
) -> reqwest::RequestBuilder {
    if skip_swap {
        request.header("x-no-swap", "true")
    } else {
        request
    }
}

async fn send_optional_decision(request: reqwest::RequestBuilder) -> Result<Value, String> {
    let response = request.send().await.map_err(|err| err.to_string())?;
    let status = response.status();
    if status.as_u16() == 409 {
        let value = response.json::<Value>().await.map_err(|e| e.to_string())?;
        if crate::model::swap::is_swap_refused(409, &value) {
            log::info!("optional decision skipped: swap_refused");
            return Ok(Value::Null);
        }
        return Err("status 409".into());
    }
    if !status.is_success() {
        return Err(format!("status {status}"));
    }
    response
        .json::<Value>()
        .await
        .map_err(|err| err.to_string())
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
        assert!(
            !answers.confident_yes("unsure", 0.9),
            "a hesitant yes is not enough"
        );
        assert!(!answers.confident_yes("no", 0.9), "a no is never a yes");
        assert!(
            !answers.confident_yes("missing", 0.0),
            "no answer is not a yes"
        );
    }

    #[test]
    fn choices_come_back_with_their_confidence() {
        let answers = Answers::from_response(&sample());
        assert_eq!(answers.choice("risk"), Some(("high".to_string(), 0.88)));
        assert_eq!(
            answers.choice("single"),
            None,
            "a yes/no answer is not a choice"
        );
    }

    #[test]
    fn a_reply_without_answers_is_empty_not_an_error() {
        let answers = Answers::from_response(&json!({ "error": { "message": "nope" } }));
        assert!(!answers.confident_yes("single", 0.0));
        assert_eq!(answers.choice("risk"), None);
    }

    #[test]
    fn the_request_uses_the_servers_names() {
        let questions = [
            Question::yes_no("done", "Is it done?"),
            Question::choice("risk", "How risky?", &["low", "high"]),
        ];
        let text_only = request_body("m", "state", &[], &questions);
        assert_eq!(text_only["state"], "state");
        assert_eq!(text_only["questions"][0]["type"], "noul");
        assert_eq!(text_only["questions"][1]["type"], "choice");
        assert_eq!(
            text_only["questions"][1]["criteria"],
            json!(["low", "high"])
        );

        let with_image = request_body(
            "m",
            "state",
            &["data:image/jpeg;base64,AA".to_string()],
            &questions,
        );
        assert_eq!(with_image["state"]["text"], "state");
        assert_eq!(
            with_image["state"]["images"][0],
            "data:image/jpeg;base64,AA"
        );
    }

    fn answers_of(answer: Value) -> Answers {
        Answers::from_response(&json!({ "answers": { "q": answer } }))
    }

    #[test]
    fn the_spec_shape_is_read_too() {
        let yes = answers_of(json!({"type":"noul","value":"yes","confidence":0.89}));
        assert!(yes.confident_yes("q", 0.85));
        assert!(!yes.confident_yes("q", 0.9), "the threshold is respected");
        assert_eq!(yes.choice("q"), None, "a yes/no answer is not a choice");

        let no = answers_of(json!({"type":"noul","value":"no","confidence":0.99}));
        assert!(!no.confident_yes("q", 0.0), "a no is never a yes");
        let as_bool = answers_of(json!({"type":"noul","value":true,"confidence":0.95}));
        assert!(
            as_bool.confident_yes("q", 0.9),
            "a JSON bool value is accepted"
        );

        let picked =
            answers_of(json!({"type":"choice","value":"canary_rollout","confidence":0.94}));
        assert_eq!(
            picked.choice("q"),
            Some(("canary_rollout".to_string(), 0.94))
        );
        assert!(!picked.confident_yes("q", 0.0), "a choice is not a yes");
    }

    #[test]
    fn the_source_shape_still_reads_with_the_documented_fixtures() {
        let yes = answers_of(json!({"decision":true,"probability":0.96}));
        assert!(yes.confident_yes("q", 0.9));
        assert!(!yes.confident_yes("q", 0.97));
        let high = answers_of(
            json!({"decision":"high","confidence":0.88,"probabilities":{"low":0.02,"medium":0.1,"high":0.88}}),
        );
        assert_eq!(high.choice("q"), Some(("high".to_string(), 0.88)));
    }

    #[test]
    fn an_answer_in_neither_shape_is_not_sure_and_never_no_or_a_pick() {
        for odd in [
            json!({}),
            json!({"type":"noul"}),
            json!({"type":"noul","value":"yes"}),
            json!({"decision":true}),
            json!({"type":"score","value":3.0,"confidence":0.9}),
            json!("yes"),
        ] {
            let answers = answers_of(odd.clone());
            assert!(!answers.confident_yes("q", 0.0), "{odd}");
            assert_eq!(answers.choice("q"), None, "{odd}");
        }
    }

    #[test]
    fn a_choice_is_as_sure_as_its_picked_probability_not_its_confidence() {
        let pay = answers_of(
            json!({"decision":"high","confidence":0.38,"probabilities":{"low":0.2,"medium":0.213,"high":0.587}}),
        );
        assert_eq!(pay.choice("q"), Some(("high".to_string(), 0.587)));
        let delete = answers_of(
            json!({"decision":"high","confidence":0.8,"probabilities":{"low":0.02,"medium":0.06,"high":0.92}}),
        );
        assert_eq!(delete.choice("q"), Some(("high".to_string(), 0.92)));
        let bare = answers_of(json!({"decision":"low","confidence":0.7}));
        assert_eq!(bare.choice("q"), Some(("low".to_string(), 0.7)));
    }

    #[test]
    fn an_empty_decision_url_means_the_same_server_as_the_chat_model() {
        assert_eq!(
            endpoint("", "http://127.0.0.1:8080/"),
            "http://127.0.0.1:8080/v1/decisions"
        );
        assert_eq!(endpoint("  ", "http://a:1"), "http://a:1/v1/decisions");
        assert_eq!(
            endpoint("http://b:2/", "http://a:1"),
            "http://b:2/v1/decisions"
        );
    }

    #[test]
    fn the_pasted_server_answer_reads_as_documented() {
        let answers = Answers::from_response(&json!({ "answers": {
            "routing": { "decision": "blue_green", "confidence": 0.1374, "probabilities": { "immediate_switch": 0.2555, "canary_rollout": 0.3196, "blue_green": 0.4249 } },
            "requires_approval": { "decision": true, "probability": 0.6816 }
        } }));
        assert_eq!(
            answers.choice("routing"),
            Some(("blue_green".to_string(), 0.4249)),
            "the picked probability, not confidence"
        );
        assert_eq!(
            answers.picked_probability("routing"),
            Some(0.4249),
            "confidence is not the picked probability"
        );
        assert_eq!(
            answers.picked_probability("requires_approval"),
            None,
            "a yes/no answer has no picked option"
        );
        assert_eq!(answers.picked_probability("missing"), None);
        assert!(answers.confident_yes("requires_approval", 0.68));
        assert!(!answers.confident_yes("requires_approval", 0.69));
    }
    #[tokio::test]
    async fn decision_refusal_is_a_skip_and_sends_no_swap() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1/decisions", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 8192];
            let n = stream.read(&mut buf).unwrap();
            let head = String::from_utf8_lossy(&buf[..n]).to_lowercase();
            assert!(head.contains("x-no-swap: true"));
            let body = r#"{"error":{"message":"not resident","type":"swap_refused","code":"swap_refused"}}"#;
            write!(
                stream,
                "HTTP/1.1 409 Conflict\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let request =
            with_optional_swap_policy(http.post(url).timeout(Duration::from_secs(5)), true);
        assert_eq!(send_optional_decision(request).await.unwrap(), Value::Null);
        server.join().unwrap();
        let request = with_optional_swap_policy(http.post("http://127.0.0.1"), false)
            .build()
            .unwrap();
        assert!(!request.headers().contains_key("x-no-swap"));
    }
}
