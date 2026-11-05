use base64::Engine;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_RAW_BYTES: usize = 150 * 1024 * 1024;
const MAX_RAW_PIXELS: u64 = 80_000_000;
static DEVELOPING: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopedRaw {
    path: String,
    width: u32,
    height: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CompositeInput {
    raw: String,
    model: String,
    settings: FinishSettings,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopedPreviewInput {
    raw: String,
}

#[tauri::command]
pub async fn portfolio_developed_preview(
    app: tauri::AppHandle,
    run_id: u64,
    input: DevelopedPreviewInput,
) -> Result<String, String> {
    use tauri::Manager;
    let runs = app.state::<crate::playground::PlaygroundRuns>();
    runs.run(run_id, async move {
        if input.raw.len() > MAX_RAW_BYTES.div_ceil(3) * 4 + 4 {
            return Err("RAW file exceeds the 150 MB limit".into());
        }
        tauri::async_runtime::spawn_blocking(move || {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(input.raw)
                .map_err(|_| "Could not read RAW data")?;
            if bytes.is_empty() || bytes.len() > MAX_RAW_BYTES {
                return Err("RAW file is empty or exceeds the 150 MB limit".into());
            }
            developed_preview(bytes)
        })
        .await
        .map_err(|_| "Could not develop preview".to_string())?
    })
    .await
}

fn developed_preview(bytes: Vec<u8>) -> Result<String, String> {
    let source = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| develop_raw(bytes)))
        .map_err(|_| "RAW development failed unexpectedly")??;
    let preview = image::DynamicImage::ImageRgb16(source)
        .resize(768, 768, image::imageops::FilterType::Lanczos3)
        .into_rgb8();
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(preview)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| "Could not encode developed preview")?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(output.into_inner())
    ))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookSample {
    original: String,
    cutout: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedLook {
    exposure: i16,
    warmth: i16,
}

#[tauri::command]
pub async fn portfolio_suggest_look(samples: Vec<LookSample>) -> Result<SuggestedLook, String> {
    tauri::async_runtime::spawn_blocking(move || suggest_look(&samples))
        .await
        .map_err(|_| "Could not estimate the shoot look".to_string())?
}

fn suggest_look(samples: &[LookSample]) -> Result<SuggestedLook, String> {
    if samples.is_empty() || samples.len() > 5 {
        return Err("Choose one to five photos to estimate the look".into());
    }
    let mut luminance = [0_u64; 256];
    let mut maximum = [0_u64; 256];
    let mut count = 0_u64;
    let (mut neutral_r, mut neutral_b, mut neutral_count) = (0_u64, 0_u64, 0_u64);
    for sample in samples {
        let decode = |uri: &str| -> Result<image::DynamicImage, String> {
            let bytes = crate::playground::portfolio_image(uri)?;
            let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .map_err(|_| "Invalid look sample")?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(768);
            limits.max_image_height = Some(768);
            limits.max_alloc = Some(32 * 1024 * 1024);
            reader.limits(limits);
            reader.decode().map_err(|_| "Invalid look sample".into())
        };
        let rgb = decode(&sample.original)?.into_rgb8();
        let mask = decode(&sample.cutout)?;
        if !mask.color().has_alpha() || mask.width() != rgb.width() || mask.height() != rgb.height()
        {
            return Err("Look sample and matte do not match".into());
        }
        let mask = mask.into_rgba8();
        for (pixel, alpha) in rgb.pixels().zip(mask.pixels()) {
            if alpha.0[3] < 240 {
                continue;
            }
            let [r, g, b] = pixel.0;
            let l = (u32::from(r) * 2126 + u32::from(g) * 7152 + u32::from(b) * 722 + 5000) / 10000;
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            luminance[l as usize] += 1;
            maximum[max as usize] += 1;
            count += 1;
            if l >= 120 && u32::from(max - min) * 100 <= u32::from(max) * 18 {
                neutral_r += u64::from(r);
                neutral_b += u64::from(b);
                neutral_count += 1;
            }
        }
    }
    if count < 200 {
        return Err("Not enough visible subject pixels to estimate the look".into());
    }
    let percentile = |hist: &[u64; 256], percent: u64| {
        let goal = (count * percent).div_ceil(100);
        let mut total = 0;
        for (value, n) in hist.iter().enumerate() {
            total += n;
            if total >= goal {
                return value as f32;
            }
        }
        255.0
    };
    let p95 = percentile(&luminance, 95);
    let p99 = percentile(&maximum, 99);
    if p95 < 10.0 {
        return Err("The subject is too dark to estimate the look safely".into());
    }
    let gain = (180.0 / p95).min(235.0 / p99.max(1.0)).clamp(0.8, 1.5);
    let exposure = ((gain.log2() * 100.0 / 5.0).round() * 5.0).clamp(-100.0, 100.0) as i16;
    let warmth = if neutral_count >= 128 && neutral_r + neutral_b > 0 {
        let offset = (neutral_b as f64 - neutral_r as f64) / (neutral_b + neutral_r) as f64;
        ((offset * 100.0 / 0.15 / 5.0).round() * 5.0).clamp(-20.0, 20.0) as i16
    } else {
        0
    };
    Ok(SuggestedLook { exposure, warmth })
}

#[tauri::command]
pub async fn portfolio_export_composite(
    app: tauri::AppHandle,
    run_id: u64,
    input: CompositeInput,
) -> Result<Option<DevelopedRaw>, String> {
    use tauri::Manager;
    let runs = app.state::<crate::playground::PlaygroundRuns>();
    let token = runs.token(run_id).ok_or("Export was cancelled")?;
    runs.run(run_id, export_composite(&app, input, token)).await
}

async fn export_composite(
    app: &tauri::AppHandle,
    input: CompositeInput,
    cancel: crate::exec::CancelToken,
) -> Result<Option<DevelopedRaw>, String> {
    use tauri::Manager;
    if input.raw.len() > MAX_RAW_BYTES.div_ceil(3) * 4 + 4 {
        return Err("RAW file exceeds the 150 MB limit".into());
    }
    validate_finish_settings(&input.settings)?;
    let (settings, key, model, caps) =
        crate::playground::session(app, &input.model, "image-to-matting", None).await?;
    if caps.locality != crate::model::capabilities::Locality::Local
        || !caps.portrait_matting_verified
    {
        return Err(
            "Choose a local verified portrait matting model for full-resolution export".into(),
        );
    }
    let url = crate::playground::infer_url(&settings, "/v1/images/portrait-mattings")?;
    let host = url.host_str().unwrap_or_default().trim_matches(['[', ']']);
    if host != "localhost"
        && !host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        return Err("Full-resolution export requires a local Infer server".into());
    }
    #[cfg(windows)]
    let owner = app
        .get_webview_window("playground")
        .and_then(|window| window.hwnd().ok())
        .map(|hwnd| hwnd.0 as isize)
        .unwrap_or(0);
    #[cfg(not(windows))]
    let owner = 0_isize;
    let destination = tauri::async_runtime::spawn_blocking(move || choose_developed_path(owner))
        .await
        .map_err(|_| "Could not choose an output file")??;
    let Some(destination) = destination else {
        return Ok(None);
    };
    if !destination
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        return Err("Choose a PNG filename".into());
    }
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(input.raw)
            .map_err(|_| "The RAW data is not valid")?;
        if bytes.is_empty() || bytes.len() > MAX_RAW_BYTES {
            return Err("RAW file is empty or exceeds the 150 MB limit".to_string());
        }
        let rgb = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| develop_raw(bytes)))
            .map_err(|_| "RAW development failed unexpectedly")??;
        if rgb.width() > 8192
            || rgb.height() > 8192
            || u64::from(rgb.width()) * u64::from(rgb.height()) > 40_000_000
        {
            return Err("The developed image exceeds Infer's portrait matting limits".into());
        }
        let image = image::DynamicImage::ImageRgb16(rgb.clone()).into_rgb8();
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
            .encode_image(&image::DynamicImage::ImageRgb8(image))
            .map_err(|_| "Could not prepare matting image")?;
        if jpeg.len() > 32 * 1024 * 1024 {
            return Err("The matting image exceeds Infer's 32 MiB limit".into());
        }
        Ok::<_, String>((rgb, jpeg, input.settings))
    })
    .await
    .map_err(|_| "RAW development task failed")??;
    let (rgb, jpeg, finish_settings) = prepared;
    if cancel.is_cancelled() {
        return Err("Export cancelled".into());
    }
    let image = format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(jpeg)
    );
    let response = crate::playground::post_json(
        &app.state::<crate::events::AppState>().http,
        key.as_deref(),
        url.clone(),
        serde_json::json!({"model":model,"image":image}),
        "portrait matting",
    )
    .await?;
    let (width, height) = rgb.dimensions();
    let alpha = crate::playground::decode_matte_alpha(&response, width, height)?;
    let alpha = if caps
        .portrait_matting_input_width
        .zip(caps.portrait_matting_input_height)
        .is_some_and(|(w, h)| w < 2880 || h < 2880)
    {
        refine_matte(
            &app.state::<crate::events::AppState>().http,
            key.as_deref(),
            url,
            &model,
            &rgb,
            alpha,
            &cancel,
        )
        .await?
    } else {
        alpha
    };
    tauri::async_runtime::spawn_blocking(move || {
        let composite = render_full_size(&rgb, &alpha, &finish_settings, &cancel)?;
        let (width, height) = composite.dimensions();
        write_png_inner(&composite, &destination, Some(&cancel))?;
        Ok(Some(DevelopedRaw {
            path: destination.to_string_lossy().into_owned(),
            width,
            height,
        }))
    })
    .await
    .map_err(|_| "Composite export task failed".to_string())?
}

fn tile_starts(length: u32) -> Vec<u32> {
    const TILE: u32 = 2880;
    const STRIDE: u32 = 2000;
    if length <= TILE {
        return vec![0];
    }
    let remaining = length - TILE;
    let steps = remaining.div_ceil(STRIDE);
    (0..=steps)
        .map(|i| {
            ((u64::from(i) * u64::from(remaining) + u64::from(steps) / 2) / u64::from(steps)) as u32
        })
        .collect()
}

fn edge_band(alpha: &image::GrayImage, x0: u32, y0: u32, width: u32, height: u32) -> Vec<u8> {
    const RADIUS: usize = 64;
    let (w, h) = (width as usize, height as usize);
    let mut horizontal = vec![0_u8; w * h];
    for y in 0..h {
        let mut count = 0_u32;
        for x in 0..w {
            if x == 0 {
                for dx in 0..=RADIUS.min(w - 1) {
                    let a = alpha.get_pixel(x0 + dx as u32, y0 + y as u32).0[0];
                    count += u32::from(a > 5 && a < 250);
                }
            } else {
                if x > RADIUS {
                    let a = alpha
                        .get_pixel(x0 + (x - RADIUS - 1) as u32, y0 + y as u32)
                        .0[0];
                    count -= u32::from(a > 5 && a < 250);
                }
                if x + RADIUS < w {
                    let a = alpha.get_pixel(x0 + (x + RADIUS) as u32, y0 + y as u32).0[0];
                    count += u32::from(a > 5 && a < 250);
                }
            }
            horizontal[y * w + x] = u8::from(count > 0);
        }
    }
    let mut band = vec![0_u8; w * h];
    for x in 0..w {
        let mut count = 0_u32;
        for y in 0..h {
            if y == 0 {
                for dy in 0..=RADIUS.min(h - 1) {
                    count += u32::from(horizontal[dy * w + x]);
                }
            } else {
                if y > RADIUS {
                    count -= u32::from(horizontal[(y - RADIUS - 1) * w + x]);
                }
                if y + RADIUS < h {
                    count += u32::from(horizontal[(y + RADIUS) * w + x]);
                }
            }
            band[y * w + x] = u8::from(count > 0);
        }
    }
    band
}

fn blend_matte_tile(
    target: &mut image::GrayImage,
    detail: &image::GrayImage,
    band: &[u8],
    x0: u32,
    y0: u32,
) -> Result<(), String> {
    let (width, height) = detail.dimensions();
    if x0 + width > target.width()
        || y0 + height > target.height()
        || band.len() != width as usize * height as usize
    {
        return Err("Invalid matte refinement tile".into());
    }
    for y in 0..height {
        for x in 0..width {
            if band[y as usize * width as usize + x as usize] == 0 {
                continue;
            }
            let dx = (if x0 == 0 { 256 } else { x }).min(if x0 + width == target.width() {
                256
            } else {
                width - 1 - x
            });
            let dy = (if y0 == 0 { 256 } else { y }).min(if y0 + height == target.height() {
                256
            } else {
                height - 1 - y
            });
            let weight = dx.min(dy).min(256);
            let original = u32::from(target.get_pixel(x0 + x, y0 + y).0[0]);
            let refined = u32::from(detail.get_pixel(x, y).0[0]);
            target.put_pixel(
                x0 + x,
                y0 + y,
                image::Luma([((original * (256 - weight) + refined * weight + 128) / 256) as u8]),
            );
        }
    }
    Ok(())
}

async fn refine_matte(
    http: &reqwest::Client,
    key: Option<&str>,
    url: reqwest::Url,
    model: &str,
    source: &image::ImageBuffer<image::Rgb<u16>, Vec<u16>>,
    mut alpha: image::GrayImage,
    cancel: &crate::exec::CancelToken,
) -> Result<image::GrayImage, String> {
    let (width, height) = source.dimensions();
    if width <= 2880 && height <= 2880 {
        return Ok(alpha);
    }
    let base = alpha.clone();
    for y0 in tile_starts(height) {
        for x0 in tile_starts(width) {
            if cancel.is_cancelled() {
                return Err("Export cancelled".into());
            }
            let tw = (width - x0).min(2880);
            let th = (height - y0).min(2880);
            let band = edge_band(&base, x0, y0, tw, th);
            if !band.contains(&1) {
                continue;
            }
            let crop = image::imageops::crop_imm(source, x0, y0, tw, th).to_image();
            let crop = image::DynamicImage::ImageRgb16(crop).into_rgb8();
            let mut jpeg = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
                .encode_image(&image::DynamicImage::ImageRgb8(crop))
                .map_err(|_| "Could not prepare matte refinement tile")?;
            let image = format!(
                "data:image/jpeg;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(jpeg)
            );
            let response = crate::playground::post_json(
                http,
                key,
                url.clone(),
                serde_json::json!({"model":model,"image":image}),
                "matte refinement",
            )
            .await?;
            let detail = crate::playground::decode_matte_alpha(&response, tw, th)?;
            blend_matte_tile(&mut alpha, &detail, &band, x0, y0)?;
        }
    }
    Ok(alpha)
}

fn composite_full_size(
    mut source: image::ImageBuffer<image::Rgb<u16>, Vec<u16>>,
    alpha: &image::GrayImage,
    left: u8,
    right: u8,
    cancel: &crate::exec::CancelToken,
) -> Result<image::ImageBuffer<image::Rgb<u16>, Vec<u16>>, String> {
    if source.dimensions() != alpha.dimensions() {
        return Err("The matte does not match the developed image".into());
    }
    let width = source.width();
    let mut opaque = 0_u64;
    let mut visible = 0_u64;
    let mut transparent = false;
    for (x, y, pixel) in source.enumerate_pixels_mut() {
        if x == 0 && cancel.is_cancelled() {
            return Err("Export cancelled".into());
        }
        let a = u32::from(alpha.get_pixel(x, y).0[0]);
        transparent |= a < 16;
        visible += u64::from(a > 15);
        opaque += u64::from(a > 239);
        let bg = ((u32::from(left) * (width - 1 - x) + u32::from(right) * x) * 257)
            / width.saturating_sub(1).max(1);
        for channel in &mut pixel.0 {
            *channel = ((u32::from(*channel) * a + bg * (255 - a) + 127) / 255) as u16;
        }
    }
    if !transparent || opaque == 0 || opaque * 4 < visible {
        return Err("Infer returned an unusable or mostly transparent matte".into());
    }
    Ok(source)
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FinishSettings {
    crop: String,
    #[serde(default)]
    auto_center: bool,
    scale: u16,
    horizontal: i16,
    vertical: i16,
    exposure: i16,
    warmth: i16,
    background_left: u8,
    background_right: u8,
}

fn horizontal_offset(
    alpha: &image::GrayImage,
    output_width: u32,
    scaled_width: u32,
    center: bool,
    movement: i16,
) -> i64 {
    let base = if center {
        let mut min_x = alpha.width();
        let mut max_x = 0;
        let mut found = false;
        for (x, _, pixel) in alpha.enumerate_pixels() {
            if pixel.0[0] >= 128 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                found = true;
            }
        }
        if found {
            let subject_center = (u64::from(min_x) + u64::from(max_x) + 1)
                * u64::from(scaled_width)
                / (2 * u64::from(alpha.width()));
            i64::from(output_width) / 2 - subject_center as i64
        } else {
            (i64::from(output_width) - i64::from(scaled_width)) / 2
        }
    } else {
        (i64::from(output_width) - i64::from(scaled_width)) / 2
    };
    base + i64::from(output_width) * i64::from(movement) / 100
}

fn validate_finish_settings(settings: &FinishSettings) -> Result<(), String> {
    if !matches!(settings.crop.as_str(), "original" | "portrait" | "square")
        || !(50..=150).contains(&settings.scale)
        || !(-50..=50).contains(&settings.horizontal)
        || !(-50..=50).contains(&settings.vertical)
        || !(-100..=100).contains(&settings.exposure)
        || !(-100..=100).contains(&settings.warmth)
    {
        return Err("Invalid finish settings".into());
    }
    Ok(())
}

fn render_full_size(
    source: &image::ImageBuffer<image::Rgb<u16>, Vec<u16>>,
    alpha: &image::GrayImage,
    settings: &FinishSettings,
    cancel: &crate::exec::CancelToken,
) -> Result<image::ImageBuffer<image::Rgb<u16>, Vec<u16>>, String> {
    validate_finish_settings(settings)?;
    let (sw, sh) = source.dimensions();
    if sw == 0 || sh == 0 || alpha.dimensions() != (sw, sh) {
        return Err("The matte does not match the developed image".into());
    }
    if settings.crop == "original"
        && !settings.auto_center
        && settings.scale == 100
        && settings.horizontal == 0
        && settings.vertical == 0
        && settings.exposure == 0
        && settings.warmth == 0
    {
        return composite_full_size(
            source.clone(),
            alpha,
            settings.background_left,
            settings.background_right,
            cancel,
        );
    }
    let (width, height) = match settings.crop.as_str() {
        "original" => (sw, sh),
        "portrait" => {
            let width = sw.min(sh.saturating_mul(2) / 3).max(1);
            (width, (width * 3 / 2).max(1))
        }
        "square" => {
            let side = sw.min(sh);
            (side, side)
        }
        _ => return Err("Invalid crop".into()),
    };
    let factor = (f64::from(width) / f64::from(sw)).max(f64::from(height) / f64::from(sh))
        * f64::from(settings.scale)
        / 100.0;
    let rw = (f64::from(sw) * factor).round().max(1.0) as u32;
    let rh = (f64::from(sh) * factor).round().max(1.0) as u32;
    let left = horizontal_offset(alpha, width, rw, settings.auto_center, settings.horizontal);
    let top = (i64::from(height) - i64::from(rh)) / 2
        + i64::from(height) * i64::from(settings.vertical) / 100;
    let mut opaque = 0_u64;
    let mut visible = 0_u64;
    let mut transparent = false;
    for pixel in alpha.pixels() {
        let a = pixel.0[0];
        transparent |= a < 16;
        visible += u64::from(a > 15);
        opaque += u64::from(a > 239);
    }
    if !transparent || opaque == 0 || opaque * 4 < visible {
        return Err("Infer returned an unusable or mostly transparent matte".into());
    }
    let gain = 2_f32.powf(f32::from(settings.exposure) / 100.0);
    let warm = f32::from(settings.warmth) / 100.0 * 0.15;
    let adjustment = [1.0 + warm, 1.0, 1.0 - warm];
    let mut result = image::ImageBuffer::new(width, height);
    for (x, y, pixel) in result.enumerate_pixels_mut() {
        if x == 0 && cancel.is_cancelled() {
            return Err("Export cancelled".into());
        }
        let t = x as f32 / width.saturating_sub(1).max(1) as f32;
        let bg = (f32::from(settings.background_left) * (1.0 - t)
            + f32::from(settings.background_right) * t)
            * 257.0;
        let px = i64::from(x) - left;
        let py = i64::from(y) - top;
        if px < 0 || py < 0 || px >= i64::from(rw) || py >= i64::from(rh) {
            *pixel = image::Rgb([bg.round() as u16; 3]);
            continue;
        }
        let sx =
            ((px as f64 + 0.5) * f64::from(sw) / f64::from(rw) - 0.5).clamp(0.0, f64::from(sw - 1));
        let sy =
            ((py as f64 + 0.5) * f64::from(sh) / f64::from(rh) - 0.5).clamp(0.0, f64::from(sh - 1));
        let x0 = sx.floor() as u32;
        let y0 = sy.floor() as u32;
        let x1 = (x0 + 1).min(sw - 1);
        let y1 = (y0 + 1).min(sh - 1);
        let fx = (sx - f64::from(x0)) as f32;
        let fy = (sy - f64::from(y0)) as f32;
        let weights = [
            (x0, y0, (1.0 - fx) * (1.0 - fy)),
            (x1, y0, fx * (1.0 - fy)),
            (x0, y1, (1.0 - fx) * fy),
            (x1, y1, fx * fy),
        ];
        let mut rgb = [0_f32; 3];
        let mut a = 0_f32;
        for (ix, iy, weight) in weights {
            let p = source.get_pixel(ix, iy).0;
            for c in 0..3 {
                rgb[c] += f32::from(p[c]) * weight;
            }
            a += f32::from(alpha.get_pixel(ix, iy).0[0]) * weight / 255.0;
        }
        *pixel = image::Rgb(std::array::from_fn(|c| {
            ((rgb[c] * gain * adjustment[c]).clamp(0.0, 65535.0) * a + bg * (1.0 - a))
                .round()
                .clamp(0.0, 65535.0) as u16
        }));
    }
    Ok(result)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FinishInput {
    original: String,
    cutout: String,
    settings: FinishSettings,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishPreview {
    original: String,
    composite: String,
}

#[tauri::command]
pub async fn portfolio_finish_preview(input: FinishInput) -> Result<FinishPreview, String> {
    tauri::async_runtime::spawn_blocking(move || render_finish_preview(input))
        .await
        .map_err(|_| "Could not render the preview".to_string())?
}

fn render_finish_preview(input: FinishInput) -> Result<FinishPreview, String> {
    let settings = input.settings;
    validate_finish_settings(&settings)?;
    fn decode(data: &str) -> Result<image::DynamicImage, String> {
        let bytes = crate::playground::portfolio_image(data)?;
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| "Invalid preview image")?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(768);
        limits.max_image_height = Some(768);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .map_err(|_| "Invalid or oversized preview image".into())
    }
    let original = decode(&input.original)?.into_rgb8();
    let cutout = decode(&input.cutout)?;
    if !cutout.color().has_alpha() || (cutout.width(), cutout.height()) != original.dimensions() {
        return Err("The cutout does not match the original preview".into());
    }
    let (width, height) = match settings.crop.as_str() {
        "original" => original.dimensions(),
        "portrait" => (512, 768),
        "square" => (768, 768),
        _ => return Err("Invalid crop".into()),
    };
    let (sw, sh) = original.dimensions();
    if sw == 0 || sh == 0 {
        return Err("The preview is empty".into());
    }
    let factor = (f64::from(width) / f64::from(sw)).max(f64::from(height) / f64::from(sh))
        * f64::from(settings.scale)
        / 100.0;
    let rw = (f64::from(sw) * factor).round().max(1.0) as u32;
    let rh = (f64::from(sh) * factor).round().max(1.0) as u32;
    if rw > 4096 || rh > 4096 {
        return Err("Preview crop is too large".into());
    }
    let resized = image::imageops::resize(&original, rw, rh, image::imageops::FilterType::Lanczos3);
    let cutout_rgba = cutout.to_rgba8();
    let alpha =
        image::imageops::resize(&cutout_rgba, rw, rh, image::imageops::FilterType::Lanczos3);
    let cutout_alpha = image::GrayImage::from_fn(sw, sh, |x, y| {
        image::Luma([cutout_rgba.get_pixel(x, y).0[3]])
    });
    let left = horizontal_offset(
        &cutout_alpha,
        width,
        rw,
        settings.auto_center,
        settings.horizontal,
    );
    let top = (i64::from(height) - i64::from(rh)) / 2
        + i64::from(height) * i64::from(settings.vertical) / 100;
    let gain = 2_f32.powf(f32::from(settings.exposure) / 100.0);
    let warm = f32::from(settings.warmth) / 100.0 * 0.15;
    let mut source_frame = image::RgbImage::new(width, height);
    let mut composite = image::RgbImage::new(width, height);
    for (x, y, pixel) in composite.enumerate_pixels_mut() {
        let t = x as f32 / width.saturating_sub(1).max(1) as f32;
        let bg = (f32::from(settings.background_left) * (1.0 - t)
            + f32::from(settings.background_right) * t)
            .round() as u8;
        let sx = i64::from(x) - left;
        let sy = i64::from(y) - top;
        if sx < 0 || sy < 0 || sx >= i64::from(rw) || sy >= i64::from(rh) {
            *pixel = image::Rgb([bg; 3]);
            source_frame.put_pixel(x, y, image::Rgb([bg; 3]));
            continue;
        }
        let src = resized.get_pixel(sx as u32, sy as u32).0;
        source_frame.put_pixel(x, y, image::Rgb(src));
        let a = f32::from(alpha.get_pixel(sx as u32, sy as u32).0[3]) / 255.0;
        let adjustment = [1.0 + warm, 1.0, 1.0 - warm];
        *pixel = image::Rgb(std::array::from_fn(|c| {
            let value = (f32::from(src[c]) * gain * adjustment[c]).clamp(0.0, 255.0);
            (value * a + f32::from(bg) * (1.0 - a)).round() as u8
        }));
    }
    fn data_uri(image: image::RgbImage) -> Result<String, String> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|_| "Could not encode preview")?;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
        ))
    }
    Ok(FinishPreview {
        original: data_uri(source_frame)?,
        composite: data_uri(composite)?,
    })
}

#[tauri::command]
pub async fn portfolio_develop_raw(
    app: tauri::AppHandle,
    raw: String,
) -> Result<Option<DevelopedRaw>, String> {
    #[cfg(windows)]
    use tauri::Manager;
    if raw.len() > MAX_RAW_BYTES.div_ceil(3) * 4 + 4 {
        return Err("RAW file exceeds the 150 MB limit".into());
    }
    if DEVELOPING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("A RAW image is already being developed".into());
    }
    struct BusyGuard;
    impl Drop for BusyGuard {
        fn drop(&mut self) {
            DEVELOPING.store(false, Ordering::Release);
        }
    }
    let guard = BusyGuard;
    #[cfg(windows)]
    let owner = app
        .get_webview_window("playground")
        .and_then(|window| window.hwnd().ok())
        .map(|hwnd| hwnd.0 as isize)
        .unwrap_or(0);
    #[cfg(not(windows))]
    let owner = {
        let _ = app;
        0_isize
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let Some(destination) = choose_developed_path(owner)? else {
            return Ok(None);
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&raw)
            .map_err(|_| "The RAW data is not valid")?;
        drop(raw);
        if bytes.is_empty() || bytes.len() > MAX_RAW_BYTES {
            return Err("RAW file is empty or exceeds the 150 MB limit".into());
        }
        let (width, height) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            develop_raw_to_path(bytes, &destination)
        }))
        .map_err(|_| "RAW development failed unexpectedly")??;
        Ok(Some(DevelopedRaw {
            path: destination.to_string_lossy().into_owned(),
            width,
            height,
        }))
    })
    .await
    .map_err(|_| "RAW development task failed".to_string())?
}

fn develop_raw_to_path(
    bytes: Vec<u8>,
    destination: &std::path::Path,
) -> Result<(u32, u32), String> {
    if !destination
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err("Choose a PNG filename".into());
    }
    let rgb = develop_raw(bytes)?;
    write_developed_png(&rgb, destination)?;
    Ok(rgb.dimensions())
}

fn develop_raw(bytes: Vec<u8>) -> Result<image::ImageBuffer<image::Rgb<u16>, Vec<u16>>, String> {
    let source = rawler::rawsource::RawSource::new_from_shared_vec(std::sync::Arc::new(bytes));
    let decoder =
        rawler::get_decoder(&source).map_err(|e| format!("RAW format is not supported: {e}"))?;
    let params = rawler::decoders::RawDecodeParams { image_index: 0 };
    let metadata = decoder
        .raw_metadata(&source, &params)
        .map_err(|e| format!("Could not read RAW metadata: {e}"))?;
    let raw = decoder
        .raw_image(&source, &params, false)
        .map_err(|e| format!("Could not decode RAW file: {e}"))?;
    let dimensions = raw.dim();
    if dimensions.w == 0
        || dimensions.h == 0
        || (dimensions.w as u64).saturating_mul(dimensions.h as u64) > MAX_RAW_PIXELS
    {
        return Err("RAW image exceeds the 80-megapixel development limit".into());
    }
    let mut developed = rawler::imgop::develop::RawDevelop::default()
        .develop_intermediate(&raw)
        .map_err(|e| format!("Could not develop RAW image: {e}"))?
        .to_dynamic_image()
        .ok_or("Could not construct developed image")?;
    drop(raw);
    if let Some(value) = metadata.exif.orientation {
        let orientation = u8::try_from(value)
            .ok()
            .and_then(image::metadata::Orientation::from_exif)
            .ok_or("RAW image has an unsupported orientation")?;
        developed.apply_orientation(orientation);
    }
    Ok(developed.into_rgb16())
}

fn write_developed_png(
    rgb: &image::ImageBuffer<image::Rgb<u16>, Vec<u16>>,
    destination: &std::path::Path,
) -> Result<(), String> {
    write_png_inner(rgb, destination, None)
}

fn write_png_inner(
    rgb: &image::ImageBuffer<image::Rgb<u16>, Vec<u16>>,
    destination: &std::path::Path,
    cancel: Option<&crate::exec::CancelToken>,
) -> Result<(), String> {
    use std::io::Write;
    let (width, height) = rgb.dimensions();
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or("Choose an output folder")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".rune-developed-")
        .tempfile_in(parent)
        .map_err(|e| format!("Could not create output: {e}"))?;
    {
        let writer = std::io::BufWriter::new(temporary.as_file_mut());
        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Sixteen);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut png = encoder
            .write_header()
            .map_err(|e| format!("Could not write PNG header: {e}"))?;
        let mut stream = png
            .stream_writer()
            .map_err(|e| format!("Could not begin PNG: {e}"))?;
        let mut row = Vec::with_capacity(width as usize * 6);
        for pixels in rgb.rows() {
            if cancel.is_some_and(crate::exec::CancelToken::is_cancelled) {
                return Err("Export cancelled".into());
            }
            row.clear();
            for pixel in pixels {
                for channel in pixel.0 {
                    row.extend_from_slice(&channel.to_be_bytes());
                }
            }
            stream
                .write_all(&row)
                .map_err(|e| format!("Could not write PNG: {e}"))?;
        }
        stream
            .finish()
            .map_err(|e| format!("Could not finish PNG: {e}"))?;
        png.finish()
            .map_err(|e| format!("Could not finish PNG: {e}"))?;
    }
    if cancel.is_some_and(crate::exec::CancelToken::is_cancelled) {
        return Err("Export cancelled".into());
    }
    temporary
        .persist_noclobber(destination)
        .map_err(|e| format!("Could not save PNG (choose a new filename): {}", e.error))?;
    Ok(())
}

#[cfg(not(windows))]
fn choose_developed_path(_: isize) -> Result<Option<std::path::PathBuf>, String> {
    Err("Saving developed RAW images currently requires Windows".into())
}

#[cfg(windows)]
fn choose_developed_path(owner: isize) -> Result<Option<std::path::PathBuf>, String> {
    std::thread::spawn(move || unsafe {
        use windows::Win32::{
            Foundation::HWND,
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree, CoUninitialize,
            },
            UI::Shell::{
                FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FileSaveDialog, IFileSaveDialog,
                SIGDN_FILESYSPATH,
            },
        };
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|e| format!("Could not open save dialog: {e}"))?;
        struct ComGuard;
        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe { CoUninitialize() }
            }
        }
        let _guard = ComGuard;
        let dialog: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| e.to_string())?;
        let options = dialog.GetOptions().map_err(|e| e.to_string())?;
        dialog
            .SetOptions((options | FOS_FORCEFILESYSTEM) & !FOS_OVERWRITEPROMPT)
            .map_err(|e| e.to_string())?;
        dialog
            .SetDefaultExtension(windows::core::w!("png"))
            .map_err(|e| e.to_string())?;
        dialog
            .SetFileName(windows::core::w!("portfolio.png"))
            .map_err(|e| e.to_string())?;
        let parent = if owner == 0 {
            None
        } else {
            Some(HWND(owner as *mut std::ffi::c_void))
        };
        if let Err(e) = dialog.Show(parent) {
            if e.code().0 == 0x800704c7_u32 as i32 {
                return Ok(None);
            }
            return Err(format!("Could not choose output file: {e}"));
        }
        let item = dialog.GetResult().map_err(|e| e.to_string())?;
        let name = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|e| e.to_string())?;
        let path = name
            .to_string()
            .map_err(|_| "Invalid output filename".to_string());
        CoTaskMemFree(Some(name.0.cast()));
        Ok(Some(std::path::PathBuf::from(path?)))
    })
    .join()
    .map_err(|_| "Save dialog failed".to_string())?
}

#[cfg(test)]
mod develop_tests {
    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW"]
    fn develops_real_review_preview() {
        use base64::Engine;
        let bytes = std::fs::read(std::env::var_os("PORTFOLIO_TEST_ARW").unwrap()).unwrap();
        let uri = super::developed_preview(bytes).unwrap();
        let data = base64::engine::general_purpose::STANDARD
            .decode(uri.split_once(',').unwrap().1)
            .unwrap();
        let image = image::load_from_memory(&data).unwrap();
        assert_eq!((image.width(), image.height()), (512, 768));
        assert_eq!(image.color(), image::ColorType::Rgb8);
    }
    #[test]
    #[ignore = "requires PORTFOLIO_MODEL_INPUT, PORTFOLIO_MODEL_MATTE and PORTFOLIO_LOOK_OUT"]
    fn suggests_real_shoot_look() {
        use base64::Engine;
        let source = image::open(std::env::var_os("PORTFOLIO_MODEL_INPUT").unwrap())
            .unwrap()
            .resize(768, 768, image::imageops::FilterType::Lanczos3)
            .into_rgb8();
        let alpha = image::open(std::env::var_os("PORTFOLIO_MODEL_MATTE").unwrap())
            .unwrap()
            .resize_exact(
                source.width(),
                source.height(),
                image::imageops::FilterType::Triangle,
            )
            .into_luma8();
        let rgba = image::RgbaImage::from_fn(source.width(), source.height(), |x, y| {
            let p = source.get_pixel(x, y).0;
            image::Rgba([p[0], p[1], p[2], alpha.get_pixel(x, y).0[0]])
        });
        let uri = |img: image::DynamicImage| {
            let mut out = std::io::Cursor::new(Vec::new());
            img.write_to(&mut out, image::ImageFormat::Png).unwrap();
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(out.into_inner())
            )
        };
        let original = uri(image::DynamicImage::ImageRgb8(source));
        let cutout = uri(image::DynamicImage::ImageRgba8(rgba));
        let look = super::suggest_look(&[super::LookSample {
            original: original.clone(),
            cutout: cutout.clone(),
        }])
        .unwrap();
        eprintln!(
            "suggested exposure {} warmth {}",
            look.exposure, look.warmth
        );
        let result = super::render_finish_preview(super::FinishInput {
            original,
            cutout,
            settings: super::FinishSettings {
                crop: "portrait".into(),
                auto_center: true,
                scale: 100,
                horizontal: 0,
                vertical: 0,
                exposure: look.exposure,
                warmth: look.warmth,
                background_left: 160,
                background_right: 135,
            },
        })
        .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.composite.split_once(',').unwrap().1)
            .unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::env::var_os("PORTFOLIO_LOOK_OUT").unwrap())
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
    #[test]
    fn automatic_look_uses_visible_pixels_and_limits_correction() {
        use base64::Engine;
        let uri = |image: image::DynamicImage| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
            )
        };
        let rgb = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            20,
            20,
            image::Rgb([150, 150, 135]),
        ));
        let opaque = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            20,
            20,
            image::Rgba([0, 0, 0, 255]),
        ));
        let sample = super::LookSample {
            original: uri(rgb),
            cutout: uri(opaque),
        };
        let look = super::suggest_look(&[sample]).unwrap();
        assert!(look.exposure > 0 && look.exposure <= 60);
        assert!(look.warmth <= 0 && look.warmth >= -20);
        let invisible = super::LookSample {
            original: uri(image::DynamicImage::ImageRgb8(image::RgbImage::new(20, 20))),
            cutout: uri(image::DynamicImage::ImageRgba8(image::RgbaImage::new(
                20, 20,
            ))),
        };
        assert!(super::suggest_look(&[invisible]).is_err());
        assert!(super::suggest_look(&[]).is_err());
    }
    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW, PORTFOLIO_FULL_MATTE and PORTFOLIO_COMPOSITE_OUT"]
    fn renders_real_full_finish() {
        let bytes = std::fs::read(std::env::var_os("PORTFOLIO_TEST_ARW").unwrap()).unwrap();
        let source = super::develop_raw(bytes).unwrap();
        let alpha = image::open(std::env::var_os("PORTFOLIO_FULL_MATTE").unwrap())
            .unwrap()
            .into_luma8();
        let settings = super::FinishSettings {
            crop: "portrait".into(),
            auto_center: true,
            scale: 100,
            horizontal: 0,
            vertical: 0,
            exposure: 35,
            warmth: -10,
            background_left: 160,
            background_right: 135,
        };
        let token = crate::exec::CancelToken::default();
        let composite = super::render_full_size(&source, &alpha, &settings, &token).unwrap();
        assert_eq!(composite.dimensions(), (4672, 7008));
        let path = std::path::PathBuf::from(std::env::var_os("PORTFOLIO_COMPOSITE_OUT").unwrap());
        super::write_png_inner(&composite, &path, Some(&token)).unwrap();
        let png = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()))
            .read_info()
            .unwrap();
        assert_eq!(png.info().bit_depth, png::BitDepth::Sixteen);
    }
    #[test]
    fn full_finish_centers_and_grades_the_subject_with_the_requested_crop() {
        let source = image::ImageBuffer::from_pixel(6, 6, image::Rgb([10000_u16, 15000, 20000]));
        let alpha =
            image::GrayImage::from_fn(6, 6, |x, _| image::Luma([if x < 2 { 255 } else { 0 }]));
        let mut settings = super::FinishSettings {
            crop: "portrait".into(),
            auto_center: true,
            scale: 100,
            horizontal: 0,
            vertical: 0,
            exposure: 100,
            warmth: 0,
            background_left: 100,
            background_right: 100,
        };
        let token = crate::exec::CancelToken::default();
        let result = super::render_full_size(&source, &alpha, &settings, &token).unwrap();
        assert_eq!(result.dimensions(), (4, 6));
        assert_eq!(result.get_pixel(0, 2).0, [25700; 3]);
        assert_eq!(result.get_pixel(1, 2).0, [20000, 30000, 40000]);
        assert_eq!(result.get_pixel(2, 2).0, [20000, 30000, 40000]);
        settings.auto_center = false;
        let uncentered = super::render_full_size(&source, &alpha, &settings, &token).unwrap();
        assert_eq!(uncentered.get_pixel(0, 2).0, [20000, 30000, 40000]);
        settings.crop = "square".into();
        assert_eq!(
            super::render_full_size(&source, &alpha, &settings, &token)
                .unwrap()
                .dimensions(),
            (6, 6)
        );
        settings.crop = "original".into();
        settings.exposure = 0;
        settings.warmth = 100;
        settings.horizontal = 50;
        let shifted = super::render_full_size(&source, &alpha, &settings, &token).unwrap();
        assert_eq!(shifted.get_pixel(0, 2).0, [25700; 3]);
        assert_eq!(shifted.get_pixel(3, 2).0, [11500, 15000, 17000]);
        settings.exposure = 101;
        assert!(super::render_full_size(&source, &alpha, &settings, &token).is_err());
    }
    #[tokio::test]
    #[ignore = "requires PORTFOLIO_TEST_ARW, PORTFOLIO_FULL_MATTE and PORTFOLIO_REFINED_OUT and local Infer"]
    async fn refines_real_matte_with_local_infer() {
        let bytes = std::fs::read(std::env::var_os("PORTFOLIO_TEST_ARW").unwrap()).unwrap();
        let source = super::develop_raw(bytes).unwrap();
        let alpha = image::open(std::env::var_os("PORTFOLIO_FULL_MATTE").unwrap())
            .unwrap()
            .into_luma8();
        let original = alpha.clone();
        let result = super::refine_matte(
            &reqwest::Client::new(),
            None,
            reqwest::Url::parse("http://127.0.0.1:8080/v1/images/portrait-mattings").unwrap(),
            "birefnet-matting",
            &source,
            alpha,
            &crate::exec::CancelToken::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.dimensions(), original.dimensions());
        assert!(result.pixels().zip(original.pixels()).any(|(a, b)| a != b));
        let path = std::path::PathBuf::from(std::env::var_os("PORTFOLIO_REFINED_OUT").unwrap());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        image::DynamicImage::ImageLuma8(result)
            .write_to(&mut std::io::BufWriter::new(file), image::ImageFormat::Png)
            .unwrap();
    }
    #[test]
    fn matte_refinement_limits_changes_and_feathers_internal_tile_edges() {
        assert_eq!(super::tile_starts(2000), vec![0]);
        assert_eq!(super::tile_starts(4672), vec![0, 1792]);
        let mut base = image::GrayImage::from_pixel(700, 5, image::Luma([0]));
        base.put_pixel(350, 2, image::Luma([128]));
        let band = super::edge_band(&base, 100, 0, 500, 5);
        let detail = image::GrayImage::from_pixel(500, 5, image::Luma([255]));
        let mut result = base.clone();
        super::blend_matte_tile(&mut result, &detail, &band, 100, 0).unwrap();
        assert_eq!(result.get_pixel(100, 2).0[0], 0);
        assert_eq!(result.get_pixel(599, 2).0[0], 0);
        assert_eq!(result.get_pixel(200, 2).0[0], 0);
        assert!(result.get_pixel(350, 2).0[0] > 128);
        assert!(result.get_pixel(300, 2).0[0] > 0);
        assert_eq!(result.get_pixel(0, 2).0[0], 0);
        assert!(super::blend_matte_tile(&mut result, &detail, &band[..2], 100, 0).is_err());
    }
    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW, PORTFOLIO_FULL_MATTE and PORTFOLIO_COMPOSITE_OUT"]
    fn exports_real_full_resolution_composite() {
        let bytes = std::fs::read(std::env::var_os("PORTFOLIO_TEST_ARW").unwrap()).unwrap();
        let source = super::develop_raw(bytes).unwrap();
        let alpha = image::open(std::env::var_os("PORTFOLIO_FULL_MATTE").unwrap())
            .unwrap()
            .into_luma8();
        let token = crate::exec::CancelToken::default();
        let composite = super::composite_full_size(source, &alpha, 160, 135, &token).unwrap();
        assert_eq!(composite.dimensions(), (4672, 7008));
        let path = std::path::PathBuf::from(std::env::var_os("PORTFOLIO_COMPOSITE_OUT").unwrap());
        super::write_png_inner(&composite, &path, Some(&token)).unwrap();
        let png = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()))
            .read_info()
            .unwrap();
        assert_eq!(png.info().bit_depth, png::BitDepth::Sixteen);
        assert_eq!(png.info().color_type, png::ColorType::Rgb);
    }
    #[test]
    fn full_composite_keeps_16_bit_subject_and_checks_matte_and_cancellation() {
        let source = image::ImageBuffer::from_pixel(3, 1, image::Rgb([12345_u16, 23456, 34567]));
        let alpha = image::GrayImage::from_raw(3, 1, vec![0, 128, 255]).unwrap();
        let cancel = crate::exec::CancelToken::default();
        let result = super::composite_full_size(source.clone(), &alpha, 100, 200, &cancel).unwrap();
        assert_eq!(result.get_pixel(0, 0).0, [25700; 3]);
        assert_eq!(result.get_pixel(2, 0).0, [12345, 23456, 34567]);
        assert!(
            super::composite_full_size(
                source.clone(),
                &image::GrayImage::new(1, 1),
                100,
                200,
                &cancel
            )
            .is_err()
        );
        assert!(
            super::composite_full_size(
                source.clone(),
                &image::GrayImage::from_pixel(3, 1, image::Luma([255])),
                100,
                200,
                &cancel
            )
            .is_err()
        );
        cancel.cancel();
        assert!(super::composite_full_size(source, &alpha, 100, 200, &cancel).is_err());
    }
    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW and PORTFOLIO_MODEL_INPUT_OUT"]
    fn prepares_real_developed_model_input() {
        let source = std::fs::read(std::env::var_os("PORTFOLIO_TEST_ARW").unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let developed = dir.path().join("developed.png");
        super::develop_raw_to_path(source, &developed).unwrap();
        let full = image::open(developed).unwrap();
        let preview = full
            .resize_exact(1360, 2048, image::imageops::FilterType::Lanczos3)
            .to_rgb8();
        let mut destination = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::env::var_os("PORTFOLIO_MODEL_INPUT_OUT").unwrap())
            .unwrap();
        image::DynamicImage::ImageRgb8(preview)
            .write_to(&mut destination, image::ImageFormat::Png)
            .unwrap();
    }
    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW and PORTFOLIO_MODEL_INPUT_OUT"]
    fn prepares_full_size_matte_test_input() {
        let source = std::fs::read(std::env::var_os("PORTFOLIO_TEST_ARW").unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let developed = dir.path().join("developed.png");
        super::develop_raw_to_path(source, &developed).unwrap();
        let original = image::open(developed).unwrap().to_rgb8();
        let destination = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::env::var_os("PORTFOLIO_MODEL_INPUT_OUT").unwrap())
            .unwrap();
        image::codecs::jpeg::JpegEncoder::new_with_quality(destination, 95)
            .encode_image(&image::DynamicImage::ImageRgb8(original))
            .unwrap();
    }
    #[test]
    #[ignore = "requires PORTFOLIO_PREVIEW_ORIGINAL, PORTFOLIO_PREVIEW_CUTOUT and PORTFOLIO_PREVIEW_OUT"]
    fn renders_real_finish_preview() {
        use base64::Engine;
        let uri = |name: &str| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD
                    .encode(std::fs::read(std::env::var_os(name).unwrap()).unwrap())
            )
        };
        let result = super::render_finish_preview(super::FinishInput {
            original: uri("PORTFOLIO_PREVIEW_ORIGINAL"),
            cutout: uri("PORTFOLIO_PREVIEW_CUTOUT"),
            settings: super::FinishSettings {
                crop: "portrait".into(),
                auto_center: false,
                scale: 100,
                horizontal: 0,
                vertical: 0,
                exposure: 20,
                warmth: -10,
                background_left: 180,
                background_right: 140,
            },
        })
        .unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.composite.split_once(',').unwrap().1)
            .unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::env::var_os("PORTFOLIO_PREVIEW_OUT").unwrap())
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
    #[test]
    fn finish_preview_preserves_source_and_applies_background_and_exposure() {
        use base64::Engine;
        let mut original = std::io::Cursor::new(Vec::new());
        let mut cutout = std::io::Cursor::new(Vec::new());
        let rgb = image::RgbImage::from_pixel(2, 1, image::Rgb([80, 100, 120]));
        let alpha = image::RgbaImage::from_fn(2, 1, |x, _| {
            image::Rgba([0, 0, 0, if x == 0 { 255 } else { 0 }])
        });
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(&mut original, image::ImageFormat::Png)
            .unwrap();
        image::DynamicImage::ImageRgba8(alpha)
            .write_to(&mut cutout, image::ImageFormat::Png)
            .unwrap();
        let uri = |bytes: Vec<u8>| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        };
        let original = uri(original.into_inner());
        let cutout = uri(cutout.into_inner());
        let settings = super::FinishSettings {
            crop: "original".into(),
            auto_center: false,
            scale: 100,
            horizontal: 0,
            vertical: 0,
            exposure: 0,
            warmth: 0,
            background_left: 150,
            background_right: 100,
        };
        let result = super::render_finish_preview(super::FinishInput {
            original: original.clone(),
            cutout: cutout.clone(),
            settings: settings.clone(),
        })
        .unwrap();
        let decode = |uri: &str| {
            image::load_from_memory(
                &base64::engine::general_purpose::STANDARD
                    .decode(uri.split_once(',').unwrap().1)
                    .unwrap(),
            )
            .unwrap()
            .to_rgb8()
        };
        let composite = decode(&result.composite);
        assert_eq!(composite.get_pixel(0, 0).0, [80, 100, 120]);
        assert_eq!(composite.get_pixel(1, 0).0, [100, 100, 100]);
        assert_eq!(decode(&result.original).get_pixel(1, 0).0, [80, 100, 120]);
        let mut adjusted = settings.clone();
        adjusted.exposure = 100;
        let brighter = super::render_finish_preview(super::FinishInput {
            original: original.clone(),
            cutout: cutout.clone(),
            settings: adjusted.clone(),
        })
        .unwrap();
        assert_eq!(
            decode(&brighter.composite).get_pixel(0, 0).0,
            [160, 200, 240]
        );
        adjusted.crop = "square".into();
        let output = super::render_finish_preview(super::FinishInput {
            original,
            cutout,
            settings: adjusted,
        })
        .unwrap();
        assert_eq!(decode(&output.original).dimensions(), (768, 768));
        assert_eq!(decode(&output.composite).dimensions(), (768, 768));
    }

    #[test]
    fn writes_srgb_png_without_losing_precision_or_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("developed.png");
        let pixels = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_fn(2, 1, |x, _| {
            if x == 0 {
                image::Rgb([1, 257, 65535])
            } else {
                image::Rgb([1234, 2345, 3456])
            }
        });
        super::write_developed_png(&pixels, &path).unwrap();
        let reader =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()))
                .read_info()
                .unwrap();
        assert_eq!(reader.info().bit_depth, png::BitDepth::Sixteen);
        assert!(reader.info().srgb.is_some());
        assert_eq!(image::open(&path).unwrap().to_rgb16(), pixels);
        let original = std::fs::read(&path).unwrap();
        assert!(super::write_developed_png(&pixels, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW to point to a local camera RAW file"]
    fn develops_a_real_raw_at_full_resolution() {
        let path = std::env::var_os("PORTFOLIO_TEST_ARW").expect("set PORTFOLIO_TEST_ARW");
        let bytes = std::fs::read(path).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("developed.png");
        assert_eq!(
            super::develop_raw_to_path(bytes, &output).unwrap(),
            (4672, 7008)
        );
        let reader = png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(&output).unwrap(),
        ))
        .read_info()
        .unwrap();
        assert_eq!(reader.info().bit_depth, png::BitDepth::Sixteen);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);
        assert!(reader.info().srgb.is_some());
        let decoded = image::open(output).unwrap().to_rgb16();
        assert!(
            decoded
                .pixels()
                .take(100_000)
                .flat_map(|pixel| pixel.0)
                .any(|value| value % 257 != 0)
        );
    }
}

#[tauri::command]
pub async fn portfolio_raw_preview(raw: String) -> Result<String, String> {
    if raw.len() > MAX_RAW_BYTES.div_ceil(3) * 4 + 4 {
        return Err("RAW file exceeds the 150 MB limit".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|_| "Could not read the RAW file")?;
        if bytes.is_empty() || bytes.len() > MAX_RAW_BYTES {
            return Err("RAW file is empty or exceeds the 150 MB limit".into());
        }
        decode_preview(&bytes)
    })
    .await
    .map_err(|_| "RAW preview task failed".to_string())?
}

#[cfg(not(windows))]
fn decode_preview(_: &[u8]) -> Result<String, String> {
    Err("RAW previews currently require Windows and its RAW image codec".into())
}

#[cfg(windows)]
fn decode_preview(bytes: &[u8]) -> Result<String, String> {
    use std::io::Cursor;
    use windows::Win32::{
        Graphics::Imaging::{
            CLSID_WICImagingFactory, GUID_WICPixelFormat24bppRGB, IWICImagingFactory,
            WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
            WICDecodeMetadataCacheOnLoad,
        },
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
            StructuredStorage::{PROPVARIANT, PropVariantClear, PropVariantToUInt16},
        },
    };
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| format!("Could not initialize RAW decoder: {e}"))?;
        struct ComGuard;
        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe { CoUninitialize() }
            }
        }
        let _guard = ComGuard;
        let process = || -> windows::core::Result<Vec<u8>> {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = factory.CreateStream()?;
            stream.InitializeFromMemory(bytes)?;
            let decoder = factory.CreateDecoderFromStream(
                &stream,
                std::ptr::null(),
                WICDecodeMetadataCacheOnLoad,
            )?;
            let frame = decoder.GetFrame(0)?;
            let orientation = frame.GetMetadataQueryReader().ok().and_then(|reader| {
                for query in [
                    windows::core::w!("/ifd/{ushort=274}"),
                    windows::core::w!("/app1/ifd/{ushort=274}"),
                ] {
                    let mut value = PROPVARIANT::default();
                    let result = reader
                        .GetMetadataByName(query, &mut value)
                        .and_then(|_| PropVariantToUInt16(&value));
                    let _ = PropVariantClear(&mut value);
                    if let Ok(value) = result {
                        return u8::try_from(value)
                            .ok()
                            .and_then(image::metadata::Orientation::from_exif);
                    }
                }
                None
            });
            let mut width = 0;
            let mut height = 0;
            frame.GetSize(&mut width, &mut height)?;
            if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 200_000_000 {
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80070057_u32 as i32,
                )));
            }
            let scale = (1280.0 / width.max(height) as f64).min(1.0);
            let w = (width as f64 * scale).round().max(1.0) as u32;
            let h = (height as f64 * scale).round().max(1.0) as u32;
            let scaler = factory.CreateBitmapScaler()?;
            scaler.Initialize(&frame, w, h, WICBitmapInterpolationModeFant)?;
            let converter = factory.CreateFormatConverter()?;
            converter.Initialize(
                &scaler,
                &GUID_WICPixelFormat24bppRGB,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )?;
            let mut pixels = vec![0; (w * h * 3) as usize];
            converter.CopyPixels(std::ptr::null(), w * 3, &mut pixels)?;
            let rgb = image::RgbImage::from_raw(w, h, pixels).ok_or_else(|| {
                windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as i32))
            })?;
            let mut image = image::DynamicImage::ImageRgb8(rgb);
            if let Some(orientation) = orientation {
                image.apply_orientation(orientation);
            }
            let rgb = image.into_rgb8();
            let mut output = Cursor::new(Vec::new());
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 85)
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|_| {
                    windows::core::Error::from_hresult(windows::core::HRESULT(
                        0x80004005_u32 as i32,
                    ))
                })?;
            Ok(output.into_inner())
        };
        let jpeg = process().map_err(|e| {
            format!("Could not decode RAW file; verify the Windows RAW codec is installed: {e}")
        })?;
        Ok(format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(jpeg)
        ))
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn rejects_invalid_raw_data() {
        assert!(super::decode_preview(b"not a raw image").is_err());
    }

    #[test]
    #[ignore = "requires PORTFOLIO_TEST_ARW to point to a local camera RAW file"]
    fn previews_a_real_raw_file() {
        let path = std::env::var_os("PORTFOLIO_TEST_ARW").expect("set PORTFOLIO_TEST_ARW");
        let bytes = std::fs::read(path).expect("read ARW");
        let preview = super::decode_preview(&bytes).expect("decode preview");
        use base64::Engine;
        let image = base64::engine::general_purpose::STANDARD
            .decode(preview.strip_prefix("data:image/jpeg;base64,").unwrap())
            .unwrap();
        let decoded = image::load_from_memory(&image).unwrap();
        assert_eq!(decoded.width().max(decoded.height()), 1280);
        assert!(
            decoded.height() > decoded.width(),
            "sample portrait must be oriented upright"
        );
    }

    #[tokio::test]
    #[ignore = "requires a local ARW and a running local Infer with qwen3.5-4b-q4"]
    async fn classifies_a_real_raw_preview() {
        let path = std::env::var_os("PORTFOLIO_TEST_ARW").expect("set PORTFOLIO_TEST_ARW");
        let preview = super::decode_preview(&std::fs::read(path).unwrap()).unwrap();
        let bytes = crate::playground::portfolio_image(&preview)
            .expect("portfolio accepts its own preview");
        let frame = crate::observe::screenshot::insight_image(&bytes, 1280, 1280).unwrap();
        let preview = frame.data_uri();
        let http = reqwest::Client::new();
        let profile = crate::model::profile::resolve_profile("qwen3.5-4b-q4", None, None);
        let client = crate::model::client::ChatClient {
            app: None,
            output_limit: None,
            http: &http,
            base_url: "http://127.0.0.1:8080",
            api_key: None,
            model: "qwen3.5-4b-q4",
            profile: Some(&profile),
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let result = crate::analysis::run::analyze_with_client(&client,
            "Classify the photographed person's body view as exactly one of: front, left three-quarter, right three-quarter, back, waist-detail, uncertain. Use uncertain when the view is ambiguous. Start the answer with the label.",
            &preview, &crate::exec::CancelToken::default(), std::time::Duration::from_secs(120)).await.unwrap();
        assert!(!result.parsed.answer.is_empty());
        println!("{}", result.parsed.answer);
    }

    #[tokio::test]
    #[ignore = "requires the supplied shoot and a running local Infer"]
    async fn classifies_eligible_shoot_files() {
        let dir = std::env::var_os("PORTFOLIO_TEST_DIR").expect("set PORTFOLIO_TEST_DIR");
        let http = reqwest::Client::new();
        let profile = crate::model::profile::resolve_profile("qwen3.5-4b-q4", None, None);
        let client = crate::model::client::ChatClient {
            app: None,
            output_limit: None,
            http: &http,
            base_url: "http://127.0.0.1:8080",
            api_key: None,
            model: "qwen3.5-4b-q4",
            profile: Some(&profile),
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("arw"))
                || path.with_extension("xmp").exists()
            {
                continue;
            }
            let preview = super::decode_preview(&std::fs::read(&path).unwrap()).unwrap();
            let result = crate::analysis::run::analyze_with_client(&client,
                "Classify the photographed person's body view as exactly one of: front, left three-quarter, right three-quarter, back, waist-detail, uncertain. Use uncertain when the view is ambiguous. Start the answer with the label.",
                &preview, &crate::exec::CancelToken::default(), std::time::Duration::from_secs(120)).await.unwrap();
            println!(
                "{}: {}",
                path.file_name().unwrap().to_string_lossy(),
                result.parsed.answer.lines().next().unwrap_or("")
            );
            assert!(!result.parsed.answer.is_empty());
            count += 1;
        }
        assert_eq!(count, 11);
    }

    #[test]
    #[ignore = "requires PORTFOLIO_TEST_DIR to point to the supplied shoot"]
    fn previews_eligible_shoot_files() {
        let dir = std::env::var_os("PORTFOLIO_TEST_DIR").expect("set PORTFOLIO_TEST_DIR");
        let mut decoded = 0;
        let mut blocked = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("arw"))
            {
                continue;
            }
            if path.with_extension("xmp").exists() {
                blocked += 1;
                continue;
            }
            let preview = super::decode_preview(&std::fs::read(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(preview.starts_with("data:image/jpeg;base64,"));
            decoded += 1;
        }
        assert_eq!((decoded, blocked), (11, 5));
    }
}
