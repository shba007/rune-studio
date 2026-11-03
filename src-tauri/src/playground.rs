use crate::{
    events::AppState,
    model::capabilities::{ModelCapabilities, OperationStatus, OperationSupport},
    observe::screenshot::{self, CaptureTarget},
    settings::Settings,
};
use crate::{exec::CancelToken, sync::LockExt};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "playground";

#[derive(Default)]
pub struct PlaygroundRuns {
    next: AtomicU64,
    runs: Mutex<HashMap<u64, CancelToken>>,
}

impl PlaygroundRuns {
    fn start(&self) -> u64 {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        self.runs.locked().insert(id, CancelToken::default());
        id
    }
    fn cancel(&self, id: u64) {
        if let Some(token) = self.runs.locked().remove(&id) {
            token.cancel();
        }
    }
    async fn run<T>(
        &self,
        id: u64,
        work: impl Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        let token = self
            .runs
            .locked()
            .get(&id)
            .cloned()
            .ok_or("Playground run was cancelled")?;
        let result = tokio::select! {
            biased;
            () = async { while !token.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await; } } => Err("Playground run was cancelled".into()),
            result = work => if token.is_cancelled() { Err("Playground run was cancelled".into()) } else { result },
        };
        self.runs.locked().remove(&id);
        result
    }
}

#[tauri::command]
pub fn playground_start_run(app: AppHandle) -> u64 {
    app.state::<PlaygroundRuns>().start()
}

#[tauri::command]
pub fn playground_cancel_run(app: AppHandle, run_id: u64) {
    app.state::<PlaygroundRuns>().cancel(run_id);
}

macro_rules! cancellable_command {
    ($name:ident, $inner:ident, $input:ty, $output:ty) => {
        #[tauri::command]
        pub async fn $name(app: AppHandle, run_id: u64, input: $input) -> Result<$output, String> {
            app.state::<PlaygroundRuns>()
                .run(run_id, $inner(app.clone(), input))
                .await
        }
    };
}

cancellable_command!(
    playground_transcribe,
    transcribe,
    TranscribeInput,
    TranscribeResult
);
cancellable_command!(
    playground_translate_speech,
    translate_speech,
    TranscribeInput,
    TranscribeResult
);
cancellable_command!(playground_speech, speech, SpeechInput, SpeechResult);
cancellable_command!(playground_depth, depth, DepthInput, DepthResult);
cancellable_command!(playground_caption, caption, CaptionInput, CaptionResult);
cancellable_command!(playground_upscale, upscale, UpscaleInput, UpscaleResult);
cancellable_command!(playground_create, create, CreateInput, CreateResult);
cancellable_command!(playground_edit, edit, EditInput, EditResult);
cancellable_command!(playground_variation, variation, VariationInput, EditResult);
cancellable_command!(playground_reference, reference, ReferenceInput, EditResult);

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VisualPlanInput {
    target: Option<CaptureTarget>,
    image: Option<String>,
    model: String,
    question: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualPlanResult {
    image: String,
    questions: crate::analysis::run::VisualQuestions,
    model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VisualBranchInput {
    pub(crate) target: Option<CaptureTarget>,
    pub(crate) image: Option<String>,
    pub(crate) model: String,
    pub(crate) role: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualBranchResult {
    pub(crate) image: String,
    pub(crate) result: serde_json::Value,
}

fn visual_branch_contract(
    role: &str,
) -> Result<(&'static str, &'static str, &'static [&'static str]), String> {
    match role {
        "entities" => Ok(("image-to-detection", "/v1/images/detections", &["boxes"])),
        "structure" => Ok((
            "image-to-detection",
            "/v1/images/detections",
            &["boxes", "keypoints"],
        )),
        "instances" => Ok((
            "image-to-detection",
            "/v1/images/detections",
            &["boxes", "masks"],
        )),
        "oriented" => Ok((
            "image-to-detection",
            "/v1/images/detections",
            &["boxes", "oriented_boxes"],
        )),
        "regions" => Ok(("image-to-segmentation", "/v1/images/segmentations", &[])),
        _ => Err("Unknown visual analysis branch".into()),
    }
}

fn validate_visual_capability(role: &str, capabilities: &[String]) -> Result<(), String> {
    let has = |name: &str| {
        capabilities
            .iter()
            .any(|capability| capability.eq_ignore_ascii_case(name))
    };
    match role {
        "structure" if !has("Pose Estimation") => Err("Choose a model that advertises Pose Estimation".into()),
        "instances" if !has("Instance Segmentation") => Err("Choose a model that advertises Instance Segmentation".into()),
        "oriented" if !has("Oriented Bounding Boxes") => Err("Choose a model that advertises Oriented Bounding Boxes".into()),
        "entities" if ["Pose Estimation", "Instance Segmentation", "Oriented Bounding Boxes"].iter().any(|name| has(name)) => Err("Choose a standard object detection model, not a specialized pose, instance segmentation or oriented-box model".into()),
        _ => Ok(()),
    }
}

async fn visual_settings(app: &AppHandle, model: &str) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let (mut settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
    settings.models.analysis = model.trim().to_string();
    let status = crate::backend::check(&state.http, &settings, key.as_deref()).await;
    if !status.reachable || !status.auth_ok {
        return Err(status
            .error
            .unwrap_or("Infer is unavailable or authentication failed".into()));
    }
    crate::workflow::vision_model(&settings, &status.models)?;
    Ok(settings)
}

fn visual_guard(app: &AppHandle) -> Result<(), String> {
    if crate::recording::busy() {
        return Err("Stop or finish screen recording before starting Visual Analysis".into());
    }
    let state = app.state::<AppState>();
    if state.settings.locked().external_agent {
        return Err("Visual Analysis uses API mode. Select API mode first.".into());
    }
    if crate::events::agent_state(app).is_running() || state.lease.locked().is_held() {
        return Err("Finish or stop the current task first".into());
    }
    if crate::analysis::state::in_flight(state.analysis.locked().status) {
        return Err("Another analysis is running".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn playground_visual_plan(
    app: AppHandle,
    run_id: u64,
    input: VisualPlanInput,
) -> Result<VisualPlanResult, String> {
    let state = app.state::<PlaygroundRuns>();
    let token = state
        .runs
        .locked()
        .get(&run_id)
        .cloned()
        .ok_or("Playground run was cancelled")?;
    let work_app = app.clone();
    state
        .run(run_id, async move {
            visual_guard(&work_app)?;
            if input.target.is_some() == input.image.is_some() {
                return Err("Choose either a screen or window, or an image".into());
            }
            if input.question.chars().count() > 400 {
                return Err("Question exceeds 400 characters".into());
            }
            let settings = visual_settings(&work_app, &input.model).await?;
            let max_edge = settings.model_image_resolution.max_edge();
            let capture_edge = settings.recording_resolution.max_edge();
            let frame = tokio::time::timeout(
                Duration::from_secs(10),
                tauri::async_runtime::spawn_blocking(move || match (input.target, input.image) {
                    (Some(target), None) => {
                        screenshot::insight_capture(target, max_edge, capture_edge).map_err(
                            |error| {
                                format!("Capture permission denied or target unavailable: {error}")
                            },
                        )
                    }
                    (None, Some(data)) => depth_image(&data).and_then(|bytes| {
                        screenshot::insight_image(&bytes, max_edge, capture_edge)
                            .map_err(|error| error.to_string())
                    }),
                    _ => Err("Choose either a screen or window, or an image".into()),
                }),
            )
            .await
            .map_err(|_| "Reading the image timed out")?
            .map_err(|error| error.to_string())??;
            let questions = crate::analysis::run::plan_visual_questions(
                &work_app,
                &input.question,
                &frame.data_uri(),
                &token,
                &settings,
            )
            .await?;
            Ok(VisualPlanResult {
                image: format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&frame.png)
                ),
                questions,
                model: settings.models.analysis,
            })
        })
        .await
}

#[tauri::command]
pub async fn playground_visual_branch(
    app: AppHandle,
    run_id: u64,
    input: VisualBranchInput,
) -> Result<VisualBranchResult, String> {
    let state = app.state::<PlaygroundRuns>();
    let work_app = app.clone();
    state
        .run(run_id, async move {
            visual_guard(&work_app)?;
            run_visual_operation(&work_app, input).await
        })
        .await
}

pub(crate) async fn run_visual_operation(
    app: &AppHandle,
    input: VisualBranchInput,
) -> Result<VisualBranchResult, String> {
    let (task, endpoint, features) = visual_branch_contract(&input.role)?;
    let prepared = prepare(app, input.target, input.image, &input.model, task).await?;
    if input.role != "regions" {
        let (_, _, _, model) = session(app, &input.model, task, None).await?;
        validate_visual_capability(&input.role, &model.raw_capabilities)?;
    }
    let url = infer_url(&prepared.settings, endpoint)?;
    let mut body = serde_json::json!({"model": prepared.model, "image": prepared.frame.data_uri()});
    if !features.is_empty() {
        body["features"] = serde_json::json!(features);
    }
    let response = post(app, prepared.key.as_deref(), url, body, "visual analysis").await?;
    let result: serde_json::Value = serde_json::from_str(&response)
        .map_err(|_| "Infer returned an unreadable visual analysis result")?;
    let valid = if input.role == "regions" {
        result
            .get("class_map")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.starts_with("data:image/png;base64,"))
            && result.get("classes").and_then(|v| v.as_array()).is_some()
    } else {
        let has_size = input.role != "oriented"
            || ["/usage/image_width", "/usage/image_height"]
                .iter()
                .all(|path| {
                    result
                        .pointer(path)
                        .and_then(|v| v.as_u64())
                        .is_some_and(|v| v > 0)
                });
        has_size
            && result
                .get("data")
                .and_then(|v| v.as_array())
                .is_some_and(|items| {
                    items.iter().all(|item| match input.role.as_str() {
                        "instances" => item
                            .get("mask")
                            .and_then(|v| v.as_str())
                            .is_some_and(|s| s.starts_with("data:image/png;base64,")),
                        "oriented" => item
                            .pointer("/oriented_box/corners_pixels")
                            .and_then(|v| v.as_array())
                            .is_some_and(|points| {
                                points.len() == 4
                                    && points.iter().all(|point| {
                                        point.as_array().is_some_and(|coords| {
                                            coords.len() == 2
                                                && coords.iter().all(|value| {
                                                    value.as_f64().is_some_and(f64::is_finite)
                                                })
                                        })
                                    })
                            }),
                        _ => true,
                    })
                })
    };
    if !valid {
        return Err("Infer returned an invalid visual analysis result".into());
    }
    Ok(VisualBranchResult {
        image: prepared.frame.data_uri(),
        result,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImageVoiceInput {
    image: String,
    caption_model: String,
    speech_model: String,
    detail: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FlowProgress {
    run_id: u64,
    stage: &'static str,
    caption: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageVoiceResult {
    caption: String,
    caption_model: String,
    speech: Option<SpeechResult>,
    error: Option<String>,
}

#[tauri::command]
pub async fn playground_image_voice_flow(
    app: AppHandle,
    run_id: u64,
    input: ImageVoiceInput,
) -> Result<ImageVoiceResult, String> {
    let state = app.state::<PlaygroundRuns>();
    let token = state
        .runs
        .locked()
        .get(&run_id)
        .cloned()
        .ok_or("Playground run was cancelled")?;
    let work_app = app.clone();
    state
        .run(run_id, async move {
            session(&work_app, &input.caption_model, CAPTION_TASK, None).await?;
            session(
                &work_app,
                &input.speech_model,
                SPEECH_TASK,
                Some(AudioOperation::Speech),
            )
            .await?;
            let caption_app = work_app.clone();
            let speech_app = work_app.clone();
            image_voice_steps(
                &token,
                caption(
                    caption_app,
                    CaptionInput {
                        target: None,
                        image: Some(input.image),
                        model: input.caption_model,
                        detail: input.detail,
                    },
                ),
                move |text| {
                    speech(
                        speech_app,
                        SpeechInput {
                            text,
                            model: input.speech_model,
                        },
                    )
                },
                |stage, caption| {
                    if let Some(window) = work_app.get_webview_window(WINDOW) {
                        let _ = window.emit(
                            "playground-flow-progress",
                            FlowProgress {
                                run_id,
                                stage,
                                caption,
                            },
                        );
                    }
                },
            )
            .await
        })
        .await
}

async fn image_voice_steps<C, F, S, P>(
    token: &CancelToken,
    caption_call: C,
    speech_call: F,
    mut progress: P,
) -> Result<ImageVoiceResult, String>
where
    C: Future<Output = Result<CaptionResult, String>>,
    F: FnOnce(String) -> S,
    S: Future<Output = Result<SpeechResult, String>>,
    P: FnMut(&'static str, Option<String>),
{
    if token.is_cancelled() {
        return Err("Playground run was cancelled".into());
    }
    progress("caption", None);
    let result = caption_call.await?;
    if token.is_cancelled() {
        return Err("Playground run was cancelled".into());
    }
    let text = result.caption;
    progress("speech", Some(text.clone()));
    if token.is_cancelled() {
        return Err("Playground run was cancelled".into());
    }
    let output = match validate_speech_text(&text) {
        Err(err) => Err(err.to_string()),
        Ok(()) => speech_call(text.clone()).await,
    };
    if token.is_cancelled() {
        return Err("Playground run was cancelled".into());
    }
    let (speech, error) = match output {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(error)),
    };
    Ok(ImageVoiceResult {
        caption: text,
        caption_model: result.model,
        speech,
        error,
    })
}

fn open_with_flow(app: &AppHandle, flow: Option<&str>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW) {
        if let Some(flow) = flow {
            window.emit("playground-select-flow", flow)?;
        }
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(
        app,
        WINDOW,
        WebviewUrl::App(
            match flow {
                Some("image-voice") => "index.html?view=playground&flow=image-voice",
                Some("screen-insight") => "index.html?view=playground&flow=screen-insight",
                _ => "index.html?view=playground",
            }
            .into(),
        ),
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
pub async fn open_playground(app: AppHandle, flow: Option<String>) -> Result<(), String> {
    if flow
        .as_deref()
        .is_some_and(|id| !matches!(id, "image-voice" | "screen-insight"))
    {
        return Err("Unknown Flow".into());
    }
    open_with_flow(&app, flow.as_deref()).map_err(|err| format!("Could not open Playground: {err}"))
}

const DEPTH_TASK: &str = "image-to-depth";
const CAPTION_TASK: &str = "image-to-text";
// Some vision models spend part of their output budget on reasoning before the caption.
const CAPTION_MAX_TOKENS: u32 = 1024;
const UPSCALE_TASK: &str = "image-upscale";
const CREATE_TASK: &str = "text-to-image";
const EDIT_TASK: &str = "text-image-to-image";
const SPEECH_TASK: &str = "text-to-audio";
const TRANSCRIBE_TASK: &str = "audio-to-text";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);
const MAX_IMAGE_CHARS: usize = 40_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DepthInput {
    #[serde(default)]
    pub(crate) target: Option<CaptureTarget>,
    #[serde(default)]
    pub(crate) image: Option<String>,
    pub(crate) model: String,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DepthResult {
    original: String,
    pub(crate) preview: Option<String>,
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
        .filter(|m| m.tasks.iter().any(|t| task_matches(t, task)))
        .map(|m| m.id.clone())
        .collect()
}

fn task_matches(advertised: &str, requested: &str) -> bool {
    if requested != CAPTION_TASK {
        return advertised == requested;
    }
    advertised
        .split_once("-to-")
        .is_some_and(|(inputs, output)| {
            output == "text" && inputs.split('-').any(|input| input == "image")
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CaptionInput {
    #[serde(default)]
    target: Option<CaptureTarget>,
    #[serde(default)]
    image: Option<String>,
    model: String,
    detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionResult {
    image: String,
    caption: String,
    model: String,
    elapsed_ms: u64,
}

#[derive(Deserialize)]
struct CaptionEnvelope {
    caption: String,
    model: String,
}

fn caption_result(body: &str, image: String, elapsed_ms: u64) -> Result<CaptionResult, String> {
    let result: CaptionEnvelope =
        serde_json::from_str(body).map_err(|_| "Infer returned an unreadable caption")?;
    if result.caption.trim().is_empty() {
        return Err(
            "Infer returned an empty description. Retry or choose another caption model.".into(),
        );
    }
    if result.model.trim().is_empty() {
        return Err("Infer did not identify the caption model.".into());
    }
    Ok(CaptionResult {
        image,
        caption: result.caption,
        model: result.model,
        elapsed_ms,
    })
}

async fn caption(app: AppHandle, input: CaptionInput) -> Result<CaptionResult, String> {
    if !matches!(input.detail.as_str(), "concise" | "detailed") {
        return Err("Choose concise or detailed captioning".into());
    }
    let prepared = prepare(&app, input.target, input.image, &input.model, CAPTION_TASK).await?;
    let image = prepared.frame.data_uri();
    let url = infer_url(&prepared.settings, "/v1/images/captions")?;
    let state = app.state::<AppState>();
    send_caption(
        &state.http,
        prepared.key.as_deref(),
        url,
        prepared.model,
        image,
        input.detail,
    )
    .await
}

async fn send_caption(
    http: &reqwest::Client,
    key: Option<&str>,
    url: reqwest::Url,
    model: String,
    image: String,
    detail: String,
) -> Result<CaptionResult, String> {
    let started = Instant::now();
    let body = serde_json::json!({"model": model, "image": image, "detail": detail, "max_tokens": CAPTION_MAX_TOKENS});
    let response = post_json(http, key, url, body, "caption").await?;
    caption_result(&response, image, started.elapsed().as_millis() as u64)
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
    operation: Option<AudioOperation>,
) -> Result<(Settings, Option<String>, String, ModelCapabilities), String> {
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
    let capabilities = status
        .models
        .iter()
        .find(|item| item.id == model)
        .ok_or("Model is unavailable")?;
    if let Some(operation) = operation {
        validate_audio_operation(capabilities, operation)?;
    }
    Ok((settings, key, model, capabilities.clone()))
}

#[derive(Clone, Copy)]
enum AudioOperation {
    Transcription,
    Translation,
    Speech,
}

fn validate_audio_operation(
    model: &ModelCapabilities,
    operation: AudioOperation,
) -> Result<(), String> {
    let (name, support): (&str, &OperationSupport) = match operation {
        AudioOperation::Transcription => ("transcription", &model.audio_operations.transcription),
        AudioOperation::Translation => (
            "speech translation",
            &model.audio_operations.speech_translation,
        ),
        AudioOperation::Speech => (
            "speech generation",
            &model.audio_operations.speech_generation,
        ),
    };
    if support.status == OperationStatus::Supported {
        return Ok(());
    }
    let reason = support
        .reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
        .unwrap_or("Support has not been verified by Infer");
    Err(format!("Model '{}' cannot run {name}: {reason}", model.id))
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
    let (settings, key, model, _) = session(app, model, task, None).await?;
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
    post_json(&state.http, key, url, body, what).await
}

async fn post_json(
    http: &reqwest::Client,
    key: Option<&str>,
    url: reqwest::Url,
    body: serde_json::Value,
    what: &str,
) -> Result<String, String> {
    let mut request = http.post(url).timeout(REQUEST_TIMEOUT).json(&body);
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
        let detail = infer_error_detail(&text);
        return Err(format!(
            "Infer refused the {what} request ({code}): {detail}"
        ));
    }
    Ok(text)
}

fn infer_error_detail(body: &str) -> String {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(body);
    let message = parsed
        .as_ref()
        .ok()
        .and_then(|value| value.pointer("/error/message"))
        .and_then(serde_json::Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or(body);
    let detail = message
        .find('{')
        .and_then(|at| {
            let nested: serde_json::Value = serde_json::from_str(&message[at..]).ok()?;
            let inner = nested
                .pointer("/error/message")
                .or_else(|| nested.get("message"))?
                .as_str()?
                .trim();
            if inner.is_empty() {
                return None;
            }
            Some(format!("{}{}", &message[..at], inner))
        })
        .unwrap_or_else(|| message.to_string());
    detail.chars().take(300).collect()
}

fn infer_url(settings: &Settings, path: &str) -> Result<reqwest::Url, String> {
    reqwest::Url::parse(&format!(
        "{}{path}",
        settings.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "The Infer address is not valid".into())
}

const MAX_SPEECH_BYTES: usize = 4096;
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TranscribeInput {
    audio: String,
    model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeResult {
    text: String,
    model: String,
    elapsed_ms: u64,
}

fn transcribe_audio(data: &str) -> Result<Vec<u8>, &'static str> {
    let body = data
        .split_once(',')
        .and_then(|(header, body)| header.ends_with(";base64").then_some(body))
        .ok_or("Choose a WAV file")?;
    if body.len() > MAX_AUDIO_BYTES.div_ceil(3) * 4 {
        return Err("Choose a WAV file no larger than 25 MB");
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|_| "Could not read the WAV file")?;
    if bytes.len() > MAX_AUDIO_BYTES {
        return Err("Choose a WAV file no larger than 25 MB");
    }
    if !is_transcription_wav(&bytes) {
        return Err("Choose a 16 kHz mono 16-bit PCM WAV file");
    }
    Ok(bytes)
}

fn is_transcription_wav(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return false;
    }
    let declared = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if declared.checked_add(8) != Some(bytes.len()) {
        return false;
    }
    let mut pos = 12usize;
    let mut valid_format = false;
    let mut has_audio = false;
    while pos + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        let Some(end) = start.checked_add(len).filter(|end| *end <= bytes.len()) else {
            return false;
        };
        if &bytes[pos..pos + 4] == b"fmt " && len >= 16 {
            valid_format = bytes[start..start + 2] == 1u16.to_le_bytes()
                && bytes[start + 2..start + 4] == 1u16.to_le_bytes()
                && bytes[start + 4..start + 8] == 16000u32.to_le_bytes()
                && bytes[start + 14..start + 16] == 16u16.to_le_bytes();
        }
        if &bytes[pos..pos + 4] == b"data" && len > 0 {
            has_audio = true;
        }
        let Some(next) = end.checked_add(len % 2).filter(|next| *next <= bytes.len()) else {
            return false;
        };
        pos = next;
    }
    pos == bytes.len() && valid_format && has_audio
}

async fn transcribe(app: AppHandle, input: TranscribeInput) -> Result<TranscribeResult, String> {
    audio_to_text(app, input, AudioTextOperation::Transcribe).await
}

async fn translate_speech(
    app: AppHandle,
    input: TranscribeInput,
) -> Result<TranscribeResult, String> {
    audio_to_text(app, input, AudioTextOperation::Translate).await
}

#[derive(Clone, Copy)]
enum AudioTextOperation {
    Transcribe,
    Translate,
}

impl AudioTextOperation {
    fn path(self) -> &'static str {
        match self {
            Self::Transcribe => "/v1/audio/transcriptions",
            Self::Translate => "/v1/audio/translations",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Transcribe => "Transcription",
            Self::Translate => "Translation",
        }
    }
}

async fn audio_to_text(
    app: AppHandle,
    input: TranscribeInput,
    operation: AudioTextOperation,
) -> Result<TranscribeResult, String> {
    let audio = transcribe_audio(&input.audio)?;
    let required = match operation {
        AudioTextOperation::Transcribe => AudioOperation::Transcription,
        AudioTextOperation::Translate => AudioOperation::Translation,
    };
    let (settings, key, model, _) =
        session(&app, &input.model, TRANSCRIBE_TASK, Some(required)).await?;
    let url = infer_url(&settings, operation.path())?;
    let state = app.state::<AppState>();
    send_audio_text(&state.http, url, key.as_deref(), model, audio, operation).await
}

async fn send_audio_text(
    http: &reqwest::Client,
    url: reqwest::Url,
    key: Option<&str>,
    model: String,
    audio: Vec<u8>,
    operation: AudioTextOperation,
) -> Result<TranscribeResult, String> {
    let form = reqwest::multipart::Form::new()
        .text("model", model.clone())
        .text("response_format", "json")
        .part(
            "file",
            reqwest::multipart::Part::bytes(audio)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| e.to_string())?,
        );
    let started = Instant::now();
    let mut request = http.post(url).timeout(REQUEST_TIMEOUT).multipart(form);
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("{} request failed: {e}", operation.label()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("{} response failed: {e}", operation.label()))?;
    if !status.is_success() {
        return Err(format!(
            "Infer refused the {} request ({status}): {}",
            operation.label().to_lowercase(),
            infer_error_detail(&text)
        ));
    }
    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "Infer returned unreadable text")?;
    let transcript = body
        .get("text")
        .and_then(serde_json::Value::as_str)
        .ok_or("Infer returned no text")?;
    Ok(TranscribeResult {
        text: transcript.into(),
        model,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SpeechInput {
    text: String,
    model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechResult {
    audio: String,
    text: String,
    model: String,
    elapsed_ms: u64,
}

fn validate_speech_text(text: &str) -> Result<(), &'static str> {
    if text.trim().is_empty() {
        return Err("Enter text to speak");
    }
    if text.len() > MAX_SPEECH_BYTES {
        return Err("Text exceeds 4096 UTF-8 bytes");
    }
    Ok(())
}

fn speech_audio(bytes: &[u8]) -> Result<String, &'static str> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Infer returned invalid WAV audio");
    }
    Ok(format!(
        "data:audio/wav;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

async fn speech(app: AppHandle, input: SpeechInput) -> Result<SpeechResult, String> {
    validate_speech_text(&input.text)?;
    let (settings, key, model, _) = session(
        &app,
        &input.model,
        SPEECH_TASK,
        Some(AudioOperation::Speech),
    )
    .await?;
    let url = infer_url(&settings, "/v1/audio/speech")?;
    let state = app.state::<AppState>();
    let started = Instant::now();
    let mut request = state
        .http
        .post(url)
        .timeout(REQUEST_TIMEOUT)
        .json(&serde_json::json!({
            "model": model, "input": input.text, "response_format": "wav"
        }));
    if let Some(key) = key.as_deref() {
        request = request.bearer_auth(key);
    }
    let mut response = request
        .send()
        .await
        .map_err(|e| format!("Speech request failed: {e}"))?;
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Speech response failed: {e}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_AUDIO_BYTES {
            return Err("Infer returned audio exceeding 25 MB".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        return Err(format!(
            "Infer refused the speech request ({status}): {}",
            infer_error_detail(&String::from_utf8_lossy(&bytes))
        ));
    }
    Ok(SpeechResult {
        audio: speech_audio(&bytes)?,
        text: input.text,
        model,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

pub(crate) async fn depth(app: AppHandle, input: DepthInput) -> Result<DepthResult, String> {
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

async fn upscale(app: AppHandle, input: UpscaleInput) -> Result<UpscaleResult, String> {
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

const CREATE_EDGE: std::ops::RangeInclusive<u32> = 256..=2048;

fn valid_size(size: &str) -> bool {
    let Some((width, height)) = size.split_once('x') else {
        return false;
    };
    [width, height].iter().all(|edge| {
        edge.parse::<u32>()
            .is_ok_and(|edge| CREATE_EDGE.contains(&edge) && edge % 8 == 0)
    })
}
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EditInput {
    #[serde(default)]
    target: Option<CaptureTarget>,
    #[serde(default)]
    image: Option<String>,
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VariationInput {
    #[serde(default)]
    target: Option<CaptureTarget>,
    #[serde(default)]
    image: Option<String>,
    model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReferenceInput {
    images: Vec<String>,
    model: String,
    prompt: String,
    #[serde(default)]
    format: ReferenceFormat,
    #[serde(default = "default_reference_quality")]
    quality: u8,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReferenceFormat {
    #[default]
    Jpeg,
    Png,
}

fn default_reference_quality() -> u8 {
    80
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditResult {
    original: String,
    image: String,
    prompt: String,
    model: String,
    elapsed_ms: u64,
}

fn edit_result(
    body: &str,
    original: String,
    prompt: String,
    model: String,
    elapsed_ms: u64,
) -> Result<EditResult, String> {
    let envelope: CreateEnvelope =
        serde_json::from_str(body).map_err(|_| "Infer returned an unreadable edited image")?;
    let encoded = envelope
        .data
        .into_iter()
        .find_map(|item| item.b64_json.filter(|s| !s.is_empty()))
        .ok_or("Infer returned no edited image")?;
    Ok(EditResult {
        original,
        image: image_uri(&encoded),
        prompt,
        model,
        elapsed_ms,
    })
}

struct EditImages {
    data: Vec<Vec<u8>>,
    format: ReferenceFormat,
}

async fn send_edit(
    http: &reqwest::Client,
    url: reqwest::Url,
    key: Option<&str>,
    model: String,
    prompt: Option<String>,
    images: EditImages,
    original: String,
) -> Result<EditResult, String> {
    let label = if prompt.is_some() {
        "image edit"
    } else {
        "image variation"
    };
    let mut form = reqwest::multipart::Form::new()
        .text("model", model.clone())
        .text("response_format", "b64_json");
    let (extension, mime) = match images.format {
        ReferenceFormat::Jpeg => ("jpg", "image/jpeg"),
        ReferenceFormat::Png => ("png", "image/png"),
    };
    for (index, image) in images.data.into_iter().enumerate() {
        form = form.part(
            if index == 0 { "image" } else { "image[]" },
            reqwest::multipart::Part::bytes(image)
                .file_name(format!("image-{}.{extension}", index + 1))
                .mime_str(mime)
                .map_err(|e| e.to_string())?,
        );
    }
    let form = if let Some(prompt) = &prompt {
        form.text("prompt", prompt.clone())
    } else {
        form
    };
    let started = Instant::now();
    let mut request = http.post(url).timeout(REQUEST_TIMEOUT).multipart(form);
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("{label} request failed: {e}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("{label} response failed: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "Infer refused the {label} request ({status}): {}",
            infer_error_detail(&body)
        ));
    }
    edit_result(
        &body,
        original,
        prompt.unwrap_or_default(),
        model,
        started.elapsed().as_millis() as u64,
    )
}

async fn edit(app: AppHandle, input: EditInput) -> Result<EditResult, String> {
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Describe the change to make".into());
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!("Prompt exceeds {MAX_PROMPT_CHARS} characters"));
    }
    let prepared = prepare(&app, input.target, input.image, &input.model, EDIT_TASK).await?;
    let original = prepared.frame.data_uri();
    let url = infer_url(&prepared.settings, "/v1/images/edits")?;
    send_edit(
        &app.state::<AppState>().http,
        url,
        prepared.key.as_deref(),
        prepared.model,
        Some(prompt),
        EditImages {
            data: vec![prepared.frame.jpeg],
            format: ReferenceFormat::Jpeg,
        },
        original,
    )
    .await
}

async fn variation(app: AppHandle, input: VariationInput) -> Result<EditResult, String> {
    let prepared = prepare(&app, input.target, input.image, &input.model, EDIT_TASK).await?;
    let original = prepared.frame.data_uri();
    let url = infer_url(&prepared.settings, "/v1/images/variations")?;
    send_edit(
        &app.state::<AppState>().http,
        url,
        prepared.key.as_deref(),
        prepared.model,
        None,
        EditImages {
            data: vec![prepared.frame.jpeg],
            format: ReferenceFormat::Jpeg,
        },
        original,
    )
    .await
}

async fn reference(app: AppHandle, input: ReferenceInput) -> Result<EditResult, String> {
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Describe how to use the images".into());
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!("Prompt exceeds {MAX_PROMPT_CHARS} characters"));
    }
    if !(2..=10).contains(&input.images.len()) {
        return Err("Choose between 2 and 10 images".into());
    }
    if !(1..=100).contains(&input.quality) {
        return Err("JPEG quality must be between 1 and 100".into());
    }
    let (settings, key, model, caps) = session(&app, &input.model, EDIT_TASK, None).await?;
    validate_reference_count(input.images.len(), caps.max_image_references)?;
    let max_edge = settings.model_image_resolution.max_edge();
    let capture_edge = settings.recording_resolution.max_edge();
    let format = input.format;
    let quality = input.quality;
    let frames = tokio::time::timeout(
        Duration::from_secs(30),
        tauri::async_runtime::spawn_blocking(move || {
            input
                .images
                .into_iter()
                .map(|data| prepare_reference_image(data, format, quality, max_edge, capture_edge))
                .collect::<Result<Vec<_>, String>>()
        }),
    )
    .await
    .map_err(|_| "Reading the images timed out")?
    .map_err(|e| e.to_string())??;
    let original = frames[0].0.clone();
    let images = frames.into_iter().map(|(_, bytes)| bytes).collect();
    let url = infer_url(&settings, "/v1/images/edits")?;
    send_edit(
        &app.state::<AppState>().http,
        url,
        key.as_deref(),
        model,
        Some(prompt),
        EditImages {
            data: images,
            format,
        },
        original,
    )
    .await
}

fn prepare_reference_image(
    data: String,
    format: ReferenceFormat,
    quality: u8,
    max_edge: u32,
    capture_edge: u32,
) -> Result<(String, Vec<u8>), String> {
    let bytes = depth_image(&data)?;
    if format == ReferenceFormat::Png
        && image::guess_format(&bytes).ok() != Some(image::ImageFormat::Png)
    {
        return Err("Choose a valid PNG image".into());
    }
    if format == ReferenceFormat::Png {
        let reader =
            image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
        let (width, height) = reader
            .into_dimensions()
            .map_err(|_| "Could not read the PNG image")?;
        if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 64_000_000 {
            return Err("The image is too large".into());
        }
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|_| "Could not read the PNG image")?;
        Ok((data, bytes))
    } else {
        let mut frame =
            screenshot::insight_image(&bytes, max_edge, capture_edge).map_err(|e| e.to_string())?;
        if quality != screenshot::MODEL_JPEG_QUALITY {
            frame.set_jpeg_quality(quality).map_err(|e| e.to_string())?;
        }
        Ok((frame.data_uri(), frame.jpeg))
    }
}

fn validate_reference_count(count: usize, advertised: Option<u64>) -> Result<(), String> {
    let limit = advertised.unwrap_or(2).min(10) as usize;
    if limit < 2 {
        return Err("This model does not support two reference images".into());
    }
    if count < 2 || count > limit {
        return Err(format!(
            "Choose at least 2 images and no more than {limit} for this model"
        ));
    }
    Ok(())
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

async fn create(app: AppHandle, input: CreateInput) -> Result<CreateResult, String> {
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Describe the image to create".into());
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!("Prompt exceeds {MAX_PROMPT_CHARS} characters"));
    }
    if !valid_size(&input.size) {
        return Err("Choose an image size between 256 and 2048 pixels".into());
    }
    let (settings, key, model, _) = session(&app, &input.model, CREATE_TASK, None).await?;
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
    #[test]
    fn visual_branch_contract_matches_infer_image_endpoints() {
        assert_eq!(
            super::visual_branch_contract("entities").unwrap(),
            (
                "image-to-detection",
                "/v1/images/detections",
                &["boxes"][..]
            )
        );
        assert_eq!(
            super::visual_branch_contract("structure").unwrap(),
            (
                "image-to-detection",
                "/v1/images/detections",
                &["boxes", "keypoints"][..]
            )
        );
        assert_eq!(
            super::visual_branch_contract("regions").unwrap(),
            ("image-to-segmentation", "/v1/images/segmentations", &[][..])
        );
        assert!(super::visual_branch_contract("unknown").is_err());
        assert_eq!(
            super::visual_branch_contract("instances").unwrap().2,
            &["boxes", "masks"]
        );
        assert_eq!(
            super::visual_branch_contract("oriented").unwrap().2,
            &["boxes", "oriented_boxes"]
        );
    }
    #[test]
    fn specialized_detection_models_are_not_used_as_plain_detectors() {
        for (role, capability) in [
            ("structure", "Pose Estimation"),
            ("instances", "Instance Segmentation"),
            ("oriented", "Oriented Bounding Boxes"),
        ] {
            let advertised = vec![capability.to_string()];
            assert!(super::validate_visual_capability("entities", &advertised).is_err());
            assert!(super::validate_visual_capability(role, &advertised).is_ok());
            assert!(super::validate_visual_capability(role, &[]).is_err());
        }
        assert!(super::validate_visual_capability("entities", &[]).is_ok());
    }
    #[tokio::test]
    async fn image_voice_flow_passes_caption_to_speech_and_keeps_partial_errors() {
        use super::*;
        let token = CancelToken::default();
        let make_caption = || async {
            Ok(CaptionResult {
                image: "image".into(),
                caption: "A red square".into(),
                model: "caption".into(),
                elapsed_ms: 1,
            })
        };
        let result = image_voice_steps(
            &token,
            make_caption(),
            |text| async move {
                assert_eq!(text, "A red square");
                Ok(SpeechResult {
                    audio: "wav".into(),
                    text,
                    model: "voice".into(),
                    elapsed_ms: 1,
                })
            },
            |_, _| {},
        )
        .await
        .unwrap();
        assert_eq!(result.caption, "A red square");
        assert!(result.speech.is_some());
        let result = image_voice_steps(
            &token,
            make_caption(),
            |_| async { Err("voice failed".into()) },
            |_, _| {},
        )
        .await
        .unwrap();
        assert_eq!(result.caption, "A red square");
        assert_eq!(result.error.as_deref(), Some("voice failed"));
    }

    #[tokio::test]
    async fn image_voice_flow_never_calls_speech_after_cancellation_or_oversized_caption() {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = AtomicUsize::new(0);
        let token = CancelToken::default();
        let make_caption = |text: String| async move {
            Ok(CaptionResult {
                image: "image".into(),
                caption: text,
                model: "caption".into(),
                elapsed_ms: 1,
            })
        };
        let speech = |_: String| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err("unexpected".into()) }
        };
        let failed = image_voice_steps(
            &token,
            async { Err("caption failed".into()) },
            speech,
            |_, _| {},
        )
        .await;
        assert!(failed.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let speech = |_: String| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err("unexpected".into()) }
        };
        let result = image_voice_steps(&token, make_caption("x".repeat(4097)), speech, |_, _| {})
            .await
            .unwrap();
        assert!(result.error.unwrap().contains("4096"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let speech = |_: String| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Err("unexpected".into()) }
        };
        let result = image_voice_steps(
            &token,
            make_caption("caption".into()),
            speech,
            |stage, _| {
                if stage == "speech" {
                    token.cancel();
                }
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn reference_input_defaults_to_jpeg_quality_80() {
        let value: super::ReferenceInput =
            serde_json::from_str(r#"{"images":[],"model":"m","prompt":"p"}"#).unwrap();
        assert!(value.format == super::ReferenceFormat::Jpeg);
        assert_eq!(value.quality, 80);
        let png: super::ReferenceInput = serde_json::from_str(
            r#"{"images":[],"model":"m","prompt":"p","format":"png","quality":95}"#,
        )
        .unwrap();
        assert!(png.format == super::ReferenceFormat::Png);
        assert_eq!(png.quality, 95);
        assert!(
            serde_json::from_str::<super::ReferenceInput>(
                r#"{"images":[],"model":"m","prompt":"p","format":"webp"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn reference_format_preserves_png_bytes_or_encodes_resized_jpeg() {
        use base64::Engine;
        let image = image::RgbaImage::from_pixel(16, 16, image::Rgba([15, 30, 45, 128]));
        let mut buffer = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        let bytes = buffer.into_inner();
        let data = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        );
        let (preview, png) = super::prepare_reference_image(
            data.clone(),
            super::ReferenceFormat::Png,
            80,
            8,
            u32::MAX,
        )
        .unwrap();
        assert_eq!(preview, data);
        assert_eq!(png, bytes);
        let (preview, jpeg) =
            super::prepare_reference_image(data, super::ReferenceFormat::Jpeg, 95, 8, u32::MAX)
                .unwrap();
        assert!(preview.starts_with("data:image/jpeg;base64,"));
        assert_eq!(
            image::guess_format(&jpeg).unwrap(),
            image::ImageFormat::Jpeg
        );
        assert_eq!(image::load_from_memory(&jpeg).unwrap().width(), 8);
    }
    use super::*;

    #[test]
    fn reference_counts_follow_advertised_limit_and_legacy_fallback() {
        assert!(validate_reference_count(2, None).is_ok());
        assert!(validate_reference_count(3, None).is_err());
        assert!(validate_reference_count(10, Some(10)).is_ok());
        assert!(validate_reference_count(11, Some(99)).is_err());
        assert!(validate_reference_count(3, Some(2)).is_err());
        assert!(validate_reference_count(2, Some(1)).is_err());
        assert!(validate_reference_count(1, Some(10)).is_err());
    }

    #[test]
    fn edit_requires_text_and_image_task_and_valid_output() {
        assert!(task_matches("text-image-to-image", EDIT_TASK));
        assert!(!task_matches("image-to-image", EDIT_TASK));
        assert!(!task_matches("text-to-image", EDIT_TASK));
        let result = edit_result(
            r#"{"data":[{"b64_json":"iVBORw=="}]}"#,
            "source".into(),
            "blue".into(),
            "edit".into(),
            10,
        )
        .unwrap();
        assert_eq!(result.image, "data:image/png;base64,iVBORw==");
        assert_eq!(result.original, "source");
        assert!(edit_result(r#"{"data":[{}]}"#, "".into(), "".into(), "".into(), 0).is_err());
    }

    #[tokio::test]
    async fn sends_edit_as_authenticated_multipart() {
        use std::io::{Read, Write};
        for (variation, count, format) in [
            (false, 1, ReferenceFormat::Jpeg),
            (true, 1, ReferenceFormat::Jpeg),
            (false, 2, ReferenceFormat::Jpeg),
            (false, 2, ReferenceFormat::Png),
            (false, 10, ReferenceFormat::Png),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    let n = stream.read(&mut buffer).unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..n]);
                    if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|n| n.trim().parse::<usize>().ok())
                            })
                            .unwrap();
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let response = br#"{"data":[{"b64_json":"iVBORw=="}]}"#;
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                    response.len()
                )
                .unwrap();
                stream.write_all(response).unwrap();
                String::from_utf8_lossy(&request).into_owned()
            });
            let result = send_edit(
                &reqwest::Client::builder().no_proxy().build().unwrap(),
                format!(
                    "http://{address}/v1/images/{}",
                    if variation { "variations" } else { "edits" }
                )
                .parse()
                .unwrap(),
                Some("test-key"),
                "edit-model".into(),
                (!variation).then(|| "Make blue".into()),
                EditImages {
                    data: std::iter::once(b"jpeg bytes".to_vec())
                        .chain((2..=count).map(|n| format!("reference-{n:02}").into_bytes()))
                        .collect(),
                    format,
                },
                "source".into(),
            )
            .await
            .unwrap();
            assert_eq!(result.prompt, if variation { "" } else { "Make blue" });
            let request = server.join().unwrap();
            assert!(request.starts_with(&format!(
                "POST /v1/images/{} ",
                if variation { "variations" } else { "edits" }
            )));
            assert!(
                request
                    .to_lowercase()
                    .contains("authorization: bearer test-key")
            );
            for expected in [
                "name=\"model\"",
                "edit-model",
                "name=\"response_format\"",
                "b64_json",
                "name=\"image\"",
                "jpeg bytes",
            ] {
                assert!(request.contains(expected), "missing {expected}");
            }
            assert!(request.contains(if format == ReferenceFormat::Png {
                "image/png"
            } else {
                "image/jpeg"
            }));
            assert!(request.contains(if format == ReferenceFormat::Png {
                "image-1.png"
            } else {
                "image-1.jpg"
            }));
            assert_eq!(request.contains("name=\"prompt\""), !variation);
            if count > 1 {
                assert!(request.contains("name=\"image[]\""));
                let mut previous = request.find("jpeg bytes").unwrap();
                for n in 2..=count {
                    let next = request.find(&format!("reference-{n:02}")).unwrap();
                    assert!(previous < next);
                    previous = next;
                }
                assert_eq!(request.matches("name=\"image[]\"").count(), count - 1);
            } else {
                assert!(!request.contains("name=\"image[]\""));
            }
        }
    }

    #[tokio::test]
    async fn sends_caption_request_to_infer_and_keeps_frame() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = br#"{"model":"vision","caption":"A red square."}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                response.len()
            )
            .unwrap();
            stream.write_all(response).unwrap();
            String::from_utf8(request).unwrap()
        });
        let url = format!("http://{address}/v1/images/captions")
            .parse()
            .unwrap();
        let result = send_caption(
            &reqwest::Client::builder().no_proxy().build().unwrap(),
            Some("test-key"),
            url,
            "vision".into(),
            "data:image/jpeg;base64,AA==".into(),
            "detailed".into(),
        )
        .await
        .unwrap();
        assert_eq!(result.caption, "A red square.");
        assert_eq!(result.image, "data:image/jpeg;base64,AA==");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/images/captions "));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer test-key")
        );
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"model":"vision","image":"data:image/jpeg;base64,AA==","detail":"detailed","max_tokens":1024})
        );
    }

    #[test]
    fn captions_accept_compatible_tasks_and_validate_results() {
        assert!(task_matches("image-to-text", CAPTION_TASK));
        assert!(task_matches("text-image-to-text", CAPTION_TASK));
        assert!(!task_matches("text-to-text", CAPTION_TASK));
        assert!(!task_matches("image-to-image", CAPTION_TASK));
        let result = caption_result(
            r#"{"model":"vision","caption":"A red square."}"#,
            "frame".into(),
            12,
        )
        .unwrap();
        assert_eq!(
            (
                result.model.as_str(),
                result.caption.as_str(),
                result.image.as_str()
            ),
            ("vision", "A red square.", "frame")
        );
        assert!(caption_result(r#"{"model":"vision","caption":"  "}"#, "frame".into(), 0).is_err());
        assert!(caption_result("invalid", "frame".into(), 0).is_err());
    }

    #[test]
    fn audio_operations_require_specific_verified_support() {
        let mut model = ModelCapabilities::unknown("asr");
        model.audio_operations.transcription.status = OperationStatus::Supported;
        model.audio_operations.speech_translation.status = OperationStatus::Unsupported;
        model.audio_operations.speech_translation.reason =
            Some("Translation is not supported".into());
        assert!(validate_audio_operation(&model, AudioOperation::Transcription).is_ok());
        assert!(
            validate_audio_operation(&model, AudioOperation::Translation)
                .unwrap_err()
                .contains("Translation is not supported")
        );
        assert!(
            validate_audio_operation(&model, AudioOperation::Speech)
                .unwrap_err()
                .contains("not been verified")
        );
    }

    #[tokio::test]
    async fn cancelling_a_run_is_isolated_and_drops_its_future() {
        let runs = std::sync::Arc::new(PlaygroundRuns::default());
        let first = runs.start();
        let second = runs.start();
        let active = runs.clone();
        let task = tokio::spawn(async move {
            active
                .run(first, std::future::pending::<Result<(), String>>())
                .await
        });
        tokio::task::yield_now().await;
        runs.cancel(first);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        assert!(runs.run(first, async { Ok(()) }).await.is_err());
        assert_eq!(runs.run(second, async { Ok(7) }).await.unwrap(), 7);
        assert!(runs.runs.locked().is_empty());
    }

    #[tokio::test]
    async fn sends_multipart_audio_and_reads_translation() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = b"{\"text\":\"hello\"}";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                response.len()
            )
            .unwrap();
            stream.write_all(response).unwrap();
            String::from_utf8_lossy(&request).into_owned()
        });
        let result = send_audio_text(
            &reqwest::Client::builder().no_proxy().build().unwrap(),
            format!("http://{address}{}", AudioTextOperation::Translate.path())
                .parse()
                .unwrap(),
            Some("test-key"),
            "asr-model".into(),
            b"audio bytes".to_vec(),
            AudioTextOperation::Translate,
        )
        .await
        .unwrap();
        assert_eq!(result.text, "hello");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/audio/translations "));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer test-key")
        );
        for expected in [
            "multipart/form-data",
            "name=\"model\"",
            "asr-model",
            "name=\"file\"",
            "audio bytes",
            "name=\"response_format\"",
        ] {
            assert!(request.contains(expected), "{expected}");
        }
    }

    #[test]
    fn accepts_only_mono_16khz_16bit_pcm_wav_for_transcription() {
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&38u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&16000u32.to_le_bytes());
        wav.extend_from_slice(&32000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2u32.to_le_bytes());
        wav.extend_from_slice(&[0, 0]);
        assert!(is_transcription_wav(&wav));
        let uri = format!(
            "data:audio/wav;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&wav)
        );
        assert_eq!(transcribe_audio(&uri).unwrap(), wav);
        wav[22] = 2;
        assert!(!is_transcription_wav(&wav));
        assert!(!is_transcription_wav(b"RIFF"));
        assert!(transcribe_audio("data:audio/wav;base64,invalid").is_err());
    }

    #[test]
    fn validates_speech_text_and_wav() {
        assert!(validate_speech_text("  ").is_err());
        assert!(validate_speech_text(&"é".repeat(2048)).is_ok());
        assert!(validate_speech_text(&"é".repeat(2049)).is_err());
        assert!(
            speech_audio(b"RIFF0000WAVE")
                .unwrap()
                .starts_with("data:audio/wav;base64,")
        );
        assert!(speech_audio(b"not audio").is_err());
    }

    #[test]
    fn selects_models_advertising_text_to_audio() {
        let mut speech = ModelCapabilities::unknown("audio-model");
        speech.tasks = vec!["text-to-audio".into()];
        let mut legacy = ModelCapabilities::unknown("legacy-model");
        legacy.tasks = vec!["text-to-speech".into()];
        assert_eq!(
            task_models(&[speech, legacy], SPEECH_TASK),
            vec!["audio-model"]
        );
    }

    #[test]
    fn extracts_infer_error_messages_and_preserves_other_bodies() {
        let nested = serde_json::json!({"error":{"message": "nemo-speech translation failed: {\"error\":{\"message\":\"translation unsupported\"}}"}}).to_string();
        assert_eq!(
            infer_error_detail(&nested),
            "nemo-speech translation failed: translation unsupported"
        );
        assert_eq!(
            infer_error_detail(r#"{"error":{"message":"Unsupported size","code":"invalid"}}"#),
            "Unsupported size"
        );
        assert_eq!(
            infer_error_detail("Service unavailable"),
            "Service unavailable"
        );
        assert_eq!(infer_error_detail(&"x".repeat(301)).chars().count(), 300);
    }

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
    fn checks_image_sizes() {
        for size in ["256x256", "720x720", "1920x1080", "1000x1000", "2048x2048"] {
            assert!(valid_size(size), "expected valid image size: {size}");
        }
        for size in [
            "248x720",
            "720x2056",
            "4096x4096",
            "1001x1000",
            "1000x1001",
            "big",
            "720x720x720",
            "0x720",
        ] {
            assert!(!valid_size(size), "expected invalid image size: {size}");
        }
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
