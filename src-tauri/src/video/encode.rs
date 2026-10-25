#![cfg_attr(not(windows), allow(dead_code))]

use std::path::Path;

use super::schedule::{Shown, video_size};

pub const MAX_WIDTH: u32 = 1920;

fn bitrate(width: u32, height: u32) -> u32 {
    (u64::from(width) * u64::from(height) * 6_000_000 / (1920 * 1080)).clamp(6_000_000, 80_000_000)
        as u32
}

pub fn encode(
    out: &Path,
    shown: &[Shown],
    cancelled: &dyn Fn() -> bool,
) -> Result<(u32, u32), EncodeError> {
    encode_with_limit(out, shown, cancelled, MAX_WIDTH)
}

pub fn encode_native(
    out: &Path,
    shown: &[Shown],
    cancelled: &dyn Fn() -> bool,
) -> Result<(u32, u32), EncodeError> {
    encode_with_limit(out, shown, cancelled, u32::MAX)
}

#[derive(Debug)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum EncodeError {
    Unavailable(String),
    Cancelled,
    Failed(String),
}

pub struct Bgra {
    pub data: Vec<u8>,
}

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
        pixel.swap(0, 2);
    }
    Ok(Bgra { data })
}

pub fn size_of_first(shown: &[Shown]) -> Option<(u32, u32)> {
    first_size(shown, MAX_WIDTH)
}

fn first_size(shown: &[Shown], max_width: u32) -> Option<(u32, u32)> {
    shown
        .iter()
        .find_map(|frame| image::image_dimensions(&frame.path).ok())
        .map(|(w, h)| video_size(w, h, max_width))
}

#[cfg(windows)]
fn pack(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

#[cfg(windows)]
fn encode_with_limit(
    out: &Path,
    shown: &[Shown],
    cancelled: &dyn Fn() -> bool,
    max_width: u32,
) -> Result<(u32, u32), EncodeError> {
    use windows::{
        Win32::{
            Media::MediaFoundation::*,
            System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
        },
        core::HSTRING,
    };

    let size = if max_width == MAX_WIDTH {
        size_of_first(shown)
    } else {
        first_size(shown, max_width)
    }
    .ok_or_else(|| EncodeError::Failed("no frame could be read".into()))?;
    let (width, height) = size;
    const TICKS_PER_MS: i64 = 10_000;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(|err| {
            EncodeError::Unavailable(format!("Media Foundation did not start: {err}"))
        })?;
        let result = (|| -> windows::core::Result<()> {
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(out.as_os_str()), None, None)?;

            let output = MFCreateMediaType()?;
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            output.SetUINT32(
                &MF_MT_AVG_BITRATE,
                if max_width == MAX_WIDTH {
                    6_000_000
                } else {
                    bitrate(width, height)
                },
            )?;
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
                EncodeError::Unavailable(format!("the H.264 encoder failed: {err}"))
            }
        })
    }
}

#[cfg(not(windows))]
fn encode_with_limit(
    _out: &Path,
    _shown: &[Shown],
    _cancelled: &dyn Fn() -> bool,
    _max_width: u32,
) -> Result<(u32, u32), EncodeError> {
    Err(EncodeError::Unavailable(
        "there is no video encoder on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a native Windows H.264 encoder; uses synthetic frames only"]
    fn native_4k_mp4_encoder_accepts_full_resolution_frames() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native.png");
        image::RgbImage::from_pixel(3840, 2160, image::Rgb([32, 80, 160]))
            .save(&path)
            .unwrap();
        let shown = [Shown {
            path: path.display().to_string(),
            hold_ms: 1000,
        }];
        let out = dir.path().join("native.mp4");
        assert_eq!(
            encode_native(&out, &shown, &|| false).unwrap(),
            (3840, 2160)
        );
        let bytes = std::fs::read(out).unwrap();
        assert!(bytes.windows(4).any(|tag| tag == b"avc1"));
        assert!(!bytes.windows(4).any(|tag| tag == b"mp4a"));
    }

    #[test]
    fn standalone_native_size_is_not_capped_and_bitrate_scales_with_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("4k.png");
        image::RgbImage::new(3840, 2160).save(&path).unwrap();
        let shown = [Shown {
            path: path.display().to_string(),
            hold_ms: 500,
        }];
        assert_eq!(size_of_first(&shown), Some((1920, 1080)));
        assert_eq!(first_size(&shown, u32::MAX), Some((3840, 2160)));
        assert_eq!(bitrate(3840, 2160), 24_000_000);
        assert_eq!(video_size(1001, 701, u32::MAX), (1000, 700));
    }

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
