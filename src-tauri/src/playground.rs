use crate::{
    events::AppState,
    model::capabilities::ModelCapabilities,
    observe::screenshot::{self, CaptureTarget},
    settings::Settings,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "playground";

pub fn open(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(
        app,
        WINDOW,
        WebviewUrl::App("index.html?view=playground".into()),
    )
    .title("Rune Playground")
    .inner_size(crate::window::PANEL_WIDTH, crate::window::PANEL_HEIGHT)
    .min_inner_size(960.0, 600.0)
    .resizable(true)
    .decorations(false)
    .transparent(true)
    .center()
    .build()?;
    crate::window::protect(&window)?;
    window.set_focus()?;
    Ok(())
}

#[tauri::command]
pub async fn open_playground(app: AppHandle) -> Result<(), String> {
    open(&app).map_err(|err| format!("Could not open Playground: {err}"))
}

const DEPTH_TASK: &str = "image-to-depth";
const UPSCALE_TASK: &str = "image-upscale";
const CREATE_TASK: &str = "text-to-image";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_IMAGE_CHARS: usize = 40_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DepthInput {
    #[serde(default)]
    target: Option<CaptureTarget>,
    #[serde(default)]
    image: Option<String>,
    model: String,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DepthResult {
    original: String,
    preview: Option<String>,
    width: u32,
    height: u32,
    min: f32,
    max: f32,
    model: String,
    elapsed_ms: u64,
}

#[derive(Deserialize)]
struct DepthEnvelope {
    response: DepthBody,
}

#[derive(Deserialize)]
struct DepthBody {
    width: u32,
    height: u32,
    values: Vec<f32>,
    #[serde(default)]
    preview_png_base64: Option<String>,
}

pub fn task_models(models: &[ModelCapabilities], task: &str) -> Vec<String> {
    models
        .iter()
        .filter(|m| m.tasks.iter().any(|t| t == task))
        .map(|m| m.id.clone())
        .collect()
}

fn depth_result(
    body: &str,
    original: String,
    model: String,
    elapsed_ms: u64,
) -> Result<DepthResult, String> {
    let envelope: DepthEnvelope =
        serde_json::from_str(body).map_err(|_| "Infer returned an unreadable depth map")?;
    let depth = envelope.response;
    if depth.values.len() as u64 != u64::from(depth.width) * u64::from(depth.height) {
        return Err("Infer returned a depth map of the wrong size".into());
    }
    let (min, max) = depth
        .values
        .iter()
        .filter(|v| v.is_finite())
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    if min > max {
        return Err("Infer returned an empty depth map".into());
    }
    Ok(DepthResult {
        original,
        preview: depth
            .preview_png_base64
            .map(|png| format!("data:image/png;base64,{png}")),
        width: depth.width,
        height: depth.height,
        min,
        max,
        model,
        elapsed_ms,
    })
}

fn depth_image(data: &str) -> Result<Vec<u8>, String> {
    let body = data
        .strip_prefix("data:image/png;base64,")
        .ok_or("Choose a PNG image")?;
    if body.len() > MAX_IMAGE_CHARS {
        return Err("The image is too large".into());
    }
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|_| "The image data is not valid".into())
}

struct Prepared {
    settings: Settings,
    key: Option<String>,
    model: String,
    frame: screenshot::Frame,
}

async fn session(
    app: &AppHandle,
    model: &str,
    task: &str,
) -> Result<(Settings, Option<String>, String), String> {
    let state = app.state::<AppState>();
    let (settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
    let status = crate::backend::check(&state.http, &settings, key.as_deref()).await;
    if !status.reachable || !status.auth_ok {
        return Err(status
            .error
            .unwrap_or("Infer is unavailable or authentication failed".into()));
    }
    let model = model.trim().to_string();
    if !task_models(&status.models, task).contains(&model) {
        return Err(format!(
            "Choose a model that Infer lists with {task} support"
        ));
    }
    Ok((settings, key, model))
}

async fn prepare(
    app: &AppHandle,
    target: Option<CaptureTarget>,
    image: Option<String>,
    model: &str,
    task: &str,
) -> Result<Prepared, String> {
    if target.is_some() == image.is_some() {
        return Err("Choose either a screen or window, or an image".into());
    }
    let (settings, key, model) = session(app, model, task).await?;
    let max_edge = settings.model_image_resolution.max_edge();
    let capture_edge = settings.recording_resolution.max_edge();
    let frame = tokio::time::timeout(
        Duration::from_secs(10),
        tauri::async_runtime::spawn_blocking(move || match (target, image) {
            (Some(target), _) => screenshot::insight_capture(target, max_edge, capture_edge)
                .map_err(|e| format!("Capture permission denied or target unavailable: {e}")),
            (None, Some(data)) => depth_image(&data).and_then(|bytes| {
                screenshot::insight_image(&bytes, max_edge, capture_edge).map_err(|e| e.to_string())
            }),
            (None, None) => Err("Choose a screen, window or image".into()),
        }),
    )
    .await
    .map_err(|_| "Reading the image timed out")?
    .map_err(|e| e.to_string())??;
    Ok(Prepared {
        settings,
        key,
        model,
        frame,
    })
}

async fn post(
    app: &AppHandle,
    key: Option<&str>,
    url: reqwest::Url,
    body: serde_json::Value,
    what: &str,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let mut request = state.http.post(url).timeout(REQUEST_TIMEOUT).json(&body);
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("{what} request failed: {e}"))?;
    let code = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("{what} response failed: {e}"))?;
    if !code.is_success() {
        let detail: String = text.chars().take(300).collect();
        return Err(format!(
            "Infer refused the {what} request ({code}): {detail}"
        ));
    }
    Ok(text)
}

fn infer_url(settings: &Settings, path: &str) -> Result<reqwest::Url, String> {
    reqwest::Url::parse(&format!(
        "{}{path}",
        settings.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "The Infer address is not valid".into())
}

#[tauri::command]
pub async fn playground_depth(app: AppHandle, input: DepthInput) -> Result<DepthResult, String> {
    let prepared = prepare(&app, input.target, input.image, &input.model, DEPTH_TASK).await?;
    let original = prepared.frame.data_uri();
    let started = Instant::now();
    let mut url = infer_url(&prepared.settings, "/v1/inference")?;
    url.query_pairs_mut().append_pair("engine", &prepared.model);
    let body = serde_json::json!({
        "type": "image_depth",
        "image_bytes": prepared.frame.jpeg,
        "preview": true,
    });
    let text = post(&app, prepared.key.as_deref(), url, body, "depth").await?;
    depth_result(
        &text,
        original,
        prepared.model.clone(),
        started.elapsed().as_millis() as u64,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpscaleInput {
    #[serde(default)]
    target: Option<CaptureTarget>,
    #[serde(default)]
    image: Option<String>,
    model: String,
    scale: u32,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleResult {
    original: String,
    output: String,
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
    scale: u32,
    model: String,
    elapsed_ms: u64,
}

#[derive(Deserialize)]
struct UpscaleEnvelope {
    data: Vec<UpscaleImage>,
    usage: UpscaleUsage,
}

#[derive(Deserialize)]
struct UpscaleImage {
    #[serde(default)]
    b64_json: Option<String>,
}

#[derive(Deserialize)]
struct UpscaleUsage {
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
    scale: u32,
}

fn upscale_result(
    body: &str,
    original: String,
    model: String,
    elapsed_ms: u64,
) -> Result<UpscaleResult, String> {
    let envelope: UpscaleEnvelope =
        serde_json::from_str(body).map_err(|_| "Infer returned an unreadable upscaled image")?;
    let encoded = envelope
        .data
        .into_iter()
        .find_map(|image| image.b64_json)
        .ok_or("Infer returned no upscaled image")?;
    let mime = if encoded.starts_with("/9j/") {
        "image/jpeg"
    } else {
        "image/png"
    };
    let usage = envelope.usage;
    Ok(UpscaleResult {
        original,
        output: format!("data:{mime};base64,{encoded}"),
        input_width: usage.input_width,
        input_height: usage.input_height,
        output_width: usage.output_width,
        output_height: usage.output_height,
        scale: usage.scale,
        model,
        elapsed_ms,
    })
}

#[tauri::command]
pub async fn playground_upscale(
    app: AppHandle,
    input: UpscaleInput,
) -> Result<UpscaleResult, String> {
    if !matches!(input.scale, 2 | 4) {
        return Err("Choose a 2x or 4x scale".into());
    }
    let prepared = prepare(&app, input.target, input.image, &input.model, UPSCALE_TASK).await?;
    let original = prepared.frame.data_uri();
    let started = Instant::now();
    let url = infer_url(&prepared.settings, "/v1/images/upscales")?;
    let body = serde_json::json!({
        "model": prepared.model,
        "image": original,
        "scale": input.scale,
        "response_format": "b64_json",
    });
    let text = post(&app, prepared.key.as_deref(), url, body, "upscale").await?;
    upscale_result(
        &text,
        original,
        prepared.model.clone(),
        started.elapsed().as_millis() as u64,
    )
}

const CREATE_SIZES: [&str; 4] = ["512x512", "768x768", "1024x1024", "1024x768"];
const MAX_PROMPT_CHARS: usize = 1000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateInput {
    prompt: String,
    model: String,
    size: String,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateResult {
    image: String,
    prompt: String,
    size: String,
    model: String,
    elapsed_ms: u64,
}

#[derive(Deserialize)]
struct CreateEnvelope {
    data: Vec<UpscaleImage>,
}

fn image_uri(encoded: &str) -> String {
    let mime = if encoded.starts_with("/9j/") {
        "image/jpeg"
    } else {
        "image/png"
    };
    format!("data:{mime};base64,{encoded}")
}

fn create_result(
    body: &str,
    prompt: String,
    size: String,
    model: String,
    elapsed_ms: u64,
) -> Result<CreateResult, String> {
    let envelope: CreateEnvelope =
        serde_json::from_str(body).map_err(|_| "Infer returned an unreadable image")?;
    let encoded = envelope
        .data
        .into_iter()
        .find_map(|image| image.b64_json)
        .ok_or("Infer returned no image")?;
    Ok(CreateResult {
        image: image_uri(&encoded),
        prompt,
        size,
        model,
        elapsed_ms,
    })
}

#[tauri::command]
pub async fn playground_create(app: AppHandle, input: CreateInput) -> Result<CreateResult, String> {
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Describe the image to create".into());
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!("Prompt exceeds {MAX_PROMPT_CHARS} characters"));
    }
    if !CREATE_SIZES.contains(&input.size.as_str()) {
        return Err("Choose one of the listed image sizes".into());
    }
    let (settings, key, model) = session(&app, &input.model, CREATE_TASK).await?;
    let started = Instant::now();
    let url = infer_url(&settings, "/v1/images/generations")?;
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "n": 1,
        "size": input.size,
        "response_format": "b64_json",
    });
    let text = post(&app, key.as_deref(), url, body, "image").await?;
    create_result(
        &text,
        prompt,
        input.size,
        model,
        started.elapsed().as_millis() as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_models_with_the_depth_task() {
        let mut depth = ModelCapabilities::unknown("depth-small");
        depth.tasks = vec!["image-to-depth".into()];
        let mut chat = ModelCapabilities::unknown("chat");
        chat.tasks = vec!["chat".into()];
        let none = ModelCapabilities::unknown("legacy");
        assert_eq!(
            task_models(&[chat, depth, none], DEPTH_TASK),
            vec!["depth-small"]
        );
    }

    #[test]
    fn reads_range_and_preview_from_the_response() {
        let body = r#"{"response":{"width":2,"height":2,"units":"relative","values":[0.5,2.0,1.0,-1.0],"preview_png_base64":"QUE="}}"#;
        let result = depth_result(body, "orig".into(), "m".into(), 7).unwrap();
        assert_eq!((result.min, result.max), (-1.0, 2.0));
        assert_eq!(
            result.preview.as_deref(),
            Some("data:image/png;base64,QUE=")
        );
        assert_eq!((result.width, result.height, result.elapsed_ms), (2, 2, 7));
    }

    #[test]
    fn rejects_bad_depth_responses() {
        let short = r#"{"response":{"width":2,"height":2,"values":[1.0]}}"#;
        assert!(depth_result(short, String::new(), String::new(), 0).is_err());
        assert!(depth_result("nope", String::new(), String::new(), 0).is_err());
    }

    #[test]
    fn reads_the_upscaled_image_and_sizes() {
        let body = r#"{"created":1,"data":[{"b64_json":"/9j/AA=="}],"usage":{"input_width":2,"input_height":1,"output_width":8,"output_height":4,"scale":4}}"#;
        let result = upscale_result(body, "orig".into(), "m".into(), 5).unwrap();
        assert_eq!(result.output, "data:image/jpeg;base64,/9j/AA==");
        assert_eq!(
            (result.output_width, result.output_height, result.scale),
            (8, 4, 4)
        );
        let png = body.replace("/9j/AA==", "iVBORw==");
        assert!(
            upscale_result(&png, String::new(), String::new(), 0)
                .unwrap()
                .output
                .starts_with("data:image/png;")
        );
        assert!(upscale_result(r#"{"data":[{}],"usage":{"input_width":1,"input_height":1,"output_width":1,"output_height":1,"scale":2}}"#, String::new(), String::new(), 0).is_err());
    }

    #[test]
    fn reads_the_created_image() {
        let body = r#"{"created":1,"data":[{"b64_json":"iVBORw=="}]}"#;
        let result = create_result(body, "cat".into(), "512x512".into(), "m".into(), 3).unwrap();
        assert_eq!(result.image, "data:image/png;base64,iVBORw==");
        assert!(
            create_result(
                r#"{"data":[{}]}"#,
                String::new(),
                String::new(),
                String::new(),
                0
            )
            .is_err()
        );
        assert!(create_result("nope", String::new(), String::new(), String::new(), 0).is_err());
    }

    #[test]
    fn accepts_only_png_data_for_depth() {
        assert!(depth_image("data:image/jpeg;base64,AA==").is_err());
        assert_eq!(depth_image("data:image/png;base64,QUE=").unwrap(), b"AA");
    }
}
