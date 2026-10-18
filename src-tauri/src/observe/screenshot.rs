use std::io::Cursor;
use std::time::Instant;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage, codecs::jpeg::JpegEncoder, imageops::FilterType};
use xcap::Monitor as XMonitor;

use crate::model::coords::{Monitor, fit_within};

/// The longest edge a screenshot is downscaled to before it goes to the model.
/// About 2.7k image tokens at 16:9 (see ARCHITECTURE.md).
pub const MODEL_MAX_EDGE: u32 = 1280;
pub const MODEL_JPEG_QUALITY: u8 = 80;

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
}

/// One captured, downscaled monitor. Carries the monitor geometry it was taken from, so a point
/// the model picks in this image can always be mapped back to a real pixel.
pub struct Frame {
    pub monitor: Monitor,
    /// Size of the downscaled image (what the model sees).
    pub width: u32,
    pub height: u32,
    pub jpeg: Vec<u8>,
    pub elapsed_ms: u64,
    image: RgbImage,
}

impl Frame {
    /// `data:image/jpeg;base64,...`, the form rune-infer's chat endpoint takes. Used from M2.
    #[allow(dead_code)]
    pub fn data_uri(&self) -> String {
        format!("data:image/jpeg;base64,{}", BASE64.encode(&self.jpeg))
    }

    /// A smaller copy as a data URI, for showing in the UI.
    pub fn thumbnail_data_uri(&self, max_edge: u32) -> Result<String, ObserveError> {
        let (w, h) = fit_within(self.width, self.height, max_edge);
        let small = image::imageops::resize(&self.image, w, h, FilterType::Triangle);
        let jpeg = encode_jpeg(&small, 70)?;
        Ok(format!("data:image/jpeg;base64,{}", BASE64.encode(jpeg)))
    }

    /// The colour of the downscaled image at a model-space point (0..=1000 on both axes).
    /// Lets tests check what the model would actually see at a point.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn pixel_at_model(&self, x: f64, y: f64) -> Rgb<u8> {
        let px = ((x / 1000.0 * f64::from(self.width)) as u32).min(self.width - 1);
        let py = ((y / 1000.0 * f64::from(self.height)) as u32).min(self.height - 1);
        *self.image.get_pixel(px, py)
    }
}

fn describe(monitor: &XMonitor) -> Result<Monitor, ObserveError> {
    let read = |what: &str, err: xcap::XCapError| ObserveError::Display(format!("{what}: {err}"));
    Ok(Monitor {
        id: monitor.id().map_err(|e| read("id", e))?,
        x: monitor.x().map_err(|e| read("x", e))?,
        y: monitor.y().map_err(|e| read("y", e))?,
        width: monitor.width().map_err(|e| read("width", e))?,
        height: monitor.height().map_err(|e| read("height", e))?,
        scale_factor: monitor.scale_factor().map_err(|e| read("scale factor", e))?,
        primary: monitor.is_primary().map_err(|e| read("primary", e))?,
    })
}

fn all_monitors() -> Result<Vec<XMonitor>, ObserveError> {
    XMonitor::all().map_err(|err| ObserveError::Display(err.to_string()))
}

pub fn monitors() -> Result<Vec<Monitor>, ObserveError> {
    all_monitors()?.iter().map(describe).collect()
}

/// The monitor with this id, or the primary one (falling back to the first) when `id` is `None`.
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

/// Geometry of a monitor by id, or of the primary one when `id` is `None`.
pub fn monitor(id: Option<u32>) -> Result<Monitor, ObserveError> {
    resolve(id).map(|(_, monitor)| monitor)
}

/// The monitor a physical desktop pixel is on. Used from M2 to pick the monitor under the cursor.
#[allow(dead_code)]
pub fn monitor_at(px: i32, py: i32) -> Result<Monitor, ObserveError> {
    let monitor = XMonitor::from_point(px, py).map_err(|err| ObserveError::Display(err.to_string()))?;
    describe(&monitor)
}

/// What the model is sent: the screenshot downscaled to the model's budget, as JPEG.
pub struct Prepared {
    pub width: u32,
    pub height: u32,
    pub jpeg: Vec<u8>,
    image: RgbImage,
}

/// The one place a captured screen becomes a model-sized JPEG: downscale so the longer edge is at
/// most `max_edge`, then encode. The app and the M1.5 grounding probe both go through here, so the
/// probe measures exactly what the agent will send.
pub fn prepare(rgba: image::RgbaImage, max_edge: u32, quality: u8) -> Result<Prepared, ObserveError> {
    let (width, height) = fit_within(rgba.width(), rgba.height(), max_edge);
    let rgb = DynamicImage::ImageRgba8(rgba).into_rgb8();
    let image = if (width, height) == (rgb.width(), rgb.height()) {
        rgb
    } else {
        image::imageops::resize(&rgb, width, height, FilterType::Triangle)
    };
    let jpeg = encode_jpeg(&image, quality)?;
    Ok(Prepared { width, height, jpeg, image })
}

/// Captures a monitor and downscales it so the longer edge is at most `max_edge`.
pub fn capture(id: Option<u32>, max_edge: u32, quality: u8) -> Result<Frame, ObserveError> {
    let started = Instant::now();
    let (xmonitor, monitor) = resolve(id)?;
    let rgba = xmonitor.capture_image().map_err(|err| ObserveError::Capture(err.to_string()))?;
    let Prepared { width, height, jpeg, image } = prepare(rgba, max_edge, quality)?;
    Ok(Frame {
        monitor,
        width,
        height,
        jpeg,
        elapsed_ms: started.elapsed().as_millis() as u64,
        image,
    })
}

fn encode_jpeg(image: &ImageBuffer<Rgb<u8>, Vec<u8>>, quality: u8) -> Result<Vec<u8>, ObserveError> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image.clone())
        .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, quality))
        .map_err(|err| ObserveError::Encode(err.to_string()))?;
    Ok(bytes.into_inner())
}
