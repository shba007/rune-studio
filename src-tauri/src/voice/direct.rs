//! Speech straight to meaning: one call to the omni model hears the utterance and says what it
//! is, with no transcript in between (there is no separate speech-to-text model).
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

use super::{intent::Intent, vad::RATE};

/// A 16-bit mono PCM WAV file of 16 kHz audio.
pub fn wav(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1_u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2_u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

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
        AgentState::AwaitingConfirm => {
            "The assistant is waiting for the user to answer a yes/no question."
        }
        AgentState::Planning | AgentState::Acting | AgentState::Observing => {
            "The assistant is busy carrying out a task."
        }
        _ => "The assistant is idle.",
    }
}

/// Most tokens the model may use to answer: a sentence and a few goals. A longer reply is the
/// model running on (it loops on a repeated word when it has nothing to say).
const MAX_TRANSCRIPT_TOKENS: u32 = 256;
/// A word said more often than this in a row is the model looping, not a person talking.
const MAX_REPEATS: usize = 3;

/// Whether the same word comes more than `MAX_REPEATS` times in a row ("warn warn warn warn").
pub fn repeats_a_word(text: &str) -> bool {
    let mut run = 0;
    let mut previous = String::new();
    for word in text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
    {
        run = if word == previous { run + 1 } else { 1 };
        if run > MAX_REPEATS {
            return true;
        }
        previous = word;
    }
    false
}

/// Why a `heard` call is ignored, or `None` when it is acted on. Every ignored result is logged with this.
pub fn ignore_reason(arguments: &Value) -> Option<&'static str> {
    let text = arguments["text"].as_str().unwrap_or("").trim();
    match arguments["kind"].as_str() {
        None => Some("the reply had no kind"),
        Some("noise") => Some("the model said it was noise"),
        Some("task") if text.is_empty() => Some("a task with no words"),
        Some("task") if repeats_a_word(text) => {
            Some("the words repeat one word, so the model was looping")
        }
        Some("stop" | "stop_listening" | "yes" | "no" | "task") => None,
        Some(_) => Some("an unknown kind"),
    }
}

/// Reads the model's `heard` call. `None` means it is ignored (see `ignore_reason`).
pub fn parse(arguments: &Value) -> Option<Listened> {
    if ignore_reason(arguments).is_some() {
        return None;
    }
    let text = arguments["text"].as_str().unwrap_or("").trim().to_string();
    let intent = match arguments["kind"].as_str()? {
        "stop" => Intent::Stop,
        "stop_listening" => Intent::StopListening,
        "yes" => Intent::Yes,
        "no" => Intent::No,
        "task" => Intent::Say(text),
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

pub async fn listen(
    client: &ChatClient<'_>,
    wav_base64: &str,
    state: AgentState,
) -> Result<Option<Listened>, ModelError> {
    let tools = tools();
    let request = Request {
        system: crate::prompts::voice_direct(),
        user_text: describe(state),
        screenshot: None,
        audio: Some(wav_base64),
        tools: &tools,
        think: false,
        max_tokens: Some(MAX_TRANSCRIPT_TOKENS),
    };
    let reply = client.call(&request, &mut |_: &str| {}).await?;
    if reply.call.name != "heard" {
        log::info!(
            "voice ignored: the model called {} instead of heard",
            reply.call.name
        );
        return Ok(None);
    }
    let listened = parse(&reply.call.arguments);
    if let Some(reason) = ignore_reason(&reply.call.arguments) {
        log::info!("voice ignored: {reason} ({})", reply.call.arguments);
    }
    Ok(listened)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wav_header_describes_what_follows() {
        let file = wav(&[1, -2, 3]);
        assert_eq!(&file[..4], b"RIFF");
        assert_eq!(&file[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(file[24..28].try_into().unwrap()), RATE);
        assert_eq!(&file[36..40], b"data");
        assert_eq!(u32::from_le_bytes(file[40..44].try_into().unwrap()), 6);
        assert_eq!(file.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes(file[46..48].try_into().unwrap()), -2);
        assert_eq!(
            u32::from_le_bytes(file[4..8].try_into().unwrap()) as usize,
            file.len() - 8
        );
    }

    #[test]
    fn control_words_and_tasks_are_read() {
        assert_eq!(
            parse(&json!({ "kind": "stop" })).unwrap().intent,
            Intent::Stop
        );
        assert_eq!(
            parse(&json!({ "kind": "yes" })).unwrap().intent,
            Intent::Yes
        );
        let task = parse(&json!({ "kind": "task", "text": " open notepad ", "goals": ["Open Notepad", " ", "Type hello"] })).unwrap();
        assert_eq!(task.intent, Intent::Say("open notepad".into()));
        assert_eq!(
            task.goals,
            Some(vec!["Open Notepad".into(), "Type hello".into()])
        );
    }

    #[test]
    fn a_model_that_loops_on_a_word_is_ignored_with_a_reason() {
        assert!(repeats_a_word("warn warn warn warn"));
        assert!(repeats_a_word("Warn, warn. WARN warn"));
        assert!(
            !repeats_a_word("warn warn warn"),
            "three in a row is allowed"
        );
        assert!(!repeats_a_word("open notepad and write hello hello"));
        assert!(!repeats_a_word("no no no wait stop"));
        let looping = json!({ "kind": "task", "text": "warn warn warn warn warn" });
        assert!(parse(&looping).is_none());
        assert_eq!(
            ignore_reason(&looping),
            Some("the words repeat one word, so the model was looping")
        );
        assert_eq!(
            ignore_reason(&json!({ "kind": "noise" })),
            Some("the model said it was noise")
        );
        assert_eq!(
            ignore_reason(&json!({ "kind": "task", "text": " " })),
            Some("a task with no words")
        );
        assert_eq!(
            ignore_reason(&json!({ "kind": "dance" })),
            Some("an unknown kind")
        );
        assert_eq!(ignore_reason(&json!({})), Some("the reply had no kind"));
        assert_eq!(
            ignore_reason(&json!({ "kind": "task", "text": "open notepad" })),
            None
        );
        assert_eq!(ignore_reason(&json!({ "kind": "stop" })), None);
    }

    #[test]
    fn the_request_always_has_a_transcribe_only_prompt_and_a_cap() {
        assert!(crate::prompts::voice_direct().contains("Transcribe only"));
        assert!(crate::prompts::voice_direct().contains("never guess a word"));
        let tools = tools();
        let request = Request {
            system: crate::prompts::voice_direct(),
            user_text: "idle",
            screenshot: None,
            audio: Some("AA=="),
            tools: &tools,
            think: false,
            max_tokens: Some(MAX_TRANSCRIPT_TOKENS),
        };
        let body = crate::model::client::chat_body("m", &request);
        assert_eq!(body["max_tokens"], 256);
        assert_eq!(body["messages"][0]["role"], "system");
    }

    #[test]
    fn noise_and_empty_tasks_are_ignored() {
        assert!(parse(&json!({ "kind": "noise" })).is_none());
        assert!(parse(&json!({ "kind": "task", "text": "  " })).is_none());
        assert!(parse(&json!({ "kind": "nonsense" })).is_none());
    }
}
