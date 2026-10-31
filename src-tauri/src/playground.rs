use crate::{
    events::AppState,
    model::capabilities::ModelCapabilities,
    observe::screenshot::{self, CaptureTarget},
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
const DEPTH_TIMEOUT: Duration = Duration::from_secs(120);
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

pub fn depth_models(models: &[ModelCapabilities]) -> Vec<String> {
    models
        .iter()
        .filter(|m| m.tasks.iter().any(|task| task == DEPTH_TASK))
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

#[tauri::command]
pub async fn playground_depth(app: AppHandle, input: DepthInput) -> Result<DepthResult, String> {
    if input.target.is_some() == input.image.is_some() {
        return Err("Choose either a screen or window, or an image".into());
    }
    let state = app.state::<AppState>();
    let (settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
    let status = crate::backend::check(&state.http, &settings, key.as_deref()).await;
    if !status.reachable || !status.auth_ok {
        return Err(status
            .error
            .unwrap_or("Infer is unavailable or authentication failed".into()));
    }
    let model = input.model.trim().to_string();
    if !depth_models(&status.models).contains(&model) {
        return Err("Choose a depth model that Infer lists with image-to-depth support".into());
    }
    let max_edge = settings.model_image_resolution.max_edge();
    let capture_edge = settings.recording_resolution.max_edge();
    let (target, image) = (input.target, input.image);
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
    let original = frame.data_uri();
    let started = Instant::now();
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1/inference",
        settings.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "The Infer address is not valid")?;
    url.query_pairs_mut().append_pair("engine", &model);
    let mut request = state
        .http
        .post(url)
        .timeout(DEPTH_TIMEOUT)
        .json(&serde_json::json!({
            "type": "image_depth",
            "image_bytes": frame.jpeg,
            "preview": true,
        }));
    if let Some(key) = key.as_deref() {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("Depth request failed: {e}"))?;
    let code = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Depth response failed: {e}"))?;
    if !code.is_success() {
        let detail: String = body.chars().take(300).collect();
        return Err(format!(
            "Infer refused the depth request ({code}): {detail}"
        ));
    }
    depth_result(&body, original, model, started.elapsed().as_millis() as u64)
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
        assert_eq!(depth_models(&[chat, depth, none]), vec!["depth-small"]);
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
    fn accepts_only_png_data_for_depth() {
        assert!(depth_image("data:image/jpeg;base64,AA==").is_err());
        assert_eq!(depth_image("data:image/png;base64,QUE=").unwrap(), b"AA");
    }
}
