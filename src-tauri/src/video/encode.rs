//! Turns the frame list into an H.264 `session.mp4`. Windows only, through Media Foundation's
//! sink writer (part of Windows 11; **missing on "N" editions without the Media Feature Pack**, where
//! starting it fails and the caller records `video: unavailable`). No binary is bundled. Elsewhere
//! there is no encoder at all.
//!
//! **Not run here**: this was written against the `windows` crate's bindings and cross-checked for
//! types only; no Windows machine was available, so whether it produces a file WebView2 plays is
//! unmeasured (H.264 main profile in MP4 is what Media Foundation's encoder makes by default).

// The encoder itself exists only on Windows; the frame loading is shared and tested everywhere.
#![cfg_attr(not(windows), allow(dead_code))]

use std::path::Path;

use super::schedule::{Shown, video_size};

/// The video is at most this wide.
pub const MAX_WIDTH: u32 = 1280;

#[derive(Debug)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum EncodeError {
    /// No encoder on this machine (not Windows, or Media Foundation's H.264 encoder is missing).
    Unavailable(String),
    /// A new task started: the job stops and will be tried again when the PC is idle.
    Cancelled,
    Failed(String),
}

/// One decoded frame: BGRA, top row first.
pub struct Bgra {
    pub data: Vec<u8>,
}

/// Loads a screenshot as BGRA at exactly `size` (it is scaled if the file is another size).
pub fn load(path: &Path, size: (u32, u32)) -> Result<Bgra, String> {
    let image = image::open(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let resized = if (image.width(), image.height()) == size {
        image.into_rgba8()
    } else {
        image::imageops::resize(
            &image.into_rgba8(),
            size.0,
            size.1,
            image::imageops::FilterType::Triangle,
        )
    };
    let mut data = resized.into_raw();
    let (pixels, _) = data.as_chunks_mut::<4>();
    for pixel in pixels {
        pixel.swap(0, 2); // RGBA -> BGRA
    }
    Ok(Bgra { data })
}

/// The size to encode at, from the first frame that can be read.
pub fn size_of_first(shown: &[Shown]) -> Option<(u32, u32)> {
    shown
        .iter()
        .find_map(|frame| image::image_dimensions(&frame.path).ok())
        .map(|(w, h)| video_size(w, h, MAX_WIDTH))
}

/// Media Foundation packs a size or a ratio into one 64-bit attribute: high half first, low half second.
#[cfg(windows)]
fn pack(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

#[cfg(windows)]
pub fn encode(
    out: &Path,
    shown: &[Shown],
    cancelled: &dyn Fn() -> bool,
) -> Result<(u32, u32), EncodeError> {
    use windows::{
        Win32::{
            Media::MediaFoundation::*,
            System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
        },
        core::HSTRING,
    };

    let size =
        size_of_first(shown).ok_or_else(|| EncodeError::Failed("no frame could be read".into()))?;
    let (width, height) = size;
    // 100 nanosecond units, the unit Media Foundation times are in.
    const TICKS_PER_MS: i64 = 10_000;

    // SAFETY: plain Media Foundation calls on interfaces this function creates and drops; every buffer
    // is locked, filled within its length and unlocked again.
    unsafe {
        // Already initialised on this thread is fine (S_FALSE).
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(|err| {
            EncodeError::Unavailable(format!("Media Foundation did not start: {err}"))
        })?;
        let result = (|| -> windows::core::Result<()> {
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(out.as_os_str()), None, None)?;

            let output = MFCreateMediaType()?;
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            output.SetUINT32(&MF_MT_AVG_BITRATE, 1_500_000)?;
            output.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            output.SetUINT64(&MF_MT_FRAME_SIZE, pack(width, height))?;
            output.SetUINT64(&MF_MT_FRAME_RATE, pack(2, 1))?;
            output.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
            let stream = writer.AddStream(&output)?;

            let input = MFCreateMediaType()?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            input.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            input.SetUINT32(&MF_MT_DEFAULT_STRIDE, width * 4)?;
            input.SetUINT64(&MF_MT_FRAME_SIZE, pack(width, height))?;
            input.SetUINT64(&MF_MT_FRAME_RATE, pack(2, 1))?;
            input.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
            writer.SetInputMediaType(stream, &input, None)?;
            writer.BeginWriting()?;

            let mut at: i64 = 0;
            for frame in shown {
                if cancelled() {
                    return Err(windows::core::Error::from(
                        windows::Win32::Foundation::E_ABORT,
                    ));
                }
                // A frame that cannot be read is skipped; its time is given to the previous one by the hold.
                let Ok(bgra) = load(Path::new(&frame.path), size) else {
                    continue;
                };
                let length = bgra.data.len() as u32;
                let buffer = MFCreateMemoryBuffer(length)?;
                let mut target = std::ptr::null_mut();
                buffer.Lock(&mut target, None, None)?;
                std::ptr::copy_nonoverlapping(bgra.data.as_ptr(), target, bgra.data.len());
                buffer.Unlock()?;
                buffer.SetCurrentLength(length)?;
                let sample = MFCreateSample()?;
                sample.AddBuffer(&buffer)?;
                let duration = frame.hold_ms as i64 * TICKS_PER_MS;
                sample.SetSampleTime(at)?;
                sample.SetSampleDuration(duration)?;
                writer.WriteSample(stream, &sample)?;
                at += duration;
            }
            writer.Finalize()?;
            Ok(())
        })();
        let _ = MFShutdown();
        result.map(|()| size).map_err(|err| {
            if err.code() == windows::Win32::Foundation::E_ABORT {
                EncodeError::Cancelled
            } else {
                // 0xC00D36B4 and friends: no H.264 encoder (an "N" edition without the Media Feature Pack).
                EncodeError::Unavailable(format!("the H.264 encoder failed: {err}"))
            }
        })
    }
}

#[cfg(not(windows))]
pub fn encode(
    _out: &Path,
    _shown: &[Shown],
    _cancelled: &dyn Fn() -> bool,
) -> Result<(u32, u32), EncodeError> {
    Err(EncodeError::Unavailable(
        "there is no video encoder on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_loaded_as_bgra_at_the_asked_size() {
        let dir = std::env::temp_dir().join(format!("rune-video-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.png");
        image::RgbaImage::from_pixel(4, 2, image::Rgba([10, 20, 30, 255]))
            .save(&path)
            .unwrap();
        let same = load(&path, (4, 2)).unwrap();
        assert_eq!(&same.data[..4], &[30, 20, 10, 255], "red and blue swapped");
        assert_eq!(same.data.len(), 4 * 2 * 4);
        let bigger = load(&path, (8, 4)).unwrap();
        assert_eq!(bigger.data.len(), 8 * 4 * 4);
        assert_eq!(
            size_of_first(&[
                Shown {
                    path: "missing.png".into(),
                    hold_ms: 1
                },
                Shown {
                    path: path.display().to_string(),
                    hold_ms: 1
                }
            ]),
            Some((4, 2)),
            "the first readable frame sets the size"
        );
        assert!(load(&dir.join("missing.png"), (4, 2)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(windows))]
    #[test]
    fn without_an_encoder_the_result_is_unavailable_not_a_crash() {
        let result = encode(Path::new("x.mp4"), &[], &|| false);
        assert!(matches!(result, Err(EncodeError::Unavailable(_))));
    }
}
