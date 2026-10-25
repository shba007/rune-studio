use std::{collections::BTreeMap, time::Duration};

use super::attempts::{Attempt, AttemptSink, Guard};
use crate::model::{
    capabilities::TriState,
    profile::{ModelProfile, ThinkingStyle, ThinkingSwitch},
};
use serde_json::{Value, json};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_ATTEMPTS: u32 = 3;
const RETRY_WAITS: [Duration; 2] = [Duration::from_millis(500), Duration::from_millis(1500)];

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("could not reach the model: {0}")]
    Request(String),
    #[error("the model server answered {status}: {body}")]
    Status { status: u16, body: String },
    #[error("model is loading (estimated {estimated_ms:?} ms)")]
    Loading { estimated_ms: Option<u64> },
    #[error("skipped: model is not resident (swap_refused)")]
    SwapRefused,
    #[error("deferred: server busy; try again after {retry_after_secs} seconds")]
    Deferred { retry_after_secs: u64 },
    #[error("the model's reply was not understood: {0}")]
    Reply(String),
    #[error("the model did not call a tool: {0}")]
    NoToolCall(String),
    #[error("the model's tool arguments were not valid JSON: {0}")]
    BadArguments(String),
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

pub struct Reply {
    pub call: ToolCall,
    pub completion_tokens: Option<u64>,
    pub thinking: String,
    pub served_model: Option<String>,
    pub swap_ms: Option<u64>,
}

pub struct ChatClient<'a> {
    pub http: &'a reqwest::Client,
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub profile: Option<&'a ModelProfile>,
    pub sink: Option<&'a dyn AttemptSink>,
    pub no_swap: bool,
    pub on_loading: Option<&'a (dyn Fn() + Send + Sync)>,
}

pub struct Request<'a> {
    pub system: &'a str,
    pub user_text: &'a str,
    pub screenshot: Option<&'a str>,
    pub audio: Option<&'a str>,
    pub tools: &'a Value,
    pub think: bool,
    pub max_tokens: Option<u32>,
    pub data_class: Option<crate::life::DataClass>,
    pub background: bool,
}

impl<'a> Default for Request<'a> {
    fn default() -> Self {
        Self {
            system: "",
            user_text: "",
            screenshot: None,
            audio: None,
            tools: &Value::Null,
            think: false,
            max_tokens: None,
            data_class: None,
            background: false,
        }
    }
}

#[cfg(test)]
impl<'a> Request<'a> {
    pub fn simple(user_text: &'a str) -> Self {
        Self {
            user_text,
            ..Default::default()
        }
    }

    pub fn with_data_class(mut self, class: crate::life::DataClass) -> Self {
        self.data_class = Some(class);
        self
    }
}

pub fn is_retryable(error: &ModelError) -> bool {
    match error {
        ModelError::Request(_) => true,
        ModelError::Status { status, .. } => matches!(status, 429 | 502 | 503 | 504),
        ModelError::Loading { .. } => true,
        ModelError::SwapRefused | ModelError::Deferred { .. } => false,
        ModelError::Reply(_) | ModelError::NoToolCall(_) | ModelError::BadArguments(_) => false,
    }
}

pub fn is_loading(error: &ModelError) -> Option<Option<u64>> {
    match error {
        ModelError::Loading { estimated_ms } => Some(*estimated_ms),
        _ => None,
    }
}

pub fn backoff(attempt: u32) -> Duration {
    let base = RETRY_WAITS[(attempt.max(1) as usize - 1).min(RETRY_WAITS.len() - 1)];
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    base + base.mul_f64(f64::from(nanos % 1001) / 1000.0 * 0.2)
}

fn snippet(text: &str) -> String {
    text.chars().take(300).collect()
}

pub(crate) fn chat_body(model: &str, profile: &ModelProfile, request: &Request<'_>) -> Value {
    let max_output_tokens = request.max_tokens.unwrap_or(if request.think {
        profile.max_tokens.agent_thinking
    } else {
        profile.max_tokens.agent
    });
    let tool_choice = match profile.tool_choice_required {
        TriState::Yes => "required",
        TriState::No | TriState::Unknown => "auto",
    };
    let content = if request.screenshot.is_none() && request.audio.is_none() {
        json!(request.user_text)
    } else {
        let mut parts = Vec::new();
        if let Some(audio) = request.audio {
            parts.push(
                json!({ "type": "input_audio", "input_audio": { "data": audio, "format": "wav" } }),
            );
        }
        if let Some(screenshot) = request.screenshot {
            parts.push(json!({ "type": "image_url", "image_url": { "url": screenshot } }));
        }
        parts.push(json!({ "type": "text", "text": request.user_text }));
        json!(parts)
    };
    let mut body = json!({
        "model": model,
        "temperature": 0,
        "max_tokens": max_output_tokens,
        "messages": [
            { "role": "system", "content": request.system },
            { "role": "user", "content": content },
        ],
        "tools": request.tools,
        "tool_choice": tool_choice,
    });
    match &profile.thinking {
        ThinkingSwitch::None => {}
        ThinkingSwitch::Styles(styles) => {
            for style in styles {
                match style {
                    ThinkingStyle::ThinkingType => {
                        body["thinking"] =
                            json!({ "type": if request.think { "enabled" } else { "disabled" } });
                    }
                    ThinkingStyle::ChatTemplateKwargs => {
                        body["chat_template_kwargs"] = json!({ "enable_thinking": request.think });
                    }
                    ThinkingStyle::BudgetTokens => {
                        if request.think {
                            body["thinking"] = json!({ "type": "enabled", "budget_tokens": 2048 });
                        } else {
                            body["thinking"] = json!({ "type": "disabled" });
                        }
                    }
                }
            }
        }
    }
    if request.think {
        body["stream"] = json!(true);
    }
    body
}

struct Exchange {
    result: Result<Reply, ModelError>,
    status: Option<u16>,
    response: Value,
    served_model: Option<String>,
    locality: Option<String>,
    swap_ms: Option<u64>,
}

impl ChatClient<'_> {
    pub async fn call_retrying(
        &self,
        request: &Request<'_>,
        on_thinking: &mut (dyn FnMut(&str) + Send),
    ) -> Result<Reply, ModelError> {
        self.call_retrying_with_schema(request, on_thinking, None)
            .await
    }

    pub async fn call_json_retrying(
        &self,
        request: &Request<'_>,
        schema: &Value,
    ) -> Result<Value, ModelError> {
        self.call_json_reply_retrying(request, schema)
            .await
            .map(|reply| reply.call.arguments)
    }

    pub async fn call_json_reply_retrying(
        &self,
        request: &Request<'_>,
        schema: &Value,
    ) -> Result<Reply, ModelError> {
        self.call_retrying_with_schema(request, &mut |_| {}, Some(schema))
            .await
    }

    pub async fn call_json_thinking_retrying(
        &self,
        request: &Request<'_>,
        schema: &Value,
        on_thinking: &mut (dyn FnMut(&str) + Send),
    ) -> Result<Reply, ModelError> {
        self.call_retrying_with_schema(request, on_thinking, Some(schema))
            .await
    }

    pub async fn call_json(
        &self,
        request: &Request<'_>,
        schema: &Value,
    ) -> Result<Value, ModelError> {
        let (result, report) = self
            .attempt_with_schema(request, &mut |_| {}, 1, Duration::ZERO, Some(schema))
            .await;
        crate::model::swap::record_call_activity();
        if let (Some(sink), Some(report)) = (self.sink, report) {
            sink.record(report);
        }
        result.map(|reply| reply.call.arguments)
    }

    async fn call_retrying_with_schema(
        &self,
        request: &Request<'_>,
        on_thinking: &mut (dyn FnMut(&str) + Send),
        schema: Option<&Value>,
    ) -> Result<Reply, ModelError> {
        let mut attempt = 1;
        let mut wait_before = Duration::ZERO;
        let max_loading_attempts = 15;
        let mut loading_attempts = 0;
        loop {
            let (result, mut report) = self
                .attempt_with_schema(request, on_thinking, attempt, wait_before, schema)
                .await;
            match result {
                Err(ref error)
                    if is_loading(error).is_some() && loading_attempts < max_loading_attempts =>
                {
                    loading_attempts += 1;
                    if let Some(cb) = self.on_loading {
                        cb();
                    }
                    let estimated_ms = is_loading(error).unwrap();
                    let wait = estimated_ms
                        .map(Duration::from_millis)
                        .unwrap_or(Duration::from_millis(1500))
                        .clamp(Duration::from_millis(500), Duration::from_secs(15));
                    log::info!(
                        "model is loading/swapping (attempt {loading_attempts}/{max_loading_attempts}), waiting {} ms",
                        wait.as_millis()
                    );
                    if let (Some(sink), Some(report)) = (self.sink, report.as_mut()) {
                        report.outcome = "loading";
                        sink.record(report.clone());
                    }
                    tokio::time::sleep(wait).await;
                    wait_before = wait;
                }
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable(&error) => {
                    let wait = backoff(attempt);
                    log::warn!(
                        "model call failed (attempt {attempt} of {MAX_ATTEMPTS}): {error}; trying again in {} ms",
                        wait.as_millis()
                    );
                    if let (Some(sink), Some(report)) = (self.sink, report.as_mut()) {
                        report.outcome = "retried";
                        sink.record(report.clone());
                    }
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                    wait_before = wait;
                }
                other => {
                    crate::model::swap::record_call_activity();
                    if let (Some(sink), Some(report)) = (self.sink, report) {
                        sink.record(report);
                    }
                    return other;
                }
            }
        }
    }

    #[cfg(test)]
    pub async fn call(
        &self,
        request: &Request<'_>,
        on_thinking: &mut (dyn FnMut(&str) + Send),
    ) -> Result<Reply, ModelError> {
        let (result, report) = self.attempt(request, on_thinking, 1, Duration::ZERO).await;
        crate::model::swap::record_call_activity();
        if let (Some(sink), Some(report)) = (self.sink, report) {
            sink.record(report);
        }
        result
    }

    #[cfg(test)]
    async fn attempt(
        &self,
        request: &Request<'_>,
        on_thinking: &mut (dyn FnMut(&str) + Send),
        attempt: u32,
        wait_before: Duration,
    ) -> (Result<Reply, ModelError>, Option<Attempt>) {
        self.attempt_with_schema(request, on_thinking, attempt, wait_before, None)
            .await
    }

    async fn attempt_with_schema(
        &self,
        request: &Request<'_>,
        on_thinking: &mut (dyn FnMut(&str) + Send),
        attempt: u32,
        wait_before: Duration,
        schema: Option<&Value>,
    ) -> (Result<Reply, ModelError>, Option<Attempt>) {
        let default_profile;
        let profile = match self.profile {
            Some(p) => p,
            None => {
                default_profile = crate::model::profile::resolve_profile(self.model, None, None);
                &default_profile
            }
        };

        if let Some(dc) = request.data_class {
            let locality = self.profile.map(|p| p.locality.as_str());
            match crate::life::policy::may_send_to_route(Some(dc), self.base_url, locality, true) {
                crate::life::policy::PolicyDecision::Allow => {}
                crate::life::policy::PolicyDecision::Deny(reason) => {
                    log::warn!("Rejected request before network send: data class '{dc}': {reason}");
                    return (
                        Err(ModelError::Reply(format!(
                            "Local-only policy violation: data class '{dc}' cannot be sent: {reason}"
                        ))),
                        None,
                    );
                }
            }
        }

        let body = match schema {
            Some(schema) => json_body(self.model, profile, request, schema),
            None => chat_body(self.model, profile, request),
        };
        let started = std::time::Instant::now();
        let mut guard = Guard {
            sink: self.sink,
            started,
            attempt: self.sink.map(|_| Attempt {
                attempt,
                wait_before_ms: wait_before.as_millis() as u64,
                started_utc: crate::trace::now_utc(),
                duration_ms: 0,
                status: None,
                outcome: "cancelled",
                error: None,
                request: body.clone(),
                response: Value::Null,
                model: self.model.to_string(),
                prompt_tokens: None,
                completion_tokens: None,
                locality: None,
                served_model: None,
                swap_ms: None,
            }),
        };
        let exchange = self.exchange(request, &body, on_thinking).await;
        let report = guard.finish().map(|mut report| {
            report.duration_ms = started.elapsed().as_millis() as u64;
            report.status = exchange.status;
            report.response = exchange.response.clone();
            report.prompt_tokens = exchange.response["usage"]["prompt_tokens"].as_u64();
            report.completion_tokens = exchange.response["usage"]["completion_tokens"].as_u64();
            report.served_model = exchange.served_model.clone().or_else(|| {
                exchange
                    .response
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
            });
            report.swap_ms = exchange.swap_ms;
            report.locality = exchange.locality.clone();
            if let Some(ref served) = report.served_model
                && served != self.model
            {
                log::warn!(
                    "served model '{served}' differs from requested model '{}'",
                    self.model
                );
            }
            match &exchange.result {
                Ok(_) => report.outcome = "ok",
                Err(ModelError::Loading { .. }) => report.outcome = "loading",
                Err(ModelError::SwapRefused) => {
                    report.outcome = "skipped";
                    report.error = Some("swap_refused".into());
                }
                Err(ModelError::Deferred { .. }) => report.outcome = "deferred",
                Err(error) => {
                    let text = error.to_string();
                    report.outcome = if text.contains("timed out") {
                        "timeout"
                    } else {
                        "failed"
                    };
                    report.error = Some(text);
                }
            }
            report
        });
        (exchange.result, report)
    }

    async fn exchange(
        &self,
        request: &Request<'_>,
        body: &Value,
        on_thinking: &mut (dyn FnMut(&str) + Send),
    ) -> Exchange {
        let sensitive_http;
        let transport = if request.data_class.is_some_and(|dc| dc.is_sensitive()) {
            sensitive_http = match reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
            {
                Ok(client) => client,
                Err(error) => {
                    return Exchange {
                        result: Err(ModelError::Request(error.to_string())),
                        status: None,
                        response: Value::Null,
                        locality: None,
                        served_model: None,
                        swap_ms: None,
                    };
                }
            };
            &sensitive_http
        } else {
            self.http
        };
        let mut http = transport
            .post(format!(
                "{}/v1/chat/completions",
                self.base_url.trim_end_matches('/')
            ))
            .timeout(REQUEST_TIMEOUT)
            .json(body);
        if let Some(key) = self.api_key {
            http = http.bearer_auth(key);
        }
        if self.no_swap {
            http = http.header("x-no-swap", "true");
        }
        if request.background {
            http = http.header("x-rune-priority", "background");
        }
        let mut response = match http.send().await {
            Ok(response) => response,
            Err(err) => {
                return Exchange {
                    result: Err(ModelError::Request(err.to_string())),
                    status: None,
                    response: Value::Null,
                    locality: None,
                    served_model: None,
                    swap_ms: None,
                };
            }
        };
        let headers = response.headers().clone();
        if request.data_class.is_some_and(|dc| dc.is_sensitive())
            && crate::life::policy::response_is_cloud(&headers)
        {
            return Exchange {
                result: Err(ModelError::Reply(
                    "Local-only policy violation: response reported cloud locality; discarded"
                        .into(),
                )),
                status: Some(response.status().as_u16()),
                response: Value::Null,
                locality: Some("cloud".into()),
                served_model: None,
                swap_ms: None,
            };
        }
        let status = response.status();
        let code = Some(status.as_u16());
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let body_val = serde_json::from_str::<Value>(&text).ok();
            let signal = crate::model::swap::detect_swap_signal(
                &headers,
                status.as_u16(),
                body_val.as_ref(),
            );
            let err = if body_val
                .as_ref()
                .is_some_and(|v| crate::model::swap::is_swap_refused(status.as_u16(), v))
            {
                ModelError::SwapRefused
            } else if status.as_u16() == 429 && request.background {
                ModelError::Deferred {
                    retry_after_secs: headers
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1),
                }
            } else {
                match &signal {
                    crate::model::swap::SwapSignal::Loading { estimated_ms } => {
                        ModelError::Loading {
                            estimated_ms: *estimated_ms,
                        }
                    }
                    _ => ModelError::Status {
                        status: status.as_u16(),
                        body: snippet(&text),
                    },
                }
            };
            return Exchange {
                response: Value::String(text.chars().take(4_000).collect()),
                status: code,
                result: Err(err),
                locality: None,
                served_model: None,
                swap_ms: None,
            };
        }

        let header_signal = crate::model::swap::detect_swap_signal(&headers, status.as_u16(), None);
        if matches!(
            header_signal,
            crate::model::swap::SwapSignal::Loading { .. }
        ) && let Some(cb) = self.on_loading
        {
            cb();
        }
        let header_swap_ms = headers
            .get("x-model-load-duration-ms")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .or(match header_signal {
                crate::model::swap::SwapSignal::Swapped { swap_ms } => swap_ms,
                _ => None,
            });

        if body["stream"] != true {
            let text = match response.text().await {
                Ok(text) => text,
                Err(err) => {
                    return Exchange {
                        result: Err(ModelError::Request(err.to_string())),
                        status: code,
                        response: Value::Null,
                        locality: None,
                        served_model: None,
                        swap_ms: header_swap_ms,
                    };
                }
            };
            let value = match serde_json::from_str::<Value>(&text) {
                Ok(val) => val,
                Err(err) => {
                    return Exchange {
                        result: Err(ModelError::Reply(format!("{err}: {}", snippet(&text)))),
                        status: code,
                        response: Value::String(text.chars().take(4_000).collect()),
                        locality: None,
                        served_model: None,
                        swap_ms: header_swap_ms,
                    };
                }
            };
            let body_signal =
                crate::model::swap::detect_swap_signal(&headers, status.as_u16(), Some(&value));
            let swap_ms = header_swap_ms.or_else(|| match body_signal {
                crate::model::swap::SwapSignal::Swapped { swap_ms } => swap_ms,
                _ => value.get("swap_ms").and_then(|v| v.as_u64()),
            });
            let served_model = headers
                .get("x-model-served")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
                .or_else(|| {
                    value
                        .get("model")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
                });
            let mut result = if body.get("response_format").is_some() {
                parse_json_reply(&value)
            } else {
                parse_reply(&value)
            };
            if let Ok(ref mut reply) = result {
                if served_model.is_some() {
                    reply.served_model = served_model.clone();
                }
                if reply.swap_ms.is_none() {
                    reply.swap_ms = swap_ms;
                }
            }
            return Exchange {
                result,
                status: code,
                response: value,
                locality: headers
                    .get("x-model-locality")
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned),
                served_model,
                swap_ms,
            };
        }

        let mut stream = Stream::default();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    if stream.feed(&chunk) {
                        on_thinking(&stream.thinking);
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    return Exchange {
                        result: Err(ModelError::Request(err.to_string())),
                        status: code,
                        response: stream.summary(),
                        locality: headers
                            .get("x-model-locality")
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_owned),
                        served_model: stream.model.clone(),
                        swap_ms: header_swap_ms,
                    };
                }
            }
        }
        let summary = stream.summary();
        let served_model = headers
            .get("x-model-served")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .or_else(|| stream.model.clone());
        let mut result = if body.get("response_format").is_some() {
            stream.finish_json(header_swap_ms)
        } else {
            stream.finish(header_swap_ms)
        };
        if let Ok(reply) = &mut result {
            reply.served_model = served_model.clone();
        }
        Exchange {
            result,
            status: code,
            response: summary,
            locality: headers
                .get("x-model-locality")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned),
            served_model,
            swap_ms: header_swap_ms,
        }
    }
}

pub(crate) fn json_body(
    model: &str,
    profile: &ModelProfile,
    request: &Request<'_>,
    schema: &Value,
) -> Value {
    let mut body = chat_body(model, profile, request);
    let fields = body.as_object_mut().expect("chat body is an object");
    fields.remove("tools");
    fields.remove("tool_choice");
    fields.insert("stream".into(), json!(request.think));
    fields.insert(
        "response_format".into(),
        json!({
            "type": "json_schema",
            "json_schema": {"name": "structured_response", "strict": true, "schema": schema}
        }),
    );
    body
}

fn parse_json_reply(value: &Value) -> Result<Reply, ModelError> {
    let choice = &value["choices"][0];
    if choice["finish_reason"] == "length" {
        return Err(ModelError::Reply(
            "The structured response exceeded the model output limit; retry with a smaller request"
                .into(),
        ));
    }
    let message = &choice["message"];
    if message["tool_calls"]
        .as_array()
        .is_some_and(|calls| !calls.is_empty())
    {
        return Err(ModelError::Reply(
            "Expected a JSON response, but the model returned tool calls".into(),
        ));
    }
    let arguments: Value = serde_json::from_str(message["content"].as_str().unwrap_or(""))
        .map_err(|_| {
            ModelError::Reply(
                "The model did not return a complete JSON response; retry the request".into(),
            )
        })?;
    if !arguments.is_object() {
        return Err(ModelError::Reply(
            "The JSON response must be an object".into(),
        ));
    }
    Ok(Reply {
        call: ToolCall {
            name: String::new(),
            arguments,
        },
        completion_tokens: value["usage"]["completion_tokens"].as_u64(),
        thinking: reasoning_of(message).to_string(),
        served_model: value["model"].as_str().map(str::to_owned),
        swap_ms: value["swap_ms"].as_u64(),
    })
}

fn reasoning_of(message: &Value) -> &str {
    ["reasoning", "reasoning_content"]
        .iter()
        .filter_map(|key| message[*key].as_str())
        .find(|text| !text.is_empty())
        .unwrap_or("")
}

pub fn parse_reply(value: &Value) -> Result<Reply, ModelError> {
    let message = &value["choices"][0]["message"];
    let Some(call) = message["tool_calls"].get(0) else {
        return Err(ModelError::NoToolCall(snippet(
            message["content"].as_str().unwrap_or(""),
        )));
    };
    let name = call["function"]["name"].as_str().unwrap_or("").to_string();
    let served_model = value
        .get("model")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let swap_ms = value.get("swap_ms").and_then(|v| v.as_u64());
    Ok(Reply {
        call: ToolCall {
            name,
            arguments: parse_arguments(&call["function"]["arguments"])?,
        },
        completion_tokens: value["usage"]["completion_tokens"].as_u64(),
        thinking: reasoning_of(message).to_string(),
        served_model,
        swap_ms,
    })
}

fn parse_arguments(arguments: &Value) -> Result<Value, ModelError> {
    match arguments {
        Value::String(text) => match serde_json::from_str(text) {
            Ok(value) => Ok(value),
            Err(err) => {
                repair_arguments(text).ok_or_else(|| ModelError::BadArguments(err.to_string()))
            }
        },
        other => Ok(other.clone()),
    }
}

pub(crate) fn repair_arguments(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let (mut depth, mut in_string, mut escaped) = (0_u32, false, false);
    for (offset, c) in text[start..].char_indices() {
        if in_string {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let object = &text[start..start + offset + 1];
                    return serde_json::from_str(object).ok().filter(Value::is_object);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Default)]
struct Stream {
    pending: Vec<u8>,
    thinking: String,
    content: String,
    calls: BTreeMap<u64, (String, String)>,
    completion_tokens: Option<u64>,
    model: Option<String>,
    finish_reason: Option<String>,
}

impl Stream {
    fn summary(&self) -> Value {
        json!({
            "streamed": true,
            "model": self.model,
            "reasoning": self.thinking,
            "content": self.content,
            "tool_calls": self.calls.values().map(|(name, arguments)| json!({ "name": name, "arguments": arguments })).collect::<Vec<_>>(),
            "usage": { "completion_tokens": self.completion_tokens },
        })
    }

    fn feed(&mut self, bytes: &[u8]) -> bool {
        self.pending.extend_from_slice(bytes);
        let mut grew = false;
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            grew |= self.line(line.trim());
        }
        grew
    }

    fn line(&mut self, line: &str) -> bool {
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            return false;
        };
        if data == "[DONE]" {
            return false;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else {
            return false;
        };
        if let Some(tokens) = event["usage"]["completion_tokens"]
            .as_u64()
            .filter(|tokens| *tokens > 0)
        {
            self.completion_tokens = Some(tokens);
        }
        if self.model.is_none() {
            self.model = event
                .get("model")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
        }
        if let Some(reason) = event["choices"][0]["finish_reason"].as_str() {
            self.finish_reason = Some(reason.to_string());
        }
        let delta = &event["choices"][0]["delta"];
        for call in delta["tool_calls"].as_array().into_iter().flatten() {
            let entry = self
                .calls
                .entry(call["index"].as_u64().unwrap_or(0))
                .or_default();
            if let Some(name) = call["function"]["name"]
                .as_str()
                .filter(|name| !name.is_empty())
            {
                entry.0 = name.to_string();
            }
            match &call["function"]["arguments"] {
                Value::String(part) => entry.1.push_str(part),
                Value::Object(_) => entry.1 = call["function"]["arguments"].to_string(),
                _ => {}
            }
        }
        if let Some(text) = delta["content"].as_str() {
            self.content.push_str(text);
        }
        match Some(reasoning_of(delta)).filter(|text| !text.is_empty()) {
            Some(text) => {
                self.thinking.push_str(text);
                true
            }
            None => false,
        }
    }

    fn finish_json(self, swap_ms: Option<u64>) -> Result<Reply, ModelError> {
        if !self.calls.is_empty() {
            return Err(ModelError::Reply(
                "Expected a JSON response, but the model returned tool calls".into(),
            ));
        }
        parse_json_reply(&json!({
            "model": self.model, "swap_ms": swap_ms,
            "usage": {"completion_tokens": self.completion_tokens},
            "choices": [{"finish_reason": self.finish_reason, "message": {
                "content": self.content, "reasoning": self.thinking
            }}]
        }))
    }

    fn finish(self, swap_ms: Option<u64>) -> Result<Reply, ModelError> {
        let Some((name, arguments)) = self.calls.into_values().next() else {
            return Err(ModelError::NoToolCall(snippet(&self.content)));
        };
        let arguments = parse_arguments(&Value::String(arguments))?;
        Ok(Reply {
            call: ToolCall { name, arguments },
            completion_tokens: self.completion_tokens,
            thinking: self.thinking,
            served_model: self.model,
            swap_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn structured_json_stream_preserves_thinking_and_model_identity() {
        let events = [
            json!({"model":"served", "choices":[{"delta":{"reasoning_content":"First "}}]}),
            json!({"choices":[{"delta":{"reasoning_content":"plan."}}]}),
            json!({"choices":[{"delta":{"content":"{\"goals\":"}}]}),
            json!({"choices":[{"delta":{"content":"[\"Open\"]}"}}]}),
            json!({"choices":[{"delta":{},"finish_reason":"stop"}], "usage":{"completion_tokens":12}}),
        ];
        let response = events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect::<String>()
            + "data: [DONE]\n\n";
        let (url, _) = serve(vec![("200 OK", response)]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = crate::model::attempts::Collect::default();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "requested",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: None,
        };
        let tools = Value::Null;
        let request = Request {
            think: true,
            ..a_click_request(&tools)
        };
        let mut thinking = String::new();
        let reply = client
            .call_json_thinking_retrying(&request, &json!({"type":"object"}), &mut |text| {
                thinking = text.to_string()
            })
            .await
            .unwrap();
        assert_eq!(reply.call.arguments, json!({"goals":["Open"]}));
        assert_eq!(reply.thinking, "First plan.");
        assert_eq!(thinking, "First plan.");
        assert_eq!(reply.served_model.as_deref(), Some("served"));
        assert_eq!(reply.completion_tokens, Some(12));
        let attempts = sink.0.lock().unwrap();
        assert_eq!(attempts[0].request["stream"], true);
        assert!(attempts[0].request.get("tools").is_none());
        assert!(attempts[0].request.get("tool_choice").is_none());
    }

    #[test]
    fn json_stream_rejects_tool_calls_and_truncated_results() {
        let truncated = Stream {
            content: "{}".into(),
            finish_reason: Some("length".into()),
            ..Default::default()
        };
        assert!(truncated.finish_json(None).is_err());
        let mut tools = Stream {
            content: "{}".into(),
            ..Default::default()
        };
        tools.calls.insert(0, ("click".into(), "{}".into()));
        assert!(tools.finish_json(None).is_err());
    }

    #[tokio::test]
    async fn structured_planning_sends_schema_without_tools_and_reads_content() {
        let plan = json!({"depends_on_requests": [], "goals": []});
        let response = json!({"choices": [{"finish_reason": "stop", "message": {"content": plan.to_string()}}]});
        let (url, _) = serve(vec![("200 OK", response.to_string())]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = crate::model::attempts::Collect::default();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "test",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: None,
        };
        let schema = json!({"type": "object", "properties": {}});
        let tools = json!([{"type": "function", "function": {"name": "must_not_be_sent"}}]);
        let result = client
            .call_json_retrying(&a_click_request(&tools), &schema)
            .await
            .unwrap();
        assert_eq!(result, plan);
        let attempts = sink.0.lock().unwrap();
        let body = &attempts[0].request;
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
        assert_eq!(body["stream"], false);
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["schema"], schema);
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    }

    #[test]
    fn structured_plans_reject_wrappers_truncation_and_tool_calls() {
        for content in [
            "<tool_call>{}</tool_call>",
            "```json\n{}\n```",
            "{",
            "[]",
            "",
        ] {
            let value = json!({"choices": [{"message": {"content": content}}]});
            assert!(parse_json_reply(&value).is_err());
        }
        for value in [
            json!({"choices": [{"finish_reason": "length", "message": {"content": "{}"}}]}),
            json!({"choices": [{"message": {"content": "{}", "tool_calls": [{"function": {"name": "plan_task_graph"}}]}}]}),
        ] {
            assert!(parse_json_reply(&value).is_err());
        }
    }

    #[test]
    fn arguments_may_be_a_string_or_an_object() {
        let as_string = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{\"x\": 1, \"y\": 2}" } }] } }] });
        let reply = parse_reply(&as_string).unwrap();
        assert_eq!(reply.call.name, "click");
        assert_eq!(reply.call.arguments["x"], 1);

        let as_object = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "done", "arguments": { "summary": "ok" } } }] } }] });
        assert_eq!(
            parse_reply(&as_object).unwrap().call.arguments["summary"],
            "ok"
        );
    }

    #[test]
    fn a_reply_without_a_tool_call_is_an_error_with_the_text() {
        let value =
            json!({ "choices": [{ "message": { "content": "I will click the button now" } }] });
        let error = parse_reply(&value).err().unwrap();
        assert!(matches!(error, ModelError::NoToolCall(text) if text.contains("click the button")));
    }

    #[test]
    fn broken_argument_json_is_an_error_the_agent_can_answer() {
        let value = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{x:" } }] } }] });
        assert!(matches!(
            parse_reply(&value),
            Err(ModelError::BadArguments(_))
        ));
    }

    fn arguments_of(text: &str) -> Result<Value, ModelError> {
        parse_arguments(&Value::String(text.to_string()))
    }

    #[test]
    fn junk_after_the_closing_brace_is_dropped_and_nothing_inside_is_touched() {
        assert_eq!(
            arguments_of(r#"{"x": 1, "y": 2}"#).unwrap(),
            json!({ "x": 1, "y": 2 })
        );
        assert_eq!(
            arguments_of(r#"{"name":"calc","risk":"low"}"}"#).unwrap(),
            json!({ "name": "calc", "risk": "low" })
        );
        assert_eq!(
            arguments_of(r#"{"text":"a } b \" {"} trailing"#).unwrap(),
            json!({ "text": "a } b \" {" }),
            "braces inside strings do not count"
        );
        assert_eq!(
            arguments_of("Sure! {\"x\": 1} hope that helps").unwrap(),
            json!({ "x": 1 }),
            "prose around it is dropped"
        );
        assert_eq!(
            arguments_of(r#"{"a":{"b":2}} {"a":3}"#).unwrap(),
            json!({ "a": { "b": 2 } }),
            "the first object wins"
        );
    }

    #[test]
    fn what_cannot_be_repaired_stays_an_error_and_values_are_never_guessed() {
        for broken in [
            "{x:",
            r#"{"a": 1"#,
            r#"{"a": }"#,
            "no braces at all",
            r#"{"name":"calc\",reason:","#,
            "",
        ] {
            assert!(
                matches!(arguments_of(broken), Err(ModelError::BadArguments(_))),
                "{broken:?}"
            );
        }
    }

    fn event(delta: Value) -> String {
        format!(
            "data: {}\n\n",
            json!({ "choices": [{ "index": 0, "delta": delta }], "usage": { "completion_tokens": 0 } })
        )
    }

    #[test]
    fn a_stream_collects_thinking_and_the_tool_call() {
        let mut stream = Stream::default();
        let mut text = String::new();
        text.push_str(&event(json!({ "content": "", "role": "assistant" })));
        text.push_str(&event(json!({ "reasoning_content": "The user " })));
        text.push_str(&event(json!({ "reasoning_content": "wants Notepad" })));
        text.push_str(&event(json!({ "tool_calls": [{ "index": 0, "id": "a", "function": { "name": "set_plan", "arguments": "{\"goals\":[\"Open\"," } }] })));
        text.push_str(&event(
            json!({ "tool_calls": [{ "index": 0, "function": { "arguments": "\"Type\"]}" } }] }),
        ));
        text.push_str("data: [DONE]\n\n");
        let bytes = text.as_bytes();
        let mut grew = 0;
        for piece in bytes.chunks(37) {
            if stream.feed(piece) {
                grew += 1;
            }
        }
        assert!(grew >= 1);
        let reply = stream.finish(None).unwrap();
        assert_eq!(reply.thinking, "The user wants Notepad");
        assert_eq!(reply.call.name, "set_plan");
        assert_eq!(reply.call.arguments["goals"], json!(["Open", "Type"]));
    }

    #[test]
    fn thinking_is_read_from_either_field_name() {
        assert_eq!(reasoning_of(&json!({ "reasoning": "a" })), "a");
        assert_eq!(reasoning_of(&json!({ "reasoning_content": "b" })), "b");
        assert_eq!(reasoning_of(&json!({ "content": "c" })), "");
    }

    #[test]
    fn a_stream_without_a_tool_call_reports_what_the_model_said() {
        let mut stream = Stream::default();
        stream.feed(event(json!({ "content": "I would click it" })).as_bytes());
        assert!(
            matches!(stream.finish(None), Err(ModelError::NoToolCall(text)) if text.contains("click it"))
        );
    }

    #[test]
    fn multibyte_text_split_across_chunks_survives() {
        let mut stream = Stream::default();
        let line = event(json!({ "reasoning_content": "héllo 你好" }));
        let bytes = line.as_bytes();
        let split = bytes.iter().position(|byte| *byte == 0xC3).unwrap() + 1;
        stream.feed(&bytes[..split]);
        stream.feed(&bytes[split..]);
        assert_eq!(stream.thinking, "héllo 你好");
    }

    #[test]
    fn only_transient_failures_are_retried() {
        assert!(is_retryable(&ModelError::Request("refused".into())));
        for status in [429, 502, 503, 504] {
            assert!(
                is_retryable(&ModelError::Status {
                    status,
                    body: String::new()
                }),
                "{status}"
            );
        }
        for status in [400, 401, 404, 500] {
            assert!(
                !is_retryable(&ModelError::Status {
                    status,
                    body: String::new()
                }),
                "{status}"
            );
        }
        assert!(!is_retryable(&ModelError::Reply("bad json".into())));
        assert!(!is_retryable(&ModelError::NoToolCall(
            "I would click".into()
        )));
    }

    #[test]
    fn the_wait_grows_and_the_jitter_stays_within_20_percent() {
        for _ in 0..200 {
            let first = backoff(1);
            assert!(
                first >= Duration::from_millis(500) && first <= Duration::from_millis(600),
                "{first:?}"
            );
            let second = backoff(2);
            assert!(
                second >= Duration::from_millis(1500) && second <= Duration::from_millis(1800),
                "{second:?}"
            );
        }
        assert!(
            backoff(9) <= Duration::from_millis(1800),
            "later attempts reuse the last wait"
        );
    }

    fn serve(
        answers: Vec<(&'static str, String)>,
    ) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
            },
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let served = Arc::new(AtomicUsize::new(0));
        let counter = served.clone();
        std::thread::spawn(move || {
            for (status, body) in answers {
                let (mut stream, _) = listener.accept().unwrap();
                let mut received = Vec::new();
                let mut chunk = [0_u8; 4096];
                let (header_end, length) = loop {
                    let read = stream.read(&mut chunk).unwrap();
                    received.extend_from_slice(&chunk[..read]);
                    if let Some(end) = received.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .and_then(|value| value.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (end + 4, length);
                    }
                    assert!(read > 0, "the client hung up before sending its request");
                };
                while received.len() < header_end + length {
                    let read = stream.read(&mut chunk).unwrap();
                    assert!(read > 0);
                    received.extend_from_slice(&chunk[..read]);
                }
                counter.fetch_add(1, Ordering::SeqCst);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (url, served)
    }

    fn a_click_request(tools: &Value) -> Request<'_> {
        Request {
            system: "s",
            user_text: "u",
            screenshot: None,
            audio: None,
            tools,
            think: false,
            max_tokens: None,
            data_class: None,
            background: false,
        }
    }

    #[tokio::test]
    async fn a_503_is_retried_and_the_second_answer_is_used() {
        let ok = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{\"x\": 1}" } }] } }] });
        let (url, served) = serve(vec![
            ("503 Service Unavailable", "busy".into()),
            ("200 OK", ok.to_string()),
        ]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "m",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        let reply = client
            .call_retrying(&a_click_request(&tools), &mut |_| {})
            .await
            .expect("the second attempt succeeds");
        assert_eq!(reply.call.name, "click");
        assert_eq!(served.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn every_attempt_is_reported_with_its_wait_and_a_retry_is_marked_retried() {
        use crate::model::attempts::Collect;
        let ok = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{\"x\": 1}" } }] } }], "usage": { "prompt_tokens": 7, "completion_tokens": 3 } });
        let (url, _) = serve(vec![
            ("503 Service Unavailable", "busy".into()),
            ("200 OK", ok.to_string()),
        ]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = Collect::default();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: Some("sk-secret"),
            model: "m",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        client
            .call_retrying(&a_click_request(&tools), &mut |_| {})
            .await
            .expect("the second attempt succeeds");
        let attempts = sink.0.lock().unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            (
                attempts[0].attempt,
                attempts[0].outcome,
                attempts[0].status,
                attempts[0].wait_before_ms
            ),
            (1, "retried", Some(503), 0)
        );
        assert_eq!(
            (attempts[1].attempt, attempts[1].outcome, attempts[1].status),
            (2, "ok", Some(200))
        );
        assert!(
            attempts[1].wait_before_ms >= 500,
            "the backoff before the second attempt is recorded: {}",
            attempts[1].wait_before_ms
        );
        assert_eq!(attempts[1].completion_tokens, Some(3));
        assert_eq!(attempts[0].response, json!("busy"));
        assert!(
            !attempts[0].request.to_string().contains("sk-secret"),
            "the key is a header, never in a body"
        );
    }

    #[tokio::test]
    async fn a_final_failure_is_reported_as_failed_and_a_dropped_call_as_cancelled() {
        use crate::model::attempts::Collect;
        let (url, _) = serve(vec![("400 Bad Request", "no".into())]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = Collect::default();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "m",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        client
            .call_retrying(&a_click_request(&tools), &mut |_| {})
            .await
            .err()
            .expect("a 400 is final");
        assert_eq!(
            sink.0
                .lock()
                .unwrap()
                .iter()
                .map(|a| a.outcome)
                .collect::<Vec<_>>(),
            ["failed"]
        );

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let silent = format!("http://{}", listener.local_addr().unwrap());
        let sink = Collect::default();
        let client = ChatClient {
            http: &http,
            base_url: &silent,
            api_key: None,
            model: "m",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: None,
        };
        let gave_up = tokio::time::timeout(
            Duration::from_millis(200),
            client.call(&a_click_request(&tools), &mut |_| {}),
        )
        .await;
        assert!(gave_up.is_err());
        let attempts = sink.0.lock().unwrap();
        assert_eq!(
            attempts.iter().map(|a| a.outcome).collect::<Vec<_>>(),
            ["cancelled"]
        );
        assert!(attempts[0].duration_ms >= 150);
        drop(listener);
    }

    #[tokio::test]
    async fn a_400_is_not_retried() {
        let (url, served) = serve(vec![("400 Bad Request", "no".into())]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "m",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        let error = client
            .call_retrying(&a_click_request(&tools), &mut |_| {})
            .await
            .err()
            .expect("a 400 is final");
        assert!(matches!(error, ModelError::Status { status: 400, .. }));
        assert_eq!(served.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_cancelled_call_closes_its_connection_instead_of_leaving_it_open() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (closed, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut chunk = [0_u8; 4096];
            let _ = stream.read(&mut chunk);
            let outcome = loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break "closed",
                    Ok(_) => continue,
                    Err(err)
                        if matches!(
                            err.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        break "left open";
                    }
                    Err(_) => break "closed",
                }
            };
            let _ = closed.send(outcome);
        });
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "m",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        let cancelled = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            client.call(&a_click_request(&tools), &mut |_| {}),
        )
        .await;
        assert!(
            cancelled.is_err(),
            "the server never answers, so the call is still pending when it is dropped"
        );
        let outcome = tokio::task::spawn_blocking(move || {
            wait.recv_timeout(std::time::Duration::from_secs(8))
                .unwrap()
        })
        .await
        .unwrap();
        assert_eq!(
            outcome, "closed",
            "the connection must be closed, not left for the server to keep generating into"
        );
    }

    #[test]
    fn the_audio_part_is_input_audio_with_the_whole_wav_and_comes_first() {
        let tools = json!([]);
        let request = Request {
            system: "sys",
            user_text: "what now",
            screenshot: Some("data:image/jpeg;base64,AA"),
            audio: Some("UklGRg=="),
            tools: &tools,
            think: false,
            max_tokens: None,
            data_class: None,
            background: false,
        };
        let profile = ModelProfile::conservative("m");
        let body = chat_body("m", &profile, &request);
        let parts = body["messages"][1]["content"].as_array().expect("parts");
        assert_eq!(parts.len(), 3);
        assert_eq!(
            parts[0],
            json!({ "type": "input_audio", "input_audio": { "data": "UklGRg==", "format": "wav" } })
        );
        assert_eq!(
            parts[1],
            json!({ "type": "image_url", "image_url": { "url": "data:image/jpeg;base64,AA" } })
        );
        assert_eq!(parts[2], json!({ "type": "text", "text": "what now" }));
        assert!(
            !body.to_string().contains("audio_url"),
            "the old part name is gone"
        );
    }

    #[test]
    fn the_thinking_switch_is_sent_both_ways() {
        let tools = json!([]);
        let mut request = Request {
            system: "s",
            user_text: "u",
            screenshot: None,
            audio: None,
            tools: &tools,
            think: false,
            max_tokens: None,
            data_class: None,
            background: false,
        };
        let profile = ModelProfile::conservative("m");
        let off = chat_body("m", &profile, &request);
        assert_eq!(off["thinking"], json!({ "type": "disabled" }));
        assert_eq!(
            off["chat_template_kwargs"],
            json!({ "enable_thinking": false })
        );
        assert_eq!(off["max_tokens"], 512);
        assert!(off.get("stream").is_none());
        assert!(off.get("reasoning_effort").is_none());

        request.think = true;
        let on = chat_body("m", &profile, &request);
        assert_eq!(on["thinking"], json!({ "type": "enabled" }));
        assert_eq!(
            on["chat_template_kwargs"],
            json!({ "enable_thinking": true })
        );
        assert_eq!(on["stream"], true);
        assert_eq!(on["max_tokens"], 3000);
    }

    #[test]
    fn a_text_only_request_keeps_its_shape() {
        let tools = json!([{ "type": "function", "function": { "name": "click" } }]);
        let request = Request {
            system: "sys",
            user_text: "plan this",
            screenshot: None,
            audio: None,
            tools: &tools,
            think: false,
            max_tokens: None,
            data_class: None,
            background: false,
        };
        let mut profile = ModelProfile::conservative("the-model");
        profile.tool_choice_required = TriState::Yes;
        let body = chat_body("the-model", &profile, &request);
        assert_eq!(body["model"], "the-model");
        assert_eq!(
            body["messages"][0],
            json!({ "role": "system", "content": "sys" })
        );
        assert_eq!(
            body["messages"][1],
            json!({ "role": "user", "content": "plan this" })
        );
        assert_eq!(body["tools"], tools);
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["temperature"], 0);
    }

    #[test]
    fn profile_controls_thinking_style_and_tool_choice() {
        let tools = json!([]);
        let request = Request {
            system: "sys",
            user_text: "test",
            screenshot: None,
            audio: None,
            tools: &tools,
            think: true,
            max_tokens: None,
            data_class: None,
            background: false,
        };

        let mut p_none = ModelProfile::conservative("m");
        p_none.thinking = ThinkingSwitch::None;
        let b_none = chat_body("m", &p_none, &request);
        assert!(b_none.get("thinking").is_none());
        assert!(b_none.get("chat_template_kwargs").is_none());

        let mut p_type = ModelProfile::conservative("m");
        p_type.thinking = ThinkingSwitch::Styles(vec![ThinkingStyle::ThinkingType]);
        let b_type = chat_body("m", &p_type, &request);
        assert_eq!(b_type["thinking"], json!({ "type": "enabled" }));
        assert!(b_type.get("chat_template_kwargs").is_none());

        let mut p_auto = ModelProfile::conservative("m");
        p_auto.tool_choice_required = TriState::Unknown;
        let b_auto = chat_body("m", &p_auto, &request);
        assert_eq!(b_auto["tool_choice"], "auto");

        let mut p_req = ModelProfile::conservative("m");
        p_req.tool_choice_required = TriState::Yes;
        let b_req = chat_body("m", &p_req, &request);
        assert_eq!(b_req["tool_choice"], "required");
    }

    type ServerAnswer = (&'static str, Vec<(&'static str, &'static str)>, String);

    fn serve_with_headers(
        answers: Vec<ServerAnswer>,
    ) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            sync::{
                Arc,
                atomic::{AtomicUsize, Ordering},
            },
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let served = Arc::new(AtomicUsize::new(0));
        let counter = served.clone();
        std::thread::spawn(move || {
            for (status, headers, body) in answers {
                let (mut stream, _) = listener.accept().unwrap();
                let mut received = Vec::new();
                let mut chunk = [0_u8; 4096];
                let (header_end, length) = loop {
                    let read = stream.read(&mut chunk).unwrap();
                    received.extend_from_slice(&chunk[..read]);
                    if let Some(end) = received.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .and_then(|value| value.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (end + 4, length);
                    }
                    assert!(read > 0, "the client hung up before sending its request");
                };
                while received.len() < header_end + length {
                    let read = stream.read(&mut chunk).unwrap();
                    assert!(read > 0);
                    received.extend_from_slice(&chunk[..read]);
                }
                counter.fetch_add(1, Ordering::SeqCst);
                let mut header_str = String::new();
                for (k, v) in headers {
                    header_str.push_str(&format!("{k}: {v}\r\n"));
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{header_str}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (url, served)
    }

    #[tokio::test]
    async fn swap_signals_trigger_loading_and_record_swap_ms() {
        use crate::model::attempts::Collect;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let ok = json!({
            "model": "served-model-v2",
            "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{\"x\": 10}" } }] } }],
            "swap_ms": 3200
        });
        let loading_body = json!({
            "status": "loading",
            "load_ms_estimate": 50
        });
        let (url, served) = serve_with_headers(vec![
            (
                "503 Service Unavailable",
                vec![("x-model-loading", "50")],
                loading_body.to_string(),
            ),
            ("200 OK", vec![("x-model-swap-ms", "3200")], ok.to_string()),
        ]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = Collect::default();
        let loaded = Arc::new(AtomicBool::new(false));
        let loaded_flag = loaded.clone();
        let on_loading = move || {
            loaded_flag.store(true, Ordering::SeqCst);
        };
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "requested-model-v1",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: Some(&on_loading),
        };
        let tools = json!([]);
        let reply = client
            .call_retrying(&a_click_request(&tools), &mut |_| {})
            .await
            .expect("succeeds after loading");
        assert!(loaded.load(Ordering::SeqCst), "on_loading was called");
        assert_eq!(reply.served_model.as_deref(), Some("served-model-v2"));
        assert_eq!(reply.swap_ms, Some(3200));
        assert_eq!(served.load(Ordering::SeqCst), 2);

        let attempts = sink.0.lock().unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].outcome, "loading");
        assert_eq!(attempts[1].outcome, "ok");
        assert_eq!(attempts[1].served_model.as_deref(), Some("served-model-v2"));
        assert_eq!(attempts[1].swap_ms, Some(3200));
    }

    #[tokio::test]
    async fn server_that_refuses_swap_fails_call() {
        use std::sync::atomic::Ordering;
        let (url, served) = serve_with_headers(vec![
            (
                "503 Service Unavailable",
                vec![],
                json!({ "error": "swap_refused", "status": "refused" }).to_string(),
            ),
            (
                "503 Service Unavailable",
                vec![],
                json!({ "error": "swap_refused", "status": "refused" }).to_string(),
            ),
            (
                "503 Service Unavailable",
                vec![],
                json!({ "error": "swap_refused", "status": "refused" }).to_string(),
            ),
        ]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "model-refused",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        let result = client
            .call_retrying(&a_click_request(&tools), &mut |_| {})
            .await;
        assert!(result.is_err());
        assert_eq!(served.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn real_refusal_and_background_deferral_are_recorded_without_retry() {
        use crate::model::attempts::Collect;
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        for background in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let head = String::from_utf8_lossy(&bytes).to_lowercase();
                assert!(head.contains("x-no-swap: true"));
                assert_eq!(head.contains("x-rune-priority: background"), background);
                let (status, body) = if background {
                    (
                        "429 Too Many Requests",
                        r#"{"error":{"type":"server_busy","code":"busy"}}"#,
                    )
                } else {
                    (
                        "409 Conflict",
                        r#"{"error":{"message":"not resident","type":"swap_refused","code":"swap_refused"}}"#,
                    )
                };
                write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nRetry-After: 7\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let http = reqwest::Client::builder().no_proxy().build().unwrap();
            let sink = Collect::default();
            let client = ChatClient {
                http: &http,
                base_url: &url,
                api_key: None,
                model: "optional",
                profile: None,
                sink: Some(&sink),
                no_swap: true,
                on_loading: None,
            };
            let tools = json!([]);
            let mut request = a_click_request(&tools);
            request.background = background;
            let result = client.call_retrying(&request, &mut |_| {}).await;
            if background {
                assert!(matches!(
                    result,
                    Err(ModelError::Deferred {
                        retry_after_secs: 7
                    })
                ));
            } else {
                assert!(matches!(result, Err(ModelError::SwapRefused)));
            }
            server.join().unwrap();
            let attempts = sink.0.lock().unwrap();
            assert_eq!(attempts.len(), 1);
            assert_eq!(
                attempts[0].outcome,
                if background { "deferred" } else { "skipped" }
            );
        }
    }

    #[tokio::test]
    async fn real_attribution_headers_override_body_model_and_preserve_body() {
        let body = json!({"model":"body-model","choices":[{"message":{"tool_calls":[{"function":{"name":"click","arguments":"{}"}}]}}]});
        let (url, _) = serve_with_headers(vec![(
            "200 OK",
            vec![
                ("x-model-served", "canonical"),
                ("x-model-locality", "local"),
                ("x-model-resident", "false"),
                ("x-model-load-duration-ms", "25"),
            ],
            body.to_string(),
        )]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let sink = crate::model::attempts::Collect::default();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "role",
            profile: None,
            sink: Some(&sink),
            no_swap: false,
            on_loading: None,
        };
        let tools = json!([]);
        let reply = client
            .call(&a_click_request(&tools), &mut |_| {})
            .await
            .unwrap();
        assert_eq!(reply.served_model.as_deref(), Some("canonical"));
        assert_eq!(reply.swap_ms, Some(25));
        let calls = sink.0.lock().unwrap();
        assert_eq!(calls[0].locality.as_deref(), Some("local"));
        assert_eq!(calls[0].response["model"], "body-model");
    }

    #[test]
    fn explicit_no_thinking_suppresses_bundled_switches() {
        let caps = crate::model::capabilities::parse_model_entry(
            &json!({"id":"gemma-4-12b-it","thinking_control":"none"}),
        )
        .unwrap();
        let profile = crate::model::profile::resolve_profile("gemma-4-12b-it", None, Some(&caps));
        let tools = json!([]);
        let body = chat_body("gemma-4-12b-it", &profile, &a_click_request(&tools));
        assert!(body.get("thinking").is_none());
        assert!(body.get("chat_template_kwargs").is_none());
    }
    #[tokio::test]
    async fn sensitive_cloud_profile_is_blocked_before_network_send() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let http = reqwest::Client::new();
        let mut profile = ModelProfile::conservative("test");
        profile.locality = crate::model::capabilities::Locality::Cloud;
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "test",
            profile: Some(&profile),
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let request = Request::simple("private").with_data_class(crate::life::DataClass::Health);
        assert!(
            client
                .call(&request, &mut |_| {})
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("Local-only")
        );
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }

    #[tokio::test]
    async fn sensitive_cloud_responses_are_discarded_for_errors_and_streams() {
        for (status, think) in [
            ("200 OK", false),
            ("500 Internal Server Error", false),
            ("200 OK", true),
        ] {
            let (url, _) = serve_with_headers(vec![(
                status,
                vec![("x-model-locality", "cloud"), ("x-rune-locality", "local")],
                r#"{"private":"must never enter trace"}"#.into(),
            )]);
            let http = reqwest::Client::new();
            let sink = crate::model::attempts::Collect::default();
            let client = ChatClient {
                http: &http,
                base_url: &url,
                api_key: None,
                model: "test",
                profile: None,
                sink: Some(&sink),
                no_swap: false,
                on_loading: None,
            };
            let mut request =
                Request::simple("private").with_data_class(crate::life::DataClass::Finance);
            request.think = think;
            let error = client.call(&request, &mut |_| {}).await.err().unwrap();
            assert!(error.to_string().contains("discarded"));
            let attempts = sink.0.lock().unwrap();
            assert_eq!(attempts.len(), 1);
            assert_eq!(attempts[0].response, Value::Null);
        }
    }

    #[tokio::test]
    async fn sensitive_requests_never_follow_redirects() {
        let target = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let destination: &'static str =
            Box::leak(format!("http://{}/leak", target.local_addr().unwrap()).into_boxed_str());
        let (url, _) = serve_with_headers(vec![(
            "302 Found",
            vec![("location", destination)],
            String::new(),
        )]);
        let http = reqwest::Client::new();
        let client = ChatClient {
            http: &http,
            base_url: &url,
            api_key: None,
            model: "test",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let request = Request::simple("private").with_data_class(crate::life::DataClass::Mind);
        assert!(matches!(
            client.call(&request, &mut |_| {}).await,
            Err(ModelError::Status { status: 302, .. })
        ));
        assert!(matches!(target.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}
