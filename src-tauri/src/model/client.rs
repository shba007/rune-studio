//! The chat call to the omni model: the task (a screenshot and/or what the user just said) in,
//! one tool call out. Every model role goes through this one call.
//!
//! Two modes. By default thinking is off (`chat_template_kwargs.enable_thinking: false`): the M1.5 probe showed
//! grounding does not need it, and it only adds latency. With `think` on, the reply is streamed so
//! the model's thinking can be shown while it happens.

use std::{collections::BTreeMap, time::Duration};

use serde_json::{Value, json};

/// A turn can take half a minute on a 27B model that has to read a fresh screenshot.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_OUTPUT_TOKENS: u32 = 512;
/// Thinking uses tokens before the tool call, so it needs room or the call would be cut off.
const MAX_OUTPUT_TOKENS_THINKING: u32 = 3000;
/// `call_retrying` tries this many times in all, waiting `RETRY_WAITS` (plus up to 20% jitter) between tries.
const MAX_ATTEMPTS: u32 = 3;
const RETRY_WAITS: [Duration; 2] = [Duration::from_millis(500), Duration::from_millis(1500)];

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("could not reach the model: {0}")]
    Request(String),
    #[error("the model server answered {status}: {body}")]
    Status { status: u16, body: String },
    #[error("the model's reply was not understood: {0}")]
    Reply(String),
    #[error("the model did not call a tool: {0}")]
    NoToolCall(String),
}

/// A tool call as the model wrote it, before it is checked against the protocol.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

pub struct Reply {
    pub call: ToolCall,
    pub completion_tokens: Option<u64>,
    /// What the model thought before answering (empty when thinking was off).
    pub thinking: String,
}

pub struct ChatClient<'a> {
    pub http: &'a reqwest::Client,
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
}

/// One question for the model.
pub struct Request<'a> {
    pub system: &'a str,
    pub user_text: &'a str,
    /// The screen as a data URI. The planning call has none.
    pub screenshot: Option<&'a str>,
    /// What the user just said, as base64 WAV (16 kHz mono).
    pub audio: Option<&'a str>,
    pub tools: &'a Value,
    /// Let the model think, and stream that thinking to `on_thinking`.
    pub think: bool,
}

/// Whether trying the same request again can help: the server was not reachable or was briefly
/// overloaded. A reply we could not understand, a model that chose no tool, and every other
/// error status would come back the same.
pub fn is_retryable(error: &ModelError) -> bool {
    match error {
        ModelError::Request(_) => true,
        ModelError::Status { status, .. } => matches!(status, 429 | 502 | 503 | 504),
        ModelError::Reply(_) | ModelError::NoToolCall(_) => false,
    }
}

/// How long to wait after failed attempt number `attempt` (1-based): 500 ms, then 1500 ms, each
/// plus a random 0 to 20%.
pub fn backoff(attempt: u32) -> Duration {
    let base = RETRY_WAITS[(attempt.max(1) as usize - 1).min(RETRY_WAITS.len() - 1)];
    // No random-number crate for this: the clock's nanoseconds are random enough to spread retries.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.subsec_nanos());
    base + base.mul_f64(f64::from(nanos % 1001) / 1000.0 * 0.2)
}

fn snippet(text: &str) -> String {
    text.chars().take(300).collect()
}

impl ChatClient<'_> {
    /// `call`, tried again when the failure is transient (see `is_retryable`), at most
    /// `MAX_ATTEMPTS` times. The waits are plain sleeps inside this future, so a caller that races
    /// it against Stop (`unless_stopped`) abandons them by dropping it.
    pub async fn call_retrying(&self, request: &Request<'_>, on_thinking: &mut (dyn FnMut(&str) + Send)) -> Result<Reply, ModelError> {
        let mut attempt = 1;
        loop {
            match self.call(request, on_thinking).await {
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable(&error) => {
                    let wait = backoff(attempt);
                    log::warn!("model call failed (attempt {attempt} of {MAX_ATTEMPTS}): {error}; trying again in {} ms", wait.as_millis());
                    tokio::time::sleep(wait).await;
                    attempt += 1;
                }
                other => return other,
            }
        }
    }

    /// Asks for a tool call. `on_thinking` receives the thinking so far (all of it, each time) as
    /// it streams in; it is never called when `think` is off.
    pub async fn call(&self, request: &Request<'_>, on_thinking: &mut (dyn FnMut(&str) + Send)) -> Result<Reply, ModelError> {
        let content = if request.screenshot.is_none() && request.audio.is_none() {
            json!(request.user_text)
        } else {
            // Audio and image first, text last, like the probe: the screenshot is the expensive part.
            let mut parts = Vec::new();
            if let Some(audio) = request.audio {
                parts.push(json!({ "type": "audio_url", "audio_url": { "url": format!("data:audio/wav;base64,{audio}") } }));
            }
            if let Some(screenshot) = request.screenshot {
                parts.push(json!({ "type": "image_url", "image_url": { "url": screenshot } }));
            }
            parts.push(json!({ "type": "text", "text": request.user_text }));
            json!(parts)
        };
        let mut body = json!({
            "model": self.model,
            "temperature": 0,
            "max_tokens": if request.think { MAX_OUTPUT_TOKENS_THINKING } else { MAX_OUTPUT_TOKENS },
            "messages": [
                { "role": "system", "content": request.system },
                { "role": "user", "content": content },
            ],
            "tools": request.tools,
            "tool_choice": "required",
        });
        // The thinking switch is a chat-template flag; the server's reasoning parser keeps the
        // thinking out of `content`.
        body["chat_template_kwargs"] = json!({ "enable_thinking": request.think });
        if request.think {
            body["stream"] = json!(true);
        }

        let mut http = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/')))
            .timeout(REQUEST_TIMEOUT)
            .json(&body);
        if let Some(key) = self.api_key {
            http = http.bearer_auth(key);
        }
        let mut response = http.send().await.map_err(|err| ModelError::Request(err.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(ModelError::Status { status: status.as_u16(), body: snippet(&text) });
        }

        if !request.think {
            let text = response.text().await.map_err(|err| ModelError::Request(err.to_string()))?;
            let value: Value = serde_json::from_str(&text).map_err(|err| ModelError::Reply(format!("{err}: {}", snippet(&text))))?;
            return parse_reply(&value);
        }

        let mut stream = Stream::default();
        while let Some(chunk) = response.chunk().await.map_err(|err| ModelError::Request(err.to_string()))? {
            if stream.feed(&chunk) {
                on_thinking(&stream.thinking);
            }
        }
        stream.finish()
    }
}

/// The model's thinking in a message or stream delta: servers call the field `reasoning` (vLLM)
/// or `reasoning_content` (older ones).
fn reasoning_of(message: &Value) -> &str {
    ["reasoning", "reasoning_content"].iter().filter_map(|key| message[*key].as_str()).find(|text| !text.is_empty()).unwrap_or("")
}

/// Pulls the first tool call out of a chat completion.
pub fn parse_reply(value: &Value) -> Result<Reply, ModelError> {
    let message = &value["choices"][0]["message"];
    let Some(call) = message["tool_calls"].get(0) else {
        return Err(ModelError::NoToolCall(snippet(message["content"].as_str().unwrap_or(""))));
    };
    let name = call["function"]["name"].as_str().unwrap_or("").to_string();
    Ok(Reply {
        call: ToolCall { name, arguments: parse_arguments(&call["function"]["arguments"])? },
        completion_tokens: value["usage"]["completion_tokens"].as_u64(),
        thinking: reasoning_of(message).to_string(),
    })
}

/// Arguments arrive as a JSON string; a few servers send an object instead.
fn parse_arguments(arguments: &Value) -> Result<Value, ModelError> {
    match arguments {
        Value::String(text) => serde_json::from_str(text).map_err(|err| ModelError::Reply(format!("arguments were not JSON: {err}"))),
        other => Ok(other.clone()),
    }
}

/// A streamed reply being assembled from server-sent events.
#[derive(Default)]
struct Stream {
    /// Bytes not yet split into whole lines (a chunk can end in the middle of one).
    pending: Vec<u8>,
    thinking: String,
    content: String,
    /// Tool calls by index: name, and the arguments text so far.
    calls: BTreeMap<u64, (String, String)>,
    completion_tokens: Option<u64>,
}

impl Stream {
    /// Takes the next bytes from the server. Returns whether the thinking grew.
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
        if let Some(tokens) = event["usage"]["completion_tokens"].as_u64().filter(|tokens| *tokens > 0) {
            self.completion_tokens = Some(tokens);
        }
        let delta = &event["choices"][0]["delta"];
        for call in delta["tool_calls"].as_array().into_iter().flatten() {
            let entry = self.calls.entry(call["index"].as_u64().unwrap_or(0)).or_default();
            if let Some(name) = call["function"]["name"].as_str().filter(|name| !name.is_empty()) {
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

    fn finish(self) -> Result<Reply, ModelError> {
        let Some((name, arguments)) = self.calls.into_values().next() else {
            return Err(ModelError::NoToolCall(snippet(&self.content)));
        };
        let arguments = parse_arguments(&Value::String(arguments))?;
        Ok(Reply { call: ToolCall { name, arguments }, completion_tokens: self.completion_tokens, thinking: self.thinking })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_may_be_a_string_or_an_object() {
        let as_string = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{\"x\": 1, \"y\": 2}" } }] } }] });
        let reply = parse_reply(&as_string).unwrap();
        assert_eq!(reply.call.name, "click");
        assert_eq!(reply.call.arguments["x"], 1);

        let as_object = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "done", "arguments": { "summary": "ok" } } }] } }] });
        assert_eq!(parse_reply(&as_object).unwrap().call.arguments["summary"], "ok");
    }

    #[test]
    fn a_reply_without_a_tool_call_is_an_error_with_the_text() {
        let value = json!({ "choices": [{ "message": { "content": "I will click the button now" } }] });
        let error = parse_reply(&value).err().unwrap();
        assert!(matches!(error, ModelError::NoToolCall(text) if text.contains("click the button")));
    }

    #[test]
    fn broken_argument_json_is_an_error() {
        let value = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{x:" } }] } }] });
        assert!(matches!(parse_reply(&value), Err(ModelError::Reply(_))));
    }

    fn event(delta: Value) -> String {
        format!("data: {}\n\n", json!({ "choices": [{ "index": 0, "delta": delta }], "usage": { "completion_tokens": 0 } }))
    }

    #[test]
    fn a_stream_collects_thinking_and_the_tool_call() {
        let mut stream = Stream::default();
        let mut text = String::new();
        text.push_str(&event(json!({ "content": "", "role": "assistant" })));
        text.push_str(&event(json!({ "reasoning_content": "The user " })));
        text.push_str(&event(json!({ "reasoning_content": "wants Notepad" })));
        text.push_str(&event(json!({ "tool_calls": [{ "index": 0, "id": "a", "function": { "name": "set_plan", "arguments": "{\"goals\":[\"Open\"," } }] })));
        text.push_str(&event(json!({ "tool_calls": [{ "index": 0, "function": { "arguments": "\"Type\"]}" } }] })));
        text.push_str("data: [DONE]\n\n");
        // Feed it in awkward pieces, including a split in the middle of a line and of a word.
        let bytes = text.as_bytes();
        let mut grew = 0;
        for piece in bytes.chunks(37) {
            if stream.feed(piece) {
                grew += 1;
            }
        }
        assert!(grew >= 1);
        let reply = stream.finish().unwrap();
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
        assert!(matches!(stream.finish(), Err(ModelError::NoToolCall(text)) if text.contains("click it")));
    }

    #[test]
    fn multibyte_text_split_across_chunks_survives() {
        let mut stream = Stream::default();
        let line = event(json!({ "reasoning_content": "héllo 你好" }));
        let bytes = line.as_bytes();
        let split = bytes.iter().position(|byte| *byte == 0xC3).unwrap() + 1; // in the middle of "é"
        stream.feed(&bytes[..split]);
        stream.feed(&bytes[split..]);
        assert_eq!(stream.thinking, "héllo 你好");
    }

    #[test]
    fn only_transient_failures_are_retried() {
        assert!(is_retryable(&ModelError::Request("refused".into())));
        for status in [429, 502, 503, 504] {
            assert!(is_retryable(&ModelError::Status { status, body: String::new() }), "{status}");
        }
        for status in [400, 401, 404, 500] {
            assert!(!is_retryable(&ModelError::Status { status, body: String::new() }), "{status}");
        }
        assert!(!is_retryable(&ModelError::Reply("bad json".into())));
        assert!(!is_retryable(&ModelError::NoToolCall("I would click".into())));
    }

    #[test]
    fn the_wait_grows_and_the_jitter_stays_within_20_percent() {
        for _ in 0..200 {
            let first = backoff(1);
            assert!(first >= Duration::from_millis(500) && first <= Duration::from_millis(600), "{first:?}");
            let second = backoff(2);
            assert!(second >= Duration::from_millis(1500) && second <= Duration::from_millis(1800), "{second:?}");
        }
        assert!(backoff(9) <= Duration::from_millis(1800), "later attempts reuse the last wait");
    }

    /// A one-connection-at-a-time HTTP server on a free port. Each connection gets the next canned
    /// answer as `(status line, body)`; the request is read in full first so the client does not
    /// see a reset. Returns its base URL and how many requests it has served.
    fn serve(answers: Vec<(&'static str, String)>) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::{io::{Read, Write}, net::TcpListener, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
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
                    if let Some(end) = received.windows(4).position(|window| window == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                        let length = head.lines().find_map(|line| line.strip_prefix("content-length:")).and_then(|value| value.trim().parse::<usize>().ok()).unwrap_or(0);
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
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (url, served)
    }

    fn a_click_request(tools: &Value) -> Request<'_> {
        Request { system: "s", user_text: "u", screenshot: None, audio: None, tools, think: false }
    }

    #[tokio::test]
    async fn a_503_is_retried_and_the_second_answer_is_used() {
        let ok = json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": "click", "arguments": "{\"x\": 1}" } }] } }] });
        let (url, served) = serve(vec![("503 Service Unavailable", "busy".into()), ("200 OK", ok.to_string())]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient { http: &http, base_url: &url, api_key: None, model: "m" };
        let tools = json!([]);
        let reply = client.call_retrying(&a_click_request(&tools), &mut |_| {}).await.expect("the second attempt succeeds");
        assert_eq!(reply.call.name, "click");
        assert_eq!(served.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_400_is_not_retried() {
        let (url, served) = serve(vec![("400 Bad Request", "no".into())]);
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient { http: &http, base_url: &url, api_key: None, model: "m" };
        let tools = json!([]);
        let error = client.call_retrying(&a_click_request(&tools), &mut |_| {}).await.err().expect("a 400 is final");
        assert!(matches!(error, ModelError::Status { status: 400, .. }));
        assert_eq!(served.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
