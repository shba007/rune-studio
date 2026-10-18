//! Speech to text: one utterance in, the words out, through rune-infer's
//! `POST /v1/audio/transcriptions` (the OpenAI-style multipart call). The audio goes only to the
//! local server named in the settings.

use std::time::Duration;

use serde_json::Value;

use super::vad::RATE;

const TIMEOUT: Duration = Duration::from_secs(30);

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

/// What speech models write for silence or noise. These are never commands.
const PHANTOMS: &[&str] = &["you", "thank you", "thanks for watching", "bye", "the", "uh", "um", "hmm", "mm", "okay"];

/// Cleans a transcript and drops the ones that are not speech: empty, only punctuation, or one of
/// the stock phrases models make up for noise. `None` means "ignore this utterance".
pub fn clean(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_matches(|c: char| c.is_whitespace() || "[]()*".contains(c)).trim();
    let letters: String = trimmed.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect::<String>().trim().to_lowercase();
    if letters.is_empty() || PHANTOMS.contains(&letters.as_str()) {
        return None;
    }
    Some(trimmed.to_string())
}

pub async fn transcribe(http: &reqwest::Client, base_url: &str, api_key: Option<&str>, model: &str, samples: &[i16]) -> Result<Option<String>, String> {
    let file = reqwest::multipart::Part::bytes(wav(samples)).file_name("speech.wav").mime_str("audio/wav").map_err(|err| err.to_string())?;
    let form = reqwest::multipart::Form::new().part("file", file).text("model", model.to_string()).text("response_format", "json");
    let mut request = http.post(format!("{}/v1/audio/transcriptions", base_url.trim_end_matches('/'))).timeout(TIMEOUT).multipart(form);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(|err| format!("could not reach the speech model: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("the speech model answered {status}: {}", body.chars().take(200).collect::<String>()));
    }
    let value: Value = response.json().await.map_err(|err| format!("the speech model's reply was not understood: {err}"))?;
    Ok(value["text"].as_str().and_then(clean))
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
        assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()) as usize, file.len() - 8);
    }

    #[test]
    fn noise_transcripts_are_dropped_and_speech_is_kept_tidy() {
        assert_eq!(clean("  Open Notepad.  ").as_deref(), Some("Open Notepad."));
        for noise in ["", "   ", ".", "...", "you", "You.", "Thank you.", "[ ]"] {
            assert!(clean(noise).is_none(), "{noise:?}");
        }
        assert!(clean("stop").is_some());
        assert!(clean("Yes").is_some());
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Needs rune-infer running and `RUNE_TEST_WAV` pointing at a 16 kHz mono 16-bit WAV file.
    #[tokio::test]
    #[ignore = "needs rune-infer and a WAV file"]
    async fn a_spoken_file_comes_back_as_text() {
        let path = std::env::var("RUNE_TEST_WAV").expect("RUNE_TEST_WAV");
        let bytes = std::fs::read(path).unwrap();
        let samples: Vec<i16> = bytes[44..].chunks_exact(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect();
        let started = std::time::Instant::now();
        let text = transcribe(&reqwest::Client::new(), "http://127.0.0.1:8080", None, "parakeet-tdt-0.6b-v3", &samples).await.unwrap();
        println!("transcript: {text:?} in {} ms", started.elapsed().as_millis());
        assert!(text.is_some());
    }
}
