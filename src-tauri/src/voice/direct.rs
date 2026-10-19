//! Speech straight to meaning: one call to an audio-capable model (a Qwen Omni build on
//! rune-infer) hears the utterance and says what it is, with no transcript in between.
//!
//! The model answers with one `heard` tool call: a control word (stop, yes, no), a task or a
//! steer in its own words (plus the goals when it starts a task, so no separate planning call is
//! needed), or noise. The words it returns are only for display and for the agent loop; the
//! safety gate still decides what is allowed, and a spoken yes still cannot approve a risky action.

use serde_json::{Value, json};

use crate::{
    events::AgentState,
    model::client::{ChatClient, ModelError, Request},
};

use super::intent::Intent;

const SYSTEM: &str = "\
You are the ears of a voice assistant that controls a Windows PC. You are given a short recording of the user and the state of the assistant. Call heard exactly once.

kind:
- stop: the whole recording is only a request to stop or cancel what is happening (\"stop\", \"cancel that\", \"never mind\").
- stop_listening: the whole recording is only a request to turn the microphone off (\"stop listening\", \"that's all\").
- yes / no: the whole recording is only an answer to a question (\"yes\", \"go ahead\", \"no\", \"don't\").
- task: anything else the user wants done, or, if a task is already running, extra instructions for it. Put it in \"text\" as one clear sentence in the user's language, keeping names, numbers and quoted words exactly.
- noise: silence, a cough, music, a TV or a voice that is not talking to the assistant, or words you cannot make out. When unsure, use noise.

A longer sentence that merely contains a control word (\"stop the music in Spotify\") is a task.
When kind is task and the assistant is idle, also give \"goals\": two to six short goals (a few words each, one or two for a simple task).";

pub struct Listened {
    pub intent: Intent,
    /// Goals for a new task, when the model gave them.
    pub goals: Option<Vec<String>>,
}

fn tools() -> Value {
    json!([{ "type": "function", "function": {
        "name": "heard",
        "description": "Say what the recording was.",
        "parameters": { "type": "object", "required": ["kind"], "properties": {
            "kind": { "type": "string", "enum": ["stop", "stop_listening", "yes", "no", "task", "noise"] },
            "text": { "type": "string", "description": "For task: what the user wants, as one sentence." },
            "goals": { "type": "array", "items": { "type": "string" }, "description": "For task when idle: the goals, in order." },
        }},
    }}])
}

fn describe(state: AgentState) -> &'static str {
    match state {
        AgentState::AwaitingConfirm => "The assistant is waiting for the user to answer a yes/no question.",
        AgentState::Planning | AgentState::Acting | AgentState::Observing => "The assistant is busy carrying out a task.",
        _ => "The assistant is idle.",
    }
}

/// Reads the model's `heard` call. `None` means noise.
pub fn parse(arguments: &Value) -> Option<Listened> {
    let text = arguments["text"].as_str().unwrap_or("").trim().to_string();
    let intent = match arguments["kind"].as_str()? {
        "stop" => Intent::Stop,
        "stop_listening" => Intent::StopListening,
        "yes" => Intent::Yes,
        "no" => Intent::No,
        "task" if !text.is_empty() => Intent::Say(text),
        _ => return None,
    };
    let goals = arguments["goals"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(|goal| goal.trim().chars().take(80).collect::<String>())
                .filter(|goal| !goal.is_empty())
                .take(crate::agent::protocol::MAX_GOALS)
                .collect::<Vec<_>>()
        })
        .filter(|goals| !goals.is_empty());
    Some(Listened { intent, goals })
}

pub async fn listen(client: &ChatClient<'_>, wav_base64: &str, state: AgentState) -> Result<Option<Listened>, ModelError> {
    let tools = tools();
    let request = Request {
        system: SYSTEM,
        user_text: describe(state),
        screenshot: None,
        audio: Some(wav_base64),
        tools: &tools,
        think: false,
    };
    let reply = client.call(&request, &mut |_: &str| {}).await?;
    Ok(if reply.call.name == "heard" { parse(&reply.call.arguments) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_words_and_tasks_are_read() {
        assert_eq!(parse(&json!({ "kind": "stop" })).unwrap().intent, Intent::Stop);
        assert_eq!(parse(&json!({ "kind": "yes" })).unwrap().intent, Intent::Yes);
        let task = parse(&json!({ "kind": "task", "text": " open notepad ", "goals": ["Open Notepad", " ", "Type hello"] })).unwrap();
        assert_eq!(task.intent, Intent::Say("open notepad".into()));
        assert_eq!(task.goals, Some(vec!["Open Notepad".into(), "Type hello".into()]));
    }

    #[test]
    fn noise_and_empty_tasks_are_ignored() {
        assert!(parse(&json!({ "kind": "noise" })).is_none());
        assert!(parse(&json!({ "kind": "task", "text": "  " })).is_none());
        assert!(parse(&json!({ "kind": "nonsense" })).is_none());
    }
}
