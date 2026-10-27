pub mod capture;
pub mod direct;
pub mod intent;
pub mod seam;
pub mod vad;

use crate::sync::LockExt;
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
    if state.settings.locked().external_agent {
        return;
    }
    let (base_url, voice_model, api_key) = {
        let settings = state.settings.locked();
        (
            settings.base_url.clone(),
            settings
                .model_for_role(crate::model::role::ModelRole::Voice)
                .to_string(),
            settings::api_key_for(&settings),
        )
    };
    let wav = direct::wav(&utterance);
    let profile = match crate::model::protected::discover(
        &state.http,
        &base_url,
        api_key.as_deref(),
        &voice_model,
    )
    .await
    {
        Ok(profile) => profile,
        Err(error) => {
            heard(app, "", error);
            return;
        }
    };
    if let Some(reason) = seam::voice_disabled_reason(&voice_model, &profile) {
        log::warn!("voice disabled: {reason}");
        heard(app, "", reason);
        crate::commands::close_microphone(app);
        return;
    }

    let client = ChatClient {
        http: &state.http,
        base_url: &base_url,
        api_key: api_key.as_deref(),
        model: &voice_model,
        profile: Some(&profile),
        sink: None,
        no_swap: false,
        on_loading: None,
    };
    use seam::Transcriber;
    let transcriber = seam::ChatModelAudio { client: &client };
    let mode = events::agent_state(app);
    let mode = if crate::intake::busy() && !mode.is_running() {
        AgentState::Planning
    } else {
        mode
    };
    match transcriber.transcribe(&wav, mode).await {
        Ok(Some(listened)) => {
            if let Intent::Say(said) = &listened.intent
                && !events::agent_state(app).is_running()
            {
                let (app, said) = (app.clone(), said.clone());
                heard(&app, &said, "Submitting in 2 s. Press Stop to cancel");
                let stops = STOPS.load(std::sync::atomic::Ordering::SeqCst);
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(START_DELAY).await;
                    if STOPS.load(std::sync::atomic::Ordering::SeqCst) != stops {
                        heard(&app, &said, "Cancelled");
                        return;
                    }
                    match crate::intake::submit_voice(&app, &said, true, wav) {
                        Ok(_) => heard(&app, &said, "Submitted"),
                        Err(error) => heard(&app, &said, error),
                    }
                });
                return;
            }
            let saved =
                match crate::archive::save(crate::archive::Kind::Voice, "utterance", "wav", &wav) {
                    Ok(path) => path,
                    Err(error) => {
                        heard(app, "", error.to_string());
                        return;
                    }
                };
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
            if now.is_running() || crate::intake::busy() {
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
                            heard(&app, &said, "Submitted");
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
                        heard(app, &said, "Submitted");
                    }
                    Err(err) => heard(app, &said, err.to_string()),
                }
            }
        }
    }
}
