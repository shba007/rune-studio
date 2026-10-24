use std::io::Cursor;
use std::time::Instant;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use image::{
    DynamicImage, ImageBuffer, Rgb, RgbImage, codecs::jpeg::JpegEncoder, imageops::FilterType,
};
use xcap::Monitor as XMonitor;

use crate::model::coords::{Monitor, fit_within};

pub const MODEL_MAX_EDGE: u32 = 1280;
pub const MODEL_JPEG_QUALITY: u8 = 80;
const ARCHIVE_MAX_EDGE: u32 = 2560;
const ARCHIVE_JPEG_QUALITY: u8 = 90;

#[derive(Debug, thiserror::Error)]
pub enum ObserveError {
    #[error("no monitor found")]
    NoMonitor,
    #[error("monitor {0} not found")]
    UnknownMonitor(u32),
    #[error("could not read the display: {0}")]
    Display(String),
    #[error("screen capture failed: {0}")]
    Capture(String),
    #[error("could not encode the screenshot: {0}")]
    Encode(String),
    #[error("screenshot not used because it could not be stored: {0}")]
    Archive(String),
}

pub struct Frame {
    pub monitor: Monitor,
    pub width: u32,
    pub height: u32,
    pub jpeg: Vec<u8>,
    pub path: std::path::PathBuf,
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub elapsed_ms: u64,
    image: RgbImage,
}

impl Frame {
    #[allow(dead_code)]
    pub fn data_uri(&self) -> String {
        format!("data:image/jpeg;base64,{}", BASE64.encode(&self.jpeg))
    }

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub fn thumbnail_data_uri(&self, max_edge: u32) -> Result<String, ObserveError> {
        let (w, h) = fit_within(self.width, self.height, max_edge);
        let small = image::imageops::resize(&self.image, w, h, FilterType::Triangle);
        let jpeg = encode_jpeg(&small, 70)?;
        Ok(format!("data:image/jpeg;base64,{}", BASE64.encode(jpeg)))
    }

    pub fn signature(&self) -> Vec<u8> {
        signature_of(&self.image)
    }

    #[cfg(all(test, windows))]
    pub fn pixel_at_model(&self, x: f64, y: f64) -> Rgb<u8> {
        let px = ((x / 1000.0 * f64::from(self.width)) as u32).min(self.width - 1);
        let py = ((y / 1000.0 * f64::from(self.height)) as u32).min(self.height - 1);
        *self.image.get_pixel(px, py)
    }
}

pub fn signature_of(image: &RgbImage) -> Vec<u8> {
    let small = image::imageops::resize(
        image,
        crate::agent::stuck::SIGNATURE_SIZE.0,
        crate::agent::stuck::SIGNATURE_SIZE.1,
        FilterType::Triangle,
    );
    small
        .pixels()
        .map(|pixel| {
            ((u32::from(pixel[0]) * 3 + u32::from(pixel[1]) * 6 + u32::from(pixel[2])) / 10) as u8
        })
        .collect()
}

pub fn live_jpeg(max_edge: u32, quality: u8) -> Result<Vec<u8>, ObserveError> {
    let (xmonitor, monitor) = resolve(None)?;
    let mut rgba = xmonitor
        .capture_image()
        .map_err(|err| ObserveError::Capture(err.to_string()))?;
    let masked = super::mask::current();
    if !masked.is_empty() {
        super::mask::apply(&mut rgba, &masked, monitor.x, monitor.y);
    }
    Ok(prepare(rgba, max_edge, quality)?.jpeg)
}

pub fn live_signature() -> Result<Vec<u8>, ObserveError> {
    let (xmonitor, monitor) = resolve(None)?;
    let mut rgba = xmonitor
        .capture_image()
        .map_err(|err| ObserveError::Capture(err.to_string()))?;
    let masked = super::mask::current();
    if !masked.is_empty() {
        super::mask::apply(&mut rgba, &masked, monitor.x, monitor.y);
    }
    Ok(signature_of(&DynamicImage::ImageRgba8(rgba).into_rgb8()))
}

fn describe(monitor: &XMonitor) -> Result<Monitor, ObserveError> {
    let read = |what: &str, err: xcap::XCapError| ObserveError::Display(format!("{what}: {err}"));
    Ok(Monitor {
        id: monitor.id().map_err(|e| read("id", e))?,
        x: monitor.x().map_err(|e| read("x", e))?,
        y: monitor.y().map_err(|e| read("y", e))?,
        width: monitor.width().map_err(|e| read("width", e))?,
        height: monitor.height().map_err(|e| read("height", e))?,
        scale_factor: monitor
            .scale_factor()
            .map_err(|e| read("scale factor", e))?,
        primary: monitor.is_primary().map_err(|e| read("primary", e))?,
    })
}

fn all_monitors() -> Result<Vec<XMonitor>, ObserveError> {
    XMonitor::all().map_err(|err| ObserveError::Display(err.to_string()))
}

#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub fn monitors() -> Result<Vec<Monitor>, ObserveError> {
    all_monitors()?.iter().map(describe).collect()
}

fn resolve(id: Option<u32>) -> Result<(XMonitor, Monitor), ObserveError> {
    let mut found = None;
    for candidate in all_monitors()? {
        let described = describe(&candidate)?;
        match id {
            Some(wanted) if described.id == wanted => return Ok((candidate, described)),
            Some(_) => {}
            None if described.primary => return Ok((candidate, described)),
            None if found.is_none() => found = Some((candidate, described)),
            None => {}
        }
    }
    match (id, found) {
        (Some(wanted), _) => Err(ObserveError::UnknownMonitor(wanted)),
        (None, Some(first)) => Ok(first),
        (None, None) => Err(ObserveError::NoMonitor),
    }
}

pub fn monitor(id: Option<u32>) -> Result<Monitor, ObserveError> {
    resolve(id).map(|(_, monitor)| monitor)
}

#[allow(dead_code)]
pub fn monitor_at(px: i32, py: i32) -> Result<Monitor, ObserveError> {
    let monitor =
        XMonitor::from_point(px, py).map_err(|err| ObserveError::Display(err.to_string()))?;
    describe(&monitor)
}

pub struct Prepared {
    pub width: u32,
    pub height: u32,
    pub jpeg: Vec<u8>,
    image: RgbImage,
}

pub fn prepare(
    rgba: image::RgbaImage,
    max_edge: u32,
    quality: u8,
) -> Result<Prepared, ObserveError> {
    let (width, height) = fit_within(rgba.width(), rgba.height(), max_edge);
    let rgb = DynamicImage::ImageRgba8(rgba).into_rgb8();
    let image = if (width, height) == (rgb.width(), rgb.height()) {
        rgb
    } else {
        image::imageops::resize(&rgb, width, height, FilterType::Triangle)
    };
    let jpeg = encode_jpeg(&image, quality)?;
    Ok(Prepared {
        width,
        height,
        jpeg,
        image,
    })
}

pub fn capture(id: Option<u32>, max_edge: u32, quality: u8) -> Result<Frame, ObserveError> {
    capture_region(id, None, max_edge, quality)
}

const MIN_REGION_PX: u32 = 16;

pub fn capture_region(
    id: Option<u32>,
    region: Option<[f64; 4]>,
    max_edge: u32,
    quality: u8,
) -> Result<Frame, ObserveError> {
    let started = Instant::now();
    let (xmonitor, monitor) = resolve(id)?;
    let mut rgba = xmonitor
        .capture_image()
        .map_err(|err| ObserveError::Capture(err.to_string()))?;
    let masked = super::mask::current();
    if !masked.is_empty() {
        super::mask::apply(&mut rgba, &masked, monitor.x, monitor.y);
    }
    if let Some([x0, y0, x1, y1]) = region {
        let (w, h) = (f64::from(rgba.width()), f64::from(rgba.height()));
        let to_px =
            |value: f64, extent: f64| (value.clamp(0.0, 1000.0) / 1000.0 * extent).round() as u32;
        let (left, right) = (to_px(x0.min(x1), w), to_px(x0.max(x1), w));
        let (top, bottom) = (to_px(y0.min(y1), h), to_px(y0.max(y1), h));
        if right.saturating_sub(left) < MIN_REGION_PX || bottom.saturating_sub(top) < MIN_REGION_PX
        {
            return Err(ObserveError::Capture(
                "the region is too small to show".into(),
            ));
        }
        rgba = image::imageops::crop_imm(&rgba, left, top, right - left, bottom - top).to_image();
    }
    let Prepared {
        width,
        height,
        jpeg,
        image,
    } = prepare(rgba.clone(), max_edge, quality)?;
    let sharp = prepare(rgba, max_edge.max(ARCHIVE_MAX_EDGE), ARCHIVE_JPEG_QUALITY)?;
    let path = crate::archive::save(
        crate::archive::Kind::Screenshots,
        if region.is_some() { "region" } else { "screen" },
        "jpg",
        &sharp.jpeg,
    )
    .map_err(|err| ObserveError::Archive(err.to_string()))?;
    Ok(Frame {
        monitor,
        width,
        height,
        jpeg,
        path,
        elapsed_ms: started.elapsed().as_millis() as u64,
        image,
    })
}

fn encode_jpeg(
    image: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    quality: u8,
) -> Result<Vec<u8>, ObserveError> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image.clone())
        .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, quality))
        .map_err(|err| ObserveError::Encode(err.to_string()))?;
    Ok(bytes.into_inner())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum CaptureTarget {
    Screen { id: u32 },
    Window { id: u32 },
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetOption {
    pub target: CaptureTarget,
    pub label: String,
}
pub fn insight_targets() -> Result<Vec<TargetOption>, ObserveError> {
    let mut targets = monitors()?
        .into_iter()
        .map(|m| TargetOption {
            target: CaptureTarget::Screen { id: m.id },
            label: format!(
                "Screen {} — {}×{}{}",
                m.id,
                m.width,
                m.height,
                if m.primary { " (primary)" } else { "" }
            ),
        })
        .collect::<Vec<_>>();
    for window in xcap::Window::all().map_err(|e| ObserveError::Display(e.to_string()))? {
        if window.pid().ok() == Some(std::process::id()) || window.is_minimized().unwrap_or(true) {
            continue;
        }
        if let (Ok(id), Ok(title)) = (window.id(), window.title())
            && !title.trim().is_empty()
        {
            targets.push(TargetOption {
                target: CaptureTarget::Window { id },
                label: format!("Window — {}", title.chars().take(120).collect::<String>()),
            });
        }
    }
    Ok(targets)
}
pub fn insight_capture(target: CaptureTarget) -> Result<Frame, ObserveError> {
    let started = Instant::now();
    let (mut rgba, monitor) = match target {
        CaptureTarget::Screen { id } => {
            let (screen, monitor) = resolve(Some(id))?;
            (
                screen
                    .capture_image()
                    .map_err(|e| ObserveError::Capture(e.to_string()))?,
                monitor,
            )
        }
        CaptureTarget::Window { id } => {
            let window = xcap::Window::all()
                .map_err(|e| ObserveError::Display(e.to_string()))?
                .into_iter()
                .find(|w| w.id().ok() == Some(id))
                .ok_or_else(|| {
                    ObserveError::Capture("Selected window is no longer available".into())
                })?;
            if window.pid().ok() == Some(std::process::id())
                || window.is_minimized().unwrap_or(true)
            {
                return Err(ObserveError::Capture(
                    "Selected window is unavailable for capture".into(),
                ));
            }
            let rgba = window
                .capture_image()
                .map_err(|e| ObserveError::Capture(e.to_string()))?;
            let monitor = Monitor {
                id,
                x: window.x().unwrap_or(0),
                y: window.y().unwrap_or(0),
                width: rgba.width(),
                height: rgba.height(),
                scale_factor: 1.0,
                primary: false,
            };
            (rgba, monitor)
        }
    };
    super::mask::apply(&mut rgba, &super::mask::current(), monitor.x, monitor.y);
    let prepared = prepare(rgba, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY)?;
    Ok(Frame {
        monitor,
        width: prepared.width,
        height: prepared.height,
        jpeg: prepared.jpeg,
        path: Default::default(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        image: prepared.image,
    })
}
