pub mod capture;
pub mod direct;
pub mod intent;
pub mod vad;

use crate::sync::LockExt;
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

#[derive(Clone, Serialize)]
pub struct Heard {
    pub text: String,
    pub note: String,
}

fn heard(app: &AppHandle, text: &str, note: impl Into<String>) {
    let _ = app.emit(
        VOICE_HEARD,
        Heard {
            text: text.to_string(),
            note: note.into(),
        },
    );
}

pub fn resume_listening_soon(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let state = app.state::<AppState>();
        let open = state.mic.locked().is_some();
        if open
            && matches!(
                events::agent_state(&app),
                AgentState::Done | AgentState::Failed | AgentState::Stopped
            )
        {
            events::set_agent_state(&app, AgentState::Listening);
        }
    });
}

static STOPS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const START_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

pub fn note_stop() {
    STOPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

pub async fn handle(app: &AppHandle, utterance: Vec<i16>) {
    crate::reflect::touch(app, crate::reflect::idle::Input::Voice);
    let state = app.state::<AppState>();
    let (base_url, model) = {
        let settings = state.settings.locked();
        (settings.base_url.clone(), settings.model.clone())
    };
    let api_key = settings::api_key();
    let wav = direct::wav(&utterance);
    let saved = match crate::archive::save(crate::archive::Kind::Voice, "utterance", "wav", &wav) {
        Ok(path) => path,
        Err(err) => {
            log::error!("{err}");
            heard(app, "", "Not acted on: the recording could not be stored");
            return;
        }
    };
    let client = ChatClient {
        http: &state.http,
        base_url: &base_url,
        api_key: api_key.as_deref(),
        model: &model,
        sink: None,
    };
    let audio = BASE64.encode(&wav);
    match direct::listen(&client, &audio, events::agent_state(app)).await {
        Ok(Some(listened)) => {
            log::info!("heard: {:?}", listened.intent);
            apply(
                app,
                listened.intent,
                listened.goals,
                "",
                Some(saved.as_path()),
            );
        }
        Ok(None) => {}
        Err(err) => {
            log::warn!("hearing failed: {err}");
            heard(
                app,
                "",
                "Could not understand that: the model is not answering",
            );
        }
    }
}

fn apply(
    app: &AppHandle,
    intent: Intent,
    goals: Option<Vec<String>>,
    text: &str,
    recording: Option<&std::path::Path>,
) {
    let now = events::agent_state(app);
    match intent {
        Intent::Stop => {
            if now.is_running() {
                heard(app, text, "Stopped");
                crate::commands::stop(app.clone());
            }
        }
        Intent::StopListening => {
            heard(app, text, "Stopped listening");
            crate::commands::close_microphone(app);
        }
        Intent::No => {
            if now == AgentState::AwaitingConfirm
                && gate::answer_by_voice(app, false) == VoiceAnswer::Answered
            {
                heard(app, text, "Answered no");
            }
        }
        Intent::Yes => {
            if now == AgentState::AwaitingConfirm {
                match gate::answer_by_voice(app, true) {
                    VoiceAnswer::Answered => heard(app, text, "Answered yes"),
                    VoiceAnswer::NeedsClick => heard(
                        app,
                        text,
                        "Click Yes to allow this one: it is not allowed by voice",
                    ),
                    VoiceAnswer::NothingWaiting => {}
                }
            }
        }
        Intent::Say(said) => {
            if now == AgentState::AwaitingConfirm {
                heard(app, &said, "Say yes or no");
            } else if now.is_running() {
                if crate::commands::push_steer(app, &said) {
                    heard(app, &said, "Passed to the agent");
                }
            } else if let Some(recording) = recording {
                heard(app, &said, "Starting in 2 s. Press Stop to cancel");
                let (app, said, recording) = (app.clone(), said, recording.to_path_buf());
                let stops = STOPS.load(std::sync::atomic::Ordering::SeqCst);
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(START_DELAY).await;
                    if STOPS.load(std::sync::atomic::Ordering::SeqCst) != stops {
                        log::info!("voice task cancelled before it started: {said}");
                        heard(&app, &said, "Cancelled");
                        return;
                    }
                    match crate::commands::begin_task_planned(&app, &said, true, goals) {
                        Ok(()) => {
                            crate::archive::adopt(&recording);
                            heard(&app, &said, "Starting");
                        }
                        Err(err) => heard(&app, &said, err.to_string()),
                    }
                });
            } else {
                match crate::commands::begin_task_planned(app, &said, true, goals) {
                    Ok(()) => {
                        if let Some(recording) = recording {
                            crate::archive::adopt(recording);
                        }
                        heard(app, &said, "Starting");
                    }
                    Err(err) => heard(app, &said, err.to_string()),
                }
            }
        }
    }
}
