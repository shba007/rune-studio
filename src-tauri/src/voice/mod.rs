//! Talking to the agent: the microphone, cutting speech into utterances, turning them into text,
//! and acting on what was said.
//!
//! While listening, every utterance is transcribed by rune-infer (locally) and then (with the
//! "Direct voice" setting on and an audio-capable model, `direct` hears it with no transcript):
//! - **Stop** (the word, on its own) stops the agent at once, like the Stop button.
//! - **Yes / No** answer the question on screen. A spoken **no** always works. A spoken **yes**
//!   works for the agent's own questions, but not for an action that needs approval: a TV or a
//!   colleague saying "yes" must never be able to allow something risky, so that one needs a click.
//! - Anything else is a **task** when the agent is idle, or a **steer** (extra instructions the
//!   agent reads before its next step) when it is working.

pub mod capture;
pub mod direct;
pub mod intent;
pub mod stt;
pub mod vad;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    events::{self, AgentState, AppState},
    model::client::ChatClient,
    safety::gate::{self, VoiceAnswer},
    settings,
};
use intent::Intent;

pub const VOICE_HEARD: &str = "voice_heard";

/// What the UI shows after something was said: the words, and what the app did about them.
#[derive(Clone, Serialize)]
pub struct Heard {
    pub text: String,
    pub note: String,
}

fn heard(app: &AppHandle, text: &str, note: impl Into<String>) {
    let _ = app.emit(VOICE_HEARD, Heard { text: text.to_string(), note: note.into() });
}

fn running(state: AgentState) -> bool {
    matches!(state, AgentState::Planning | AgentState::AwaitingConfirm | AgentState::Acting | AgentState::Observing)
}

/// After a task ends (or is stopped) with the microphone still open, go back to listening once
/// the result has had a moment on screen, so the user can simply say the next thing.
pub fn resume_listening_soon(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let state = app.state::<AppState>();
        let open = state.mic.lock().expect("mic lock poisoned").is_some();
        if open && matches!(events::agent_state(&app), AgentState::Done | AgentState::Failed | AgentState::Stopped) {
            events::set_agent_state(&app, AgentState::Listening);
        }
    });
}

/// One utterance, start to finish: transcribe, interpret, act.
pub async fn handle(app: &AppHandle, utterance: Vec<i16>) {
    let state = app.state::<AppState>();
    let (base_url, model, vision_model, direct_voice) = {
        let settings = state.settings.lock().expect("settings lock poisoned");
        (settings.base_url.clone(), settings.asr_model.clone(), settings.vision_model.clone(), settings.direct_voice)
    };
    let api_key = settings::api_key();
    if direct_voice {
        // No transcript: the audio model hears it and says what it was. If it cannot (the model
        // does not take audio, or the server is down) fall through to transcription below.
        let client = ChatClient { http: &state.http, base_url: &base_url, api_key: api_key.as_deref(), model: &vision_model };
        let audio = BASE64.encode(stt::wav(&utterance));
        match direct::listen(&client, &audio, events::agent_state(app)).await {
            Ok(Some(listened)) => {
                log::info!("heard directly: {:?}", listened.intent);
                apply(app, listened.intent, listened.goals, "");
                return;
            }
            Ok(None) => return,
            Err(err) => log::warn!("direct speech failed, transcribing instead: {err}"),
        }
    }
    let text = match stt::transcribe(&state.http, &base_url, api_key.as_deref(), &model, &utterance).await {
        Ok(Some(text)) => text,
        Ok(None) => return,
        Err(err) => {
            log::warn!("speech to text failed: {err}");
            heard(app, "", "Could not understand that: the speech model is not answering");
            return;
        }
    };
    log::info!("heard: {text}");
    act(app, &text);
}

/// Does what the words ask for. Separate from `handle` so it can be tested without audio.
pub fn act(app: &AppHandle, text: &str) {
    apply(app, intent::interpret(text), None, text);
}

/// Does what an intent asks for. `text` is what was heard when there is a transcript; without
/// one (direct speech) the model's own wording of a task is shown instead. `goals` are the plan an
/// audio model already made for a new task.
fn apply(app: &AppHandle, intent: Intent, goals: Option<Vec<String>>, text: &str) {
    let now = events::agent_state(app);
    match intent {
        Intent::Stop => {
            if running(now) {
                heard(app, text, "Stopped");
                crate::commands::stop(app.clone());
            }
        }
        Intent::StopListening => {
            heard(app, text, "Stopped listening");
            crate::commands::close_microphone(app);
        }
        Intent::No => {
            if now == AgentState::AwaitingConfirm {
                if gate::answer_by_voice(app, false) == VoiceAnswer::Answered {
                    heard(app, text, "Answered no");
                }
            }
        }
        Intent::Yes => {
            if now == AgentState::AwaitingConfirm {
                match gate::answer_by_voice(app, true) {
                    VoiceAnswer::Answered => heard(app, text, "Answered yes"),
                    VoiceAnswer::NeedsClick => heard(app, text, "Click Yes to allow this one: it is not allowed by voice"),
                    VoiceAnswer::NothingWaiting => {}
                }
            }
        }
        Intent::Say(said) => {
            if now == AgentState::AwaitingConfirm {
                heard(app, &said, "Say yes or no");
            } else if running(now) {
                if crate::commands::push_steer(app, &said) {
                    heard(app, &said, "Passed to the agent");
                }
            } else {
                match crate::commands::begin_task_planned(app, &said, true, goals) {
                    Ok(()) => heard(app, &said, "Starting"),
                    Err(err) => heard(app, &said, err.to_string()),
                }
            }
        }
    }
}
