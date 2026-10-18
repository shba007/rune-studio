//! The chat call to rune-infer: one screenshot plus the task in, one tool call out.

use std::time::Duration;

use serde_json::{Value, json};

/// A turn can take half a minute on a 27B model that has to read a fresh screenshot.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_OUTPUT_TOKENS: u32 = 512;

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
}

pub struct ChatClient<'a> {
    pub http: &'a reqwest::Client,
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
}

fn snippet(text: &str) -> String {
    text.chars().take(300).collect()
}

impl ChatClient<'_> {
    /// Asks for the next action. Thinking is off (`reasoning_effort: "none"`): the M1.5 probe showed
    /// grounding does not need it, and it only adds latency.
    pub async fn next_tool_call(&self, system: &str, user_text: &str, screenshot: &str, tools: &Value) -> Result<Reply, ModelError> {
        let body = json!({
            "model": self.model,
            "temperature": 0,
            "max_tokens": MAX_OUTPUT_TOKENS,
            "reasoning_effort": "none",
            "messages": [
                { "role": "system", "content": system },
                // Image first, text last, like the probe: the screenshot is the expensive part.
                { "role": "user", "content": [
                    { "type": "image_url", "image_url": { "url": screenshot } },
                    { "type": "text", "text": user_text },
                ]},
            ],
            "tools": tools,
            "tool_choice": "required",
        });

        let mut request = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/')))
            .timeout(REQUEST_TIMEOUT)
            .json(&body);
        if let Some(key) = self.api_key {
            request = request.bearer_auth(key);
        }
        let response = request.send().await.map_err(|err| ModelError::Request(err.to_string()))?;
        let status = response.status();
        let text = response.text().await.map_err(|err| ModelError::Request(err.to_string()))?;
        if !status.is_success() {
            return Err(ModelError::Status { status: status.as_u16(), body: snippet(&text) });
        }
        let value: Value = serde_json::from_str(&text).map_err(|err| ModelError::Reply(format!("{err}: {}", snippet(&text))))?;
        parse_reply(&value)
    }
}

/// Pulls the first tool call out of a chat completion.
pub fn parse_reply(value: &Value) -> Result<Reply, ModelError> {
    let message = &value["choices"][0]["message"];
    let Some(call) = message["tool_calls"].get(0) else {
        return Err(ModelError::NoToolCall(snippet(message["content"].as_str().unwrap_or(""))));
    };
    let name = call["function"]["name"].as_str().unwrap_or("").to_string();
    // Arguments arrive as a JSON string; a few servers send an object instead.
    let arguments = match &call["function"]["arguments"] {
        Value::String(text) => serde_json::from_str(text).map_err(|err| ModelError::Reply(format!("arguments were not JSON: {err}")))?,
        other => other.clone(),
    };
    Ok(Reply { call: ToolCall { name, arguments }, completion_tokens: value["usage"]["completion_tokens"].as_u64() })
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
}
