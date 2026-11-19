use crate::{
    events,
    model::{
        attempts::{Attempt, AttemptSink},
        client::ChatClient,
    },
    sync::LockExt,
};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Default)]
struct Usage {
    attempts: u32,
    model: Option<String>,
    tokens: Option<u64>,
    estimated: bool,
}
pub struct ModelStep<'a> {
    emit: Box<dyn Fn(events::StepFinished) + Send + Sync + 'a>,
    task_id: Option<u64>,
    request_id: Option<u64>,
    id: String,
    purpose: &'static str,
    sink: Option<&'a dyn AttemptSink>,
    usage: Mutex<Usage>,
    finished: bool,
}
impl<'a> ModelStep<'a> {
    pub fn start(
        app: &'a tauri::AppHandle,
        client: &'a ChatClient<'_>,
        task_id: Option<u64>,
        request_id: Option<u64>,
        purpose: &'static str,
    ) -> Self {
        let request_id =
            request_id.or_else(|| crate::live_run::live_run_snapshot().request.map(|r| r.id));
        Self::begin(
            client,
            task_id,
            request_id,
            purpose,
            |event| events::emit_step_started(app, event),
            move |event| events::emit_step_finished(app, event),
        )
    }
    fn begin(
        client: &'a ChatClient<'_>,
        task_id: Option<u64>,
        request_id: Option<u64>,
        purpose: &'static str,
        emit_start: impl FnOnce(events::StepStarted),
        emit_finish: impl Fn(events::StepFinished) + Send + Sync + 'a,
    ) -> Self {
        let id = format!("model:{}", NEXT.fetch_add(1, Ordering::Relaxed));
        emit_start(events::StepStarted {
            task_id,
            request_id,
            step_id: Some(id.clone()),
            step: 0,
            kind: "model",
            purpose: Some(purpose),
            model: Some(client.model.into()),
            tool: "model",
            reason: String::new(),
            text: purpose.into(),
            detail: String::new(),
            point: None,
            double: false,
            button: "left",
            duration_ms: 0,
        });
        Self {
            emit: Box::new(emit_finish),
            task_id,
            request_id,
            id,
            purpose,
            sink: client.sink,
            usage: Mutex::new(Usage::default()),
            finished: false,
        }
    }
    pub fn client<'b>(&'b self, original: &'b ChatClient<'b>) -> ChatClient<'b> {
        ChatClient {
            app: original.app,
            output_limit: original.output_limit,
            http: original.http,
            base_url: original.base_url,
            api_key: original.api_key,
            model: original.model,
            profile: original.profile,
            sink: Some(self),
            no_swap: original.no_swap,
            on_loading: original.on_loading,
        }
    }
    pub fn finish(&mut self, error: Option<String>) {
        if self.finished {
            return;
        }
        self.finished = true;
        let error = error.map(event_error);
        let usage = self.usage.locked();
        (self.emit)(events::StepFinished {
            task_id: self.task_id,
            request_id: self.request_id,
            step_id: Some(self.id.clone()),
            step: 0,
            purpose: Some(self.purpose),
            model: usage.model.clone(),
            completion_tokens: usage.tokens,
            tokens_estimated: usage.tokens.map(|_| usage.estimated),
            attempts: Some(usage.attempts),
            ok: error.is_none(),
            error,
            result: None,
        });
    }
}
fn event_error(error: String) -> String {
    for category in [
        "the model server answered ",
        "the model's reply was not understood",
        "the model did not call a tool",
        "the model's reply reached the output limit",
        "the model's tool arguments were not valid JSON",
    ] {
        if error.starts_with(category) {
            return error.split(':').next().unwrap_or(category).to_string();
        }
    }
    error
}
impl AttemptSink for ModelStep<'_> {
    fn record(&self, attempt: Attempt) {
        {
            let mut usage = self.usage.locked();
            usage.attempts += 1;
            if attempt.served_model.is_some() {
                usage.model = attempt.served_model.clone();
            }
            let response = &attempt.response;
            let message = if response["streamed"] == true {
                response
            } else {
                &response["choices"][0]["message"]
            };
            let mut output = message["content"].as_str().unwrap_or("").to_string();
            if let Some(calls) = message["tool_calls"].as_array() {
                for call in calls {
                    output.push_str(
                        call["function"]["arguments"]
                            .as_str()
                            .or_else(|| call["arguments"].as_str())
                            .unwrap_or(""),
                    );
                }
            }
            let thinking = message["reasoning_content"]
                .as_str()
                .or_else(|| message["reasoning"].as_str())
                .unwrap_or("");
            let record = crate::trace::StepRecord {
                completion_tokens: attempt
                    .completion_tokens
                    .and_then(|n| i64::try_from(n).ok()),
                args_json: Some(output),
                thinking_chars: Some(thinking.chars().count() as i64),
                ..Default::default()
            };
            if let Some((tokens, estimated)) = crate::model::attempts::tokens_of(
                record.completion_tokens,
                record
                    .args_json
                    .as_ref()
                    .map_or(0, |args| args.chars().count() as i64)
                    + record.thinking_chars.unwrap_or(0),
            ) {
                usage.tokens = u64::try_from(tokens).ok();
                usage.estimated = estimated;
            }
        }
        if let Some(sink) = self.sink {
            sink.record(attempt);
        }
    }
}
impl Drop for ModelStep<'_> {
    fn drop(&mut self) {
        self.finish(Some("Cancelled".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{attempts::Collect, client::Request};
    use serde_json::{Value, json};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };

    fn client<'a>(http: &'a reqwest::Client, url: &'a str, sink: &'a Collect) -> ChatClient<'a> {
        ChatClient {
            app: None,
            http,
            base_url: url,
            api_key: None,
            model: "requested",
            profile: None,
            sink: Some(sink),
            no_swap: false,
            on_loading: None,
            output_limit: None,
        }
    }
    #[test]
    fn reply_excerpts_and_server_bodies_are_not_step_errors() {
        for error in [
            "the model server answered 400: private reply",
            "the model did not call a tool: private reply",
            "the model's reply was not understood: private reply",
            "the model's tool arguments were not valid JSON: private reply",
        ] {
            assert!(!event_error(error.into()).contains("private reply"));
        }
    }
    fn serve(responses: Vec<(&'static str, String, Duration)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for (status, body, delay) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buf = [0; 4096];
                loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|s| s.strip_prefix("content-length:"))
                            .and_then(|s| s.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                std::thread::sleep(delay);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        url
    }
    #[tokio::test]
    async fn retries_are_one_step_with_actual_model_tokens_and_unchanged_model_answer() {
        let body = json!({"model":"served", "choices":[{"message":{"tool_calls":[{"function":{"name":"click", "arguments":"{\"x\":1,\"y\":2}"}}]}}], "usage":{"completion_tokens":18}}).to_string();
        let url = serve(vec![
            ("503 Service Unavailable", "busy".into(), Duration::ZERO),
            ("200 OK", body.clone(), Duration::ZERO),
            ("200 OK", body, Duration::ZERO),
        ]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = Collect::default();
        let client = client(&http, &url, &sink);
        let starts = Mutex::new(Vec::<Value>::new());
        let finishes = Mutex::new(Vec::<Value>::new());
        let mut step = ModelStep::begin(
            &client,
            Some(7),
            Some(3),
            "Decide",
            |e| starts.locked().push(serde_json::to_value(e).unwrap()),
            |e| finishes.locked().push(serde_json::to_value(e).unwrap()),
        );
        let request = Request::simple("private prompt");
        let wrapped = step
            .client(&client)
            .call_retrying(&request, &mut |_| {})
            .await
            .unwrap();
        step.finish(None);
        drop(step);
        let plain = client.call_retrying(&request, &mut |_| {}).await.unwrap();
        assert_eq!(wrapped.call.name, plain.call.name);
        assert_eq!(
            wrapped.call.arguments.to_string(),
            plain.call.arguments.to_string()
        );
        assert_eq!(starts.locked().len(), 1);
        let finishes = finishes.locked();
        assert_eq!(finishes.len(), 1);
        let finish = &finishes[0];
        assert_eq!(finish["model"], "served");
        assert_eq!(finish["completionTokens"], 18);
        assert_eq!(finish["tokensEstimated"], false);
        assert_eq!(finish["attempts"], 2);
        assert_eq!(finish["stepId"], starts.locked()[0]["stepId"]);
        assert_eq!(sink.0.locked().len(), 3);
        assert!(finish.get("result").is_none());
        assert!(!finish.to_string().contains("private prompt"));
    }
    #[tokio::test]
    async fn every_run_purpose_has_start_finish_and_usage_even_for_json_calls() {
        let body = json!({"model":"served", "choices":[{"message":{"content":"{\"answer\":\"hello world\"}"}}],"usage":{"completion_tokens":0}}).to_string();
        let url = serve(
            (0..4)
                .map(|_| ("200 OK", body.clone(), Duration::ZERO))
                .collect(),
        );
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = Collect::default();
        let client = client(&http, &url, &sink);
        for purpose in ["Plan", "Decide", "Answer", "Check"] {
            let starts = Mutex::new(Vec::new());
            let finishes = Mutex::new(Vec::new());
            let mut step = ModelStep::begin(
                &client,
                None,
                Some(1),
                purpose,
                |e| starts.locked().push(e),
                |e| finishes.locked().push(e),
            );
            let value = step
                .client(&client)
                .call_json(&Request::simple("private"), &json!({"type":"object"}))
                .await
                .unwrap();
            step.finish(None);
            drop(step);
            assert_eq!(value, json!({"answer":"hello world"}));
            assert_eq!(starts.locked()[0].purpose, Some(purpose));
            let finishes = finishes.locked();
            assert_eq!(finishes[0].purpose, Some(purpose));
            assert!(finishes[0].ok);
            assert_eq!(finishes[0].completion_tokens, Some(6));
            assert_eq!(finishes[0].tokens_estimated, Some(true));
            assert_eq!(finishes[0].attempts, Some(1));
        }
    }
    #[tokio::test]
    async fn cancellation_and_failure_finish_once_without_a_result() {
        let url = serve(vec![("200 OK", "{}".into(), Duration::from_millis(200))]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = Collect::default();
        let client = client(&http, &url, &sink);
        let finishes = Mutex::new(Vec::new());
        {
            let step = ModelStep::begin(
                &client,
                Some(1),
                Some(1),
                "Check",
                |_| {},
                |e| finishes.locked().push(e),
            );
            let wrapped = step.client(&client);
            assert!(
                tokio::time::timeout(
                    Duration::from_millis(60),
                    wrapped.call_json(&Request::simple("private"), &json!({"type":"object"}))
                )
                .await
                .is_err()
            );
        }
        let mut step = ModelStep::begin(
            &client,
            Some(1),
            Some(1),
            "Answer",
            |_| {},
            |e| finishes.locked().push(e),
        );
        step.finish(Some("Server unavailable".into()));
        drop(step);
        let finishes = finishes.locked();
        assert_eq!(finishes.len(), 2);
        assert_eq!(finishes[0].error.as_deref(), Some("Cancelled"));
        assert_eq!(finishes[0].attempts, Some(1));
        assert_eq!(finishes[1].error.as_deref(), Some("Server unavailable"));
        assert!(finishes.iter().all(|e| !e.ok && e.result.is_none()));
    }
}
