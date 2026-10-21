use crate::{
    events::AgentState,
    model::{capabilities::TriState, client::ChatClient, profile::ModelProfile},
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
