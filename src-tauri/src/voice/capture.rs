use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::vad::{Resampler, Segmenter};

pub const AUDIO_LEVEL: &str = "audio_level";

const EMIT_EVERY: Duration = Duration::from_millis(33);
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

/// Level meter range: -55 dBFS (room noise) maps to 0 and -10 dBFS (loud speech) to 1.
const FLOOR_DB: f32 = -55.0;
const CEILING_DB: f32 = -10.0;

/// An open microphone stream. cpal streams are not `Send`, so a dedicated thread owns the
/// stream for its whole life; dropping this value stops the stream and joins the thread.
pub struct Capture {
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Capture {
    /// Opens the default microphone, emits `audio_level` (0..1) events and hands every spoken
    /// utterance to the voice pipeline (transcribe, interpret, act).
    pub fn start(app: AppHandle) -> Result<Self, String> {
        let (utterances, mut heard) = unbounded_channel::<Vec<i16>>();
        let pipeline = app.clone();
        tauri::async_runtime::spawn(async move {
            // Ends when the stream is dropped (the sender goes with it).
            while let Some(utterance) = heard.recv().await {
                super::handle(&pipeline, utterance).await;
            }
        });

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        let thread = thread::spawn(move || match open_stream(&app, utterances) {
            Ok(stream) => {
                let _ = ready_tx.send(Ok(()));
                let _ = stop_rx.recv();
                drop(stream);
                let _ = app.emit(AUDIO_LEVEL, 0.0_f32);
            }
            Err(err) => {
                let _ = ready_tx.send(Err(err));
            }
        });

        match ready_rx.recv_timeout(OPEN_TIMEOUT) {
            Ok(Ok(())) => Ok(Self {
                stop: Some(stop_tx),
                thread: Some(thread),
            }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err("The microphone took too long to open".into()),
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn open_stream(app: &AppHandle, utterances: UnboundedSender<Vec<i16>>) -> Result<cpal::Stream, String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("No microphone found")?;
    let supported = device
        .default_input_config()
        .map_err(|err| format!("Could not read the microphone format: {err}"))?;
    let config = supported.config();
    log::info!(
        "microphone open: {} Hz, {} ch, {:?}",
        config.sample_rate,
        config.channels,
        supported.sample_format()
    );

    let stream = match supported.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config, app, utterances),
        SampleFormat::I16 => build::<i16>(&device, config, app, utterances),
        SampleFormat::I32 => build::<i32>(&device, config, app, utterances),
        SampleFormat::U16 => build::<u16>(&device, config, app, utterances),
        other => Err(format!("Unsupported microphone sample format {other:?}")),
    }?;

    stream
        .play()
        .map_err(|err| format!("Could not start the microphone: {err}"))?;
    Ok(stream)
}

fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, app: &AppHandle, utterances: UnboundedSender<Vec<i16>>) -> Result<cpal::Stream, String>
where
    T: SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let app = app.clone();
    let mut sum_squares = 0.0_f64;
    let mut count = 0_u64;
    let mut last_emit = Instant::now();
    let mut resampler = Resampler::new(config.sample_rate, config.channels);
    let mut segmenter = Segmenter::default();

    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mut samples = Vec::with_capacity(data.len());
                for sample in data {
                    let value = f32::from_sample(*sample);
                    sum_squares += f64::from(value * value);
                    samples.push(value);
                }
                for utterance in segmenter.push(&resampler.push(&samples)) {
                    let _ = utterances.send(utterance);
                }
                count += data.len() as u64;

                if last_emit.elapsed() >= EMIT_EVERY && count > 0 {
                    let rms = (sum_squares / count as f64).sqrt() as f32;
                    let _ = app.emit(AUDIO_LEVEL, level_from_rms(rms));
                    sum_squares = 0.0;
                    count = 0;
                    last_emit = Instant::now();
                }
            },
            |err| log::warn!("microphone stream error: {err}"),
            None,
        )
        .map_err(|err| format!("Could not open the microphone: {err}"))
}

fn level_from_rms(rms: f32) -> f32 {
    let db = 20.0 * rms.max(1e-6).log10();
    ((db - FLOOR_DB) / (CEILING_DB - FLOOR_DB)).clamp(0.0, 1.0)
}
