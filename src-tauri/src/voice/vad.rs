//! Cutting the microphone stream into utterances.
//!
//! Plain energy-based voice activity detection, no model: it tracks the room's noise level and
//! calls a 20 ms frame speech when it is clearly louder than that. An utterance starts after a
//! few speech frames in a row (with a little audio from just before, so the first syllable is
//! not clipped) and ends after a short silence. Clicks and coughs are too short to count. The
//! output is 16 kHz mono, the format speech models want.

/// Samples per second of what the segmenter works on and emits.
pub const RATE: u32 = 16_000;
const FRAME: usize = (RATE as usize) / 50;
/// Speech frames in a row before an utterance starts (60 ms).
const START_FRAMES: usize = 3;
/// Audio kept from before the start, so the first syllable survives (200 ms).
const PRE_ROLL_FRAMES: usize = 10;
/// Silence that ends an utterance (700 ms).
const END_FRAMES: usize = 35;
/// Shorter than this is a click or a cough, not speech (200 ms of speech frames).
const MIN_SPEECH_FRAMES: usize = 10;
/// An utterance is cut here even if the person keeps talking (20 s).
const MAX_FRAMES: usize = 20 * 50;
/// Speech must be this much louder than the room (dB), and above an absolute floor.
const MARGIN_DB: f32 = 12.0;
const ABSOLUTE_FLOOR_DB: f32 = -50.0;
/// The room's noise level is kept within this range (dBFS).
const NOISE_RANGE: (f32, f32) = (-75.0, -42.0);

#[derive(Debug)]
pub struct Segmenter {
    pending: Vec<f32>,
    noise_db: f32,
    pre_roll: std::collections::VecDeque<Vec<i16>>,
    speech: Vec<i16>,
    in_speech: bool,
    loud_run: usize,
    quiet_run: usize,
    speech_frames: usize,
}

impl Default for Segmenter {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            noise_db: -60.0,
            pre_roll: std::collections::VecDeque::new(),
            speech: Vec::new(),
            in_speech: false,
            loud_run: 0,
            quiet_run: 0,
            speech_frames: 0,
        }
    }
}

fn level_db(frame: &[f32]) -> f32 {
    let mean_square =
        frame.iter().map(|sample| sample * sample).sum::<f32>() / frame.len().max(1) as f32;
    10.0 * mean_square.max(1e-12).log10()
}

fn to_pcm(frame: &[f32]) -> Vec<i16> {
    frame
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
        .collect()
}

impl Segmenter {
    /// Feeds 16 kHz mono samples (-1 to 1). Returns every utterance that ended within them.
    pub fn push(&mut self, samples: &[f32]) -> Vec<Vec<i16>> {
        let mut done = Vec::new();
        self.pending.extend_from_slice(samples);
        while self.pending.len() >= FRAME {
            let frame: Vec<f32> = self.pending.drain(..FRAME).collect();
            if let Some(utterance) = self.frame(&frame) {
                done.push(utterance);
            }
        }
        done
    }

    /// Ends the utterance in progress, if it is long enough to be speech.
    #[cfg(test)]
    pub fn flush(&mut self) -> Option<Vec<i16>> {
        self.finish()
    }

    fn frame(&mut self, frame: &[f32]) -> Option<Vec<i16>> {
        let db = level_db(frame);
        let loud = db > ABSOLUTE_FLOOR_DB && db > self.noise_db + MARGIN_DB;
        let pcm = to_pcm(frame);

        if !loud {
            // The room's level follows quiet frames only, slowly, so speech never raises it.
            self.noise_db = (self.noise_db * 0.95 + db * 0.05).clamp(NOISE_RANGE.0, NOISE_RANGE.1);
        }

        if self.in_speech {
            self.speech.extend_from_slice(&pcm);
            if loud {
                self.speech_frames += 1;
                self.quiet_run = 0;
            } else {
                self.quiet_run += 1;
            }
            if self.quiet_run >= END_FRAMES || self.speech.len() >= MAX_FRAMES * FRAME {
                return self.finish();
            }
            return None;
        }

        self.pre_roll.push_back(pcm);
        while self.pre_roll.len() > PRE_ROLL_FRAMES + START_FRAMES {
            self.pre_roll.pop_front();
        }
        self.loud_run = if loud { self.loud_run + 1 } else { 0 };
        if self.loud_run >= START_FRAMES {
            self.in_speech = true;
            self.speech = self.pre_roll.drain(..).flatten().collect();
            self.speech_frames = self.loud_run;
            self.quiet_run = 0;
        }
        None
    }

    fn finish(&mut self) -> Option<Vec<i16>> {
        let utterance = std::mem::take(&mut self.speech);
        let long_enough = self.speech_frames >= MIN_SPEECH_FRAMES;
        // Trailing silence is trimmed to a short tail; the model does not need 700 ms of it.
        let trim = self.quiet_run.saturating_sub(10) * FRAME;
        self.in_speech = false;
        self.loud_run = 0;
        self.quiet_run = 0;
        self.speech_frames = 0;
        self.pre_roll.clear();
        let keep = utterance
            .len()
            .saturating_sub(trim)
            .max(FRAME)
            .min(utterance.len());
        long_enough.then(|| utterance[..keep].to_vec())
    }
}

/// Turns device audio (any rate, any channel count, interleaved) into 16 kHz mono by averaging
/// the input samples that fall into each output sample, which also filters out what a plain
/// pick-every-nth would fold back as noise.
#[derive(Debug)]
pub struct Resampler {
    channels: usize,
    ratio: f64,
    sum: f64,
    count: f64,
    position: f64,
    frame_sum: f32,
    frame_count: usize,
}

impl Resampler {
    pub fn new(input_rate: u32, channels: u16) -> Self {
        Self {
            channels: usize::from(channels).max(1),
            ratio: f64::from(input_rate) / f64::from(RATE),
            sum: 0.0,
            count: 0.0,
            position: 0.0,
            frame_sum: 0.0,
            frame_count: 0,
        }
    }

    pub fn push(&mut self, interleaved: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(
            interleaved.len() / self.channels / (self.ratio as usize).max(1) + 1,
        );
        for sample in interleaved {
            self.frame_sum += sample;
            self.frame_count += 1;
            if self.frame_count < self.channels {
                continue;
            }
            let mono = self.frame_sum / self.channels as f32;
            self.frame_sum = 0.0;
            self.frame_count = 0;

            self.sum += f64::from(mono);
            self.count += 1.0;
            self.position += 1.0;
            if self.position >= self.ratio {
                out.push((self.sum / self.count) as f32);
                self.sum = 0.0;
                self.count = 0.0;
                self.position -= self.ratio;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tone as loud as speech, `seconds` long, at 16 kHz.
    fn tone(seconds: f32, amplitude: f32) -> Vec<f32> {
        (0..(seconds * RATE as f32) as usize)
            .map(|i| amplitude * (i as f32 * 0.35).sin())
            .collect()
    }

    fn silence(seconds: f32) -> Vec<f32> {
        vec![0.0; (seconds * RATE as f32) as usize]
    }

    fn run(parts: &[Vec<f32>]) -> Vec<Vec<i16>> {
        let mut segmenter = Segmenter::default();
        let mut all = Vec::new();
        for part in parts {
            all.extend(segmenter.push(part));
        }
        all
    }

    #[test]
    fn speech_between_silences_is_one_utterance_with_a_little_context() {
        let found = run(&[silence(1.0), tone(1.0, 0.2), silence(1.5)]);
        assert_eq!(found.len(), 1);
        let seconds = found[0].len() as f32 / RATE as f32;
        assert!((1.0..1.6).contains(&seconds), "{seconds}s");
    }

    #[test]
    fn two_phrases_with_a_pause_are_two_utterances() {
        let found = run(&[
            silence(0.5),
            tone(0.8, 0.2),
            silence(1.2),
            tone(0.8, 0.2),
            silence(1.2),
        ]);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn a_short_pause_inside_a_phrase_does_not_split_it() {
        let found = run(&[
            silence(0.5),
            tone(0.6, 0.2),
            silence(0.3),
            tone(0.6, 0.2),
            silence(1.2),
        ]);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_click_is_not_speech() {
        assert!(run(&[silence(0.5), tone(0.08, 0.3), silence(1.5)]).is_empty());
    }

    #[test]
    fn steady_room_noise_is_not_speech() {
        assert!(run(&[tone(5.0, 0.002)]).is_empty());
    }

    #[test]
    fn speech_over_steady_noise_still_gets_through() {
        let noise = |seconds: f32| tone(seconds, 0.003);
        let mut talking = tone(1.0, 0.25);
        for (sample, hiss) in talking.iter_mut().zip(noise(1.0)) {
            *sample += hiss;
        }
        let found = run(&[noise(1.5), talking, noise(1.5)]);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_very_long_monologue_is_cut_and_the_rest_still_comes() {
        let found = run(&[tone(25.0, 0.2), silence(1.5)]);
        assert!(found.len() >= 2, "{} utterance(s)", found.len());
    }

    #[test]
    fn stopping_the_mic_mid_sentence_keeps_what_was_said() {
        let mut segmenter = Segmenter::default();
        assert!(segmenter.push(&silence(0.5)).is_empty());
        assert!(segmenter.push(&tone(1.0, 0.2)).is_empty());
        assert!(segmenter.flush().is_some());
        assert!(segmenter.flush().is_none());
    }

    #[test]
    fn resampling_48k_stereo_gives_16k_mono_of_the_right_length() {
        let mut resampler = Resampler::new(48_000, 2);
        let input = vec![0.5_f32; 48_000 * 2];
        let out = resampler.push(&input);
        assert!((out.len() as i32 - 16_000).abs() <= 1, "{}", out.len());
        assert!(out.iter().all(|sample| (sample - 0.5).abs() < 1e-3));
    }

    #[test]
    fn resampling_an_awkward_rate_keeps_the_duration() {
        let mut resampler = Resampler::new(44_100, 1);
        let mut total = 0;
        for _ in 0..10 {
            total += resampler.push(&vec![0.1_f32; 4_410]).len();
        }
        assert!((total as i32 - 16_000).abs() <= 2, "{total}");
    }
}
