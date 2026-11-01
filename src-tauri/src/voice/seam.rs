use crate::{
    events::AgentState,
    model::{
        capabilities::{ModelCapabilities, OperationStatus, TriState},
        client::ChatClient,
        profile::ModelProfile,
    },
    voice::direct::{self, Listened},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// Seam abstraction for turning speech into text or structured voice intent.
pub trait Transcriber: Send + Sync {
    fn transcribe(
        &self,
        wav_bytes: &[u8],
        state: AgentState,
    ) -> impl std::future::Future<Output = Result<Option<Listened>, String>> + Send;
}

/// The default transcriber: sends the WAV audio to the chat model via `input_audio`.
pub struct ChatModelAudio<'a> {
    pub client: &'a ChatClient<'a>,
}

impl<'a> Transcriber for ChatModelAudio<'a> {
    async fn transcribe(
        &self,
        wav_bytes: &[u8],
        state: AgentState,
    ) -> Result<Option<Listened>, String> {
        let audio_base64 = BASE64.encode(wav_bytes);
        direct::listen(self.client, &audio_base64, state)
            .await
            .map_err(|err| err.to_string())
    }
}

pub fn use_verified_transcription(caps: &ModelCapabilities) -> bool {
    let audio_task = caps.tasks.iter().any(|task| {
        task.split_once("-to-").is_some_and(|(inputs, output)| {
            output == "text" && inputs.split('-').any(|input| input == "audio")
        })
    });
    !caps.supports_structured_output()
        && audio_task
        && caps.audio_operations.transcription.status == OperationStatus::Supported
}

pub fn voice_error_message(error: &str) -> String {
    if error.contains("swap_refused") {
        return "The voice model is not loaded. Load it in Infer and try again".into();
    }
    let message = error
        .find('{')
        .and_then(|at| {
            let value: serde_json::Value = serde_json::from_str(&error[at..]).ok()?;
            let detail = value.pointer("/error/message")?.as_str()?;
            Some(format!("{}{}", &error[..at], detail))
        })
        .unwrap_or_else(|| error.to_string());
    message.chars().take(240).collect()
}

fn transcription_error(status: reqwest::StatusCode, bytes: &[u8]) -> String {
    let body: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
    let code = body
        .pointer("/error/code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if code == "swap_refused" {
        return "The voice model is not loaded. Load it in Infer and try again".into();
    }
    let detail = body
        .pointer("/error/message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Voice transcription failed");
    format!(
        "Infer returned {status}: {}",
        detail.chars().take(240).collect::<String>()
    )
}

pub struct VerifiedTranscriber<'a> {
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub profile: &'a ModelProfile,
}

impl Transcriber for VerifiedTranscriber<'_> {
    async fn transcribe(
        &self,
        wav_bytes: &[u8],
        _state: AgentState,
    ) -> Result<Option<Listened>, String> {
        use crate::life::{
            DataClass,
            policy::{PolicyDecision, may_send_to_route},
        };
        if let PolicyDecision::Deny(reason) = may_send_to_route(
            Some(DataClass::Finance),
            self.base_url,
            Some(self.profile.locality.as_str()),
            false,
        ) {
            return Err(format!("Voice cannot be sent: {reason}"));
        }
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Could not create a local voice connection")?;
        let url = format!(
            "{}/v1/audio/transcriptions",
            self.base_url.trim_end_matches('/')
        );
        let part = reqwest::multipart::Part::bytes(wav_bytes.to_vec())
            .file_name("utterance.wav")
            .mime_str("audio/wav")
            .map_err(|_| "Could not prepare voice audio")?;
        let form = reqwest::multipart::Form::new()
            .text("model", self.model.to_string())
            .text("response_format", "json")
            .part("file", part);
        let mut request = http
            .post(url)
            .timeout(std::time::Duration::from_secs(300))
            .header("x-no-swap", "true")
            .multipart(form);
        if let Some(key) = self.api_key {
            request = request.bearer_auth(key);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| "Could not reach Infer for voice transcription")?;
        let status = response.status();
        if status.is_success() && crate::life::policy::response_is_cloud(response.headers()) {
            return Err("Infer reported a non-local voice response".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Could not read Infer's voice response")?
        {
            if bytes.len().saturating_add(chunk.len()) > 1024 * 1024 {
                return Err("Infer's voice response was too large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            return Err(transcription_error(status, &bytes));
        }
        let body: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| "Infer returned an unreadable voice transcript")?;
        let text = body
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or("Infer returned no voice transcript")?;
        Ok(super::intent::from_transcript(text).map(|intent| Listened {
            intent,
            goals: None,
        }))
    }
}

/// Supported dedicated voice transcription engines.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedicatedEngine {
    Whisper,
    Parakeet,
    LocalEngine(String),
}

/// A place for a future dedicated transcriber engine without hardcoding or requiring it today.
#[allow(dead_code)]
pub struct DedicatedTranscriber {
    pub engine: DedicatedEngine,
    pub endpoint: Option<String>,
}

#[cfg(test)]
impl DedicatedTranscriber {
    pub fn new(engine: DedicatedEngine, endpoint: Option<String>) -> Self {
        Self { engine, endpoint }
    }
}

impl Transcriber for DedicatedTranscriber {
    async fn transcribe(
        &self,
        _wav_bytes: &[u8],
        _state: AgentState,
    ) -> Result<Option<Listened>, String> {
        Err(format!(
            "dedicated transcriber engine {:?} not yet configured",
            self.engine
        ))
    }
}

/// Checks whether voice should be disabled for the given model profile.
pub fn is_voice_disabled_for_profile(profile: &ModelProfile) -> bool {
    profile.audio_in.supported == TriState::No
}

/// Returns a human-readable explanation if voice is disabled for the model.
pub fn voice_disabled_reason(model_id: &str, profile: &ModelProfile) -> Option<String> {
    if is_voice_disabled_for_profile(profile) {
        Some(format!(
            "Voice is disabled: model '{model_id}' does not support audio input."
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::intent::Intent;

    #[test]
    fn verified_transcription_is_only_used_when_structured_mode_is_not_advertised() {
        let mut caps = ModelCapabilities::unknown("lfm");
        caps.tasks = vec!["text-audio-to-text".into()];
        assert!(!use_verified_transcription(&caps));
        caps.audio_operations.transcription.status = OperationStatus::Supported;
        assert!(use_verified_transcription(&caps));
        caps.raw_capabilities.push("Structured Output".into());
        assert!(!use_verified_transcription(&caps));
        caps.raw_capabilities.clear();
        caps.tasks = vec!["text-to-text".into()];
        assert!(!use_verified_transcription(&caps));
        caps.tasks = vec!["audio-to-text".into()];
        caps.audio_operations.transcription.status = OperationStatus::Unsupported;
        assert!(!use_verified_transcription(&caps));
    }

    #[test]
    fn voice_errors_keep_infer_detail_and_explain_swap_refusal() {
        let raw = r#"the model server answered 400: {"error":{"code":"unsupported_response_format","message":"Structured output is unsupported"}}"#;
        assert_eq!(
            voice_error_message(raw),
            "the model server answered 400: Structured output is unsupported"
        );
        assert!(
            voice_error_message("skipped: model is not resident (swap_refused)")
                .contains("Load it in Infer")
        );
        assert!(
            transcription_error(
                reqwest::StatusCode::CONFLICT,
                br#"{"error":{"code":"swap_refused"}}"#
            )
            .contains("Load it in Infer")
        );
        assert!(
            transcription_error(
                reqwest::StatusCode::BAD_REQUEST,
                br#"{"error":{"message":"Audio format rejected"}}"#
            )
            .contains("Audio format rejected")
        );
    }

    #[tokio::test]
    async fn verified_transcription_enforces_local_route_before_network() {
        let mut profile = ModelProfile::conservative("m");
        profile.locality = crate::model::capabilities::Locality::Local;
        let transcriber = VerifiedTranscriber {
            base_url: "http://example.com:8080",
            api_key: None,
            model: "m",
            profile: &profile,
        };
        let error = transcriber
            .transcribe(&direct::wav(&[1, 2]), AgentState::Idle)
            .await
            .unwrap_err();
        assert!(error.contains("loopback"));
        profile.locality = crate::model::capabilities::Locality::Cloud;
        let transcriber = VerifiedTranscriber {
            base_url: "http://127.0.0.1:8080",
            api_key: None,
            model: "m",
            profile: &profile,
        };
        assert!(
            transcriber
                .transcribe(&direct::wav(&[1, 2]), AgentState::Idle)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn verified_transcription_sends_wav_and_preserves_control_intent() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = br#"{"text":"Stop!"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                response.len()
            )
            .unwrap();
            stream.write_all(response).unwrap();
            request
        });
        let mut profile = ModelProfile::conservative("m");
        profile.locality = crate::model::capabilities::Locality::Local;
        let base = format!("http://{address}");
        let transcriber = VerifiedTranscriber {
            base_url: &base,
            api_key: Some("test-key"),
            model: "m",
            profile: &profile,
        };
        let result = transcriber
            .transcribe(&direct::wav(&[1, -1]), AgentState::Idle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.intent, Intent::Stop);
        let request = server.join().unwrap();
        let text = String::from_utf8_lossy(&request);
        assert!(text.starts_with("POST /v1/audio/transcriptions "));
        assert!(text.to_ascii_lowercase().contains("x-no-swap: true"));
        assert!(
            text.to_ascii_lowercase()
                .contains("authorization: bearer test-key")
        );
        assert!(text.contains("name=\"model\""));
        assert!(text.contains("name=\"file\""));
        assert!(request.windows(4).any(|chunk| chunk == b"RIFF"));
    }

    /// Fake transcriber for testing the seam abstraction.
    struct FakeTranscriber {
        simulated_result: Option<Listened>,
    }

    impl Transcriber for FakeTranscriber {
        async fn transcribe(
            &self,
            _wav_bytes: &[u8],
            _state: AgentState,
        ) -> Result<Option<Listened>, String> {
            Ok(self.simulated_result.clone())
        }
    }

    #[test]
    fn voice_cleanly_disabled_for_audio_incompetent_model() {
        let mut audio_incompetent = ModelProfile::conservative("text-only-model");
        audio_incompetent.audio_in.supported = TriState::No;

        assert!(is_voice_disabled_for_profile(&audio_incompetent));
        assert_eq!(
            voice_disabled_reason("text-only-model", &audio_incompetent),
            Some("Voice is disabled: model 'text-only-model' does not support audio input.".into())
        );

        let mut audio_competent = ModelProfile::conservative("omni-model");
        audio_competent.audio_in.supported = TriState::Yes;
        assert!(!is_voice_disabled_for_profile(&audio_competent));
        assert_eq!(voice_disabled_reason("omni-model", &audio_competent), None);

        let unknown = ModelProfile::conservative("unknown-model");
        assert!(!is_voice_disabled_for_profile(&unknown));
        assert_eq!(voice_disabled_reason("unknown-model", &unknown), None);
    }

    #[tokio::test]
    async fn transcriber_trait_with_fake_provider() {
        let expected = Listened {
            intent: Intent::Say("open calculator".into()),
            goals: Some(vec!["Open Calculator".into()]),
        };

        let fake = FakeTranscriber {
            simulated_result: Some(Listened {
                intent: Intent::Say("open calculator".into()),
                goals: Some(vec!["Open Calculator".into()]),
            }),
        };

        let wav = direct::wav(&[0, 100, -100, 0]);
        let result = fake.transcribe(&wav, AgentState::Idle).await.unwrap();
        assert!(result.is_some());
        let listened = result.unwrap();
        assert_eq!(listened.intent, expected.intent);
        assert_eq!(listened.goals, expected.goals);
    }

    #[tokio::test]
    async fn dedicated_transcriber_placeholder() {
        let transcriber = DedicatedTranscriber::new(DedicatedEngine::Whisper, None);
        let wav = direct::wav(&[1, 2, 3]);
        let err = transcriber
            .transcribe(&wav, AgentState::Idle)
            .await
            .unwrap_err();
        assert!(err.contains("Whisper"));
    }

    #[test]
    fn wav_formatting_unchanged_riff_headers() {
        let samples = [100, -200, 300, -400];
        let wav_data = direct::wav(&samples);

        assert_eq!(&wav_data[..4], b"RIFF");
        assert_eq!(&wav_data[8..16], b"WAVEfmt ");
        assert_eq!(&wav_data[36..40], b"data");
        // 44-byte WAV header + 4 samples * 2 bytes = 52 bytes
        assert_eq!(wav_data.len(), 44 + samples.len() * 2);
    }
}
