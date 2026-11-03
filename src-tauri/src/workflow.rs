//! Small static workflow registry. No scheduler, graph or process ownership.
use crate::{
    analysis::{
        run::{
            ModelAnalysis, PLAYGROUND_FLOW_TIMEOUT, analyze_image_with_settings,
            analyze_image_with_settings_timeout, plan_visual_questions,
        },
        state::{StoredFrame, VisualClass, VisualMaps},
    },
    events::{self, AppState},
    exec::CancelToken,
    model::capabilities::{ModelCapabilities, TriState},
    observe::screenshot::{self, CaptureTarget},
    sync::LockExt,
};
use serde_json::Value;
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Manager};
pub struct Definition {
    pub id: &'static str,
    pub name: &'static str,
    pub capabilities: &'static [&'static str],
}
#[derive(Debug)]
pub struct Output {
    pub analysis: ModelAnalysis,
    pub frame: Option<StoredFrame>,
    pub captured_utc: String,
    pub visual_maps: VisualMaps,
}
type Work<'a> = Pin<Box<dyn Future<Output = Result<Output, String>> + Send + 'a>>;
trait Workflow: Sync {
    fn definition(&self) -> Definition;
    fn validate(&self, input: &Value) -> Result<(), String>;
    fn execute<'a>(
        &'a self,
        app: &'a AppHandle,
        input: Value,
        cancel: &'a CancelToken,
        id: &'a str,
    ) -> Work<'a>;
}
struct ScreenInsight;
static SCREEN_INSIGHT: ScreenInsight = ScreenInsight;
static WORKFLOWS: &[&dyn Workflow] = &[&SCREEN_INSIGHT];
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ScreenInput {
    #[serde(default)]
    target: Option<CaptureTarget>,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    question: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    resolution: Option<crate::settings::ImageResolution>,
    #[serde(default)]
    shared_visual: bool,
}
const IMAGE_PREFIX: &str = "data:image/png;base64,";
const MAX_IMAGE_CHARS: usize = 40_000_000;

fn image_bytes(data: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let body = data
        .strip_prefix(IMAGE_PREFIX)
        .ok_or("Choose a PNG image")?;
    if body.len() > MAX_IMAGE_CHARS {
        return Err("The image is too large".into());
    }
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|_| "The image data is not valid".into())
}
pub fn vision_model(
    settings: &crate::settings::Settings,
    models: &[ModelCapabilities],
) -> Result<String, String> {
    let name = settings.models.analysis.trim();
    if name.is_empty() {
        return Err(
            "Configure an explicit Analysis model in Agents settings before running Visual Analysis"
                .into(),
        );
    }
    let model = models
        .iter()
        .find(|m| m.id == name)
        .ok_or("Configured Analysis model is not available from Infer")?;
    if model.image_in != TriState::Yes || model.tool_calls != TriState::Yes {
        return Err(
            "Visual Analysis requires a model explicitly advertising image input and tool calling"
                .into(),
        );
    }
    Ok(name.into())
}
impl Workflow for ScreenInsight {
    fn definition(&self) -> Definition {
        Definition {
            id: "screen-insight",
            name: "Visual Analysis",
            capabilities: &["screen_capture", "vision_model"],
        }
    }
    fn validate(&self, input: &Value) -> Result<(), String> {
        if let Some(resolution) = input.get("resolution").filter(|v| !v.is_null())
            && !matches!(
                resolution.as_str(),
                Some("720" | "1280" | "1920" | "2560" | "native")
            )
        {
            return Err("Choose a supported image resolution".into());
        }
        let input: ScreenInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Choose a screen, window or image and a question: {e}"))?;
        match (&input.target, &input.image) {
            (Some(_), None) => {}
            (None, Some(data)) => {
                if !data.starts_with(IMAGE_PREFIX) || data.len() > MAX_IMAGE_CHARS {
                    return Err("Choose a PNG image".into());
                }
            }
            _ => return Err("Choose either a screen or window, or an image".into()),
        }
        if input.question.chars().count() > 500 {
            return Err("Question exceeds 500 characters".into());
        }
        Ok(())
    }
    fn execute<'a>(
        &'a self,
        app: &'a AppHandle,
        input: Value,
        cancel: &'a CancelToken,
        id: &'a str,
    ) -> Work<'a> {
        Box::pin(async move {
            let input: ScreenInput = serde_json::from_value(input).map_err(|e| e.to_string())?;
            let state = app.state::<AppState>();
            let (mut settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
            if let Some(model) = input
                .model
                .as_deref()
                .map(str::trim)
                .filter(|m| !m.is_empty())
            {
                settings.models.analysis = model.into();
            }
            if input.shared_visual {
                let planner = settings.visual_analysis_models.planner.trim();
                if planner.is_empty() {
                    return Err(
                        "Choose a Visual Analysis planning model in Settings → Capture & privacy"
                            .into(),
                    );
                }
                if [
                    &settings.visual_analysis_models.entities,
                    &settings.visual_analysis_models.regions,
                    &settings.visual_analysis_models.structure,
                    &settings.visual_analysis_models.depth,
                    &settings.visual_analysis_models.instances,
                    &settings.visual_analysis_models.oriented,
                ]
                .iter()
                .all(|model| model.trim().is_empty())
                {
                    return Err(
                        "Enable at least one Visual Analysis model in Settings → Capture & privacy"
                            .into(),
                    );
                }
                settings.models.analysis = planner.to_string();
            }
            if let Some(resolution) = input.resolution {
                settings.model_image_resolution = resolution;
            }
            let status = crate::backend::check(&state.http, &settings, key.as_deref()).await;
            if !status.reachable || !status.auth_ok {
                return Err(status
                    .error
                    .unwrap_or("Infer is unavailable or authentication failed".into()));
            }
            let model = vision_model(&settings, &status.models)?;
            if !crate::analysis::run::current(app, id) {
                return Err("cancelled".into());
            }
            {
                let mut analysis = state.analysis.locked();
                if analysis.id != id {
                    return Err("cancelled".into());
                }
                analysis.model = model.clone();
            }
            let captured_utc = crate::trace::now_utc();
            let max_edge = settings.model_image_resolution.max_edge();
            let capture_edge = settings.recording_resolution.max_edge();
            let from_image = input.image.is_some();
            let frame = tokio::time::timeout(
                Duration::from_secs(10),
                tauri::async_runtime::spawn_blocking(move || match (input.target, input.image) {
                    (Some(target), _) => {
                        screenshot::insight_capture(target, max_edge, capture_edge).map_err(|e| {
                            format!("Capture permission denied or target unavailable: {e}")
                        })
                    }
                    (None, Some(data)) => image_bytes(&data).and_then(|bytes| {
                        screenshot::insight_image(&bytes, max_edge, capture_edge)
                            .map_err(|e| e.to_string())
                    }),
                    (None, None) => Err("Choose a screen, window or image".into()),
                }),
            )
            .await
            .map_err(|_| {
                if from_image {
                    "Reading the image timed out"
                } else {
                    "Screen/window capture timed out"
                }
            })?
            .map_err(|e| e.to_string())??;
            let image = frame.data_uri();
            let signature = frame.signature();
            let stored = StoredFrame {
                jpeg: std::sync::Arc::new(frame.jpeg),
                png: std::sync::Arc::new(frame.png),
                monitor: frame.monitor,
                signature,
                front: None,
            };
            if !crate::analysis::run::current(app, id) {
                return Err("cancelled".into());
            }
            {
                let mut analysis = state.analysis.locked();
                if analysis.id != id {
                    return Err("cancelled".into());
                }
                analysis.frame = Some(stored.clone());
                analysis.created_utc = captured_utc.clone();
                analysis.model = model.clone();
                analysis.status = crate::analysis::report::Status::Asking;
            }
            crate::analysis::run::publish(app);
            let question = crate::analysis::report::question_or_default(&input.question);
            let mut maps = VisualMaps::default();
            let analysis = if input.shared_visual {
                plan_visual_questions(app, &question, &image, cancel, &settings).await?;
                let png = format!("data:image/png;base64,{}", {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD.encode(stored.png.as_ref())
                });
                let selected = &settings.visual_analysis_models;
                let run = |role: &'static str, model: &str| {
                    let app = app.clone();
                    let model = model.to_string();
                    let image = png.clone();
                    async move {
                        if model.trim().is_empty() {
                            return None;
                        }
                        Some(
                            crate::playground::run_visual_operation(
                                &app,
                                crate::playground::VisualBranchInput {
                                    target: None,
                                    image: Some(image),
                                    model,
                                    role: role.into(),
                                },
                            )
                            .await,
                        )
                    }
                };
                let depth = async {
                    if selected.depth.trim().is_empty() {
                        return None;
                    }
                    Some(
                        crate::playground::depth(
                            app.clone(),
                            crate::playground::DepthInput {
                                target: None,
                                image: Some(png.clone()),
                                model: selected.depth.clone(),
                            },
                        )
                        .await,
                    )
                };
                let (answer, entities, regions, structure, instances, oriented, depth) = tokio::join!(
                    analyze_image_with_settings_timeout(
                        app,
                        &question,
                        &image,
                        cancel,
                        id,
                        &settings,
                        PLAYGROUND_FLOW_TIMEOUT
                    ),
                    run("entities", &selected.entities),
                    run("regions", &selected.regions),
                    run("structure", &selected.structure),
                    run("instances", &selected.instances),
                    run("oriented", &selected.oriented),
                    depth
                );
                let mut answer = match answer {
                    Ok(answer) => answer,
                    Err(error) => {
                        maps.errors.push(format!("Answer: {error}"));
                        ModelAnalysis {
                            model: settings.models.analysis.clone(),
                            parsed: crate::analysis::report::ParsedReport {
                                answer: "The question could not be answered; available Operation results are shown below.".into(),
                                findings: Vec::new(),
                            },
                        }
                    }
                };
                answer.parsed.findings.clear();
                for (role, result) in [
                    ("entities", entities),
                    ("regions", regions),
                    ("structure", structure),
                    ("instances", instances),
                    ("oriented", oriented),
                ] {
                    match result {
                        Some(Ok(value)) if role == "regions" => {
                            maps.segmentation = value
                                .result
                                .get("class_map")
                                .and_then(Value::as_str)
                                .map(str::to_string);
                            maps.classes = value
                                .result
                                .get("classes")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(|item| {
                                    Some(VisualClass {
                                        id: u8::try_from(item.get("id")?.as_u64()?).ok()?,
                                        label: item.get("label")?.as_str()?.to_string(),
                                    })
                                })
                                .collect();
                        }
                        Some(Ok(value)) => {
                            if role == "instances" || role == "oriented" {
                                collect_specialized(&mut maps, &value.result, role);
                            }
                            append_visual_findings(&mut answer.parsed.findings, &value.result, role)
                        }
                        Some(Err(error)) => maps.errors.push(format!("{role}: {error}")),
                        None => {}
                    }
                }
                match depth {
                    Some(Ok(value)) => maps.depth = value.preview,
                    Some(Err(error)) => maps.errors.push(format!("depth: {error}")),
                    None => {}
                }
                answer
            } else {
                analyze_image_with_settings(app, &question, &image, cancel, id, &settings)
                    .await
                    .map_err(|e| e.to_string())?
            };
            Ok(Output {
                analysis,
                frame: Some(stored),
                captured_utc,
                visual_maps: maps,
            })
        })
    }
}

fn collect_specialized(maps: &mut VisualMaps, value: &Value, role: &str) {
    let Some(items) = value.get("data").and_then(Value::as_array) else {
        return;
    };
    let width = value
        .pointer("/usage/image_width")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let height = value
        .pointer("/usage/image_height")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    for item in items {
        let label = item
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("object")
            .chars()
            .take(64)
            .collect();
        if role == "instances" {
            if let Some(mask) = item
                .get("mask")
                .and_then(Value::as_str)
                .filter(|mask| mask.starts_with("data:image/png;base64,"))
            {
                maps.instances.push(crate::analysis::state::VisualInstance {
                    mask: mask.into(),
                    label,
                });
            }
        } else if width > 0.0 && height > 0.0 {
            let Some(raw) = item
                .pointer("/oriented_box/corners_pixels")
                .and_then(Value::as_array)
                .filter(|a| a.len() == 4)
            else {
                continue;
            };
            let mut corners = [[0.0; 2]; 4];
            let mut valid = true;
            for (i, point) in raw.iter().enumerate() {
                match (
                    point.get(0).and_then(Value::as_f64),
                    point.get(1).and_then(Value::as_f64),
                ) {
                    (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
                        corners[i] = [x / width * 1000.0, y / height * 1000.0]
                    }
                    _ => valid = false,
                }
            }
            if valid {
                maps.oriented
                    .push(crate::analysis::state::VisualOriented { corners, label });
            }
        }
    }
}

fn append_visual_findings(
    findings: &mut Vec<crate::analysis::report::Finding>,
    value: &Value,
    role: &str,
) {
    use crate::analysis::report::{Finding, Group, Keypoint, Kind, MAX_FINDINGS, Pose};
    let Some(items) = value.get("data").and_then(Value::as_array) else {
        return;
    };
    for item in items {
        if findings.len() >= MAX_FINDINGS {
            break;
        }
        let Some(label) = item.get("label").and_then(Value::as_str) else {
            continue;
        };
        let b = &item["box"];
        let numbers = [
            b["x"].as_f64(),
            b["y"].as_f64(),
            b["width"].as_f64(),
            b["height"].as_f64(),
        ];
        let bbox = match numbers {
            [Some(x), Some(y), Some(w), Some(h)]
                if x.is_finite()
                    && y.is_finite()
                    && w.is_finite()
                    && h.is_finite()
                    && w > 0.0
                    && h > 0.0 =>
            {
                let clamp = |v: f64| (v * 1000.0).clamp(0.0, 1000.0) as f32;
                let b = [
                    clamp(x - w / 2.0),
                    clamp(y - h / 2.0),
                    clamp(x + w / 2.0),
                    clamp(y + h / 2.0),
                ];
                (b[0] < b[2] && b[1] < b[3]).then_some(b)
            }
            _ => None,
        };
        let mut names = Vec::new();
        let points: Vec<Keypoint> = item
            .get("pose")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|point| {
                let name = point.get("name")?.as_str()?;
                let x = point.get("x")?.as_f64()?;
                let y = point.get("y")?.as_f64()?;
                let confidence = point.get("confidence")?.as_f64()?;
                if name.is_empty()
                    || name.chars().count() > 24
                    || !(0.0..=1.0).contains(&x)
                    || !(0.0..=1.0).contains(&y)
                    || confidence < 0.5
                {
                    return None;
                }
                names.push(name.to_string());
                Some(Keypoint {
                    label: name.replace('_', " "),
                    x: (x * 1000.0) as f32,
                    y: (y * 1000.0) as f32,
                })
            })
            .take(32)
            .collect();
        let links = [
            ("nose", "left_eye"),
            ("nose", "right_eye"),
            ("left_eye", "left_ear"),
            ("right_eye", "right_ear"),
            ("left_shoulder", "right_shoulder"),
            ("left_shoulder", "left_elbow"),
            ("left_elbow", "left_wrist"),
            ("right_shoulder", "right_elbow"),
            ("right_elbow", "right_wrist"),
            ("left_shoulder", "left_hip"),
            ("right_shoulder", "right_hip"),
            ("left_hip", "right_hip"),
            ("left_hip", "left_knee"),
            ("left_knee", "left_ankle"),
            ("right_hip", "right_knee"),
            ("right_knee", "right_ankle"),
        ];
        let connections = links
            .iter()
            .filter_map(|(a, b)| {
                Some([
                    names.iter().position(|v| v == a)?,
                    names.iter().position(|v| v == b)?,
                ])
            })
            .collect();
        let pose = (!points.is_empty()).then_some(Pose {
            keypoints: points,
            connections,
            rejected: 0,
        });
        let confidence = item
            .get("confidence")
            .and_then(Value::as_f64)
            .map(|c| format!("{:.0}% confidence", c * 100.0))
            .unwrap_or_default();
        findings.push(Finding {
            n: (findings.len() + 1) as u8,
            label: label.chars().take(24).collect(),
            kind: Kind::Image,
            group: if role == "structure" {
                Group::Structure
            } else {
                Group::Entities
            },
            bbox,
            pose,
            description: confidence,
            evidence: String::new(),
            point: None,
        });
    }
}
async fn bounded<F: Future<Output = Result<Output, String>>>(
    future: F,
    cancel: &CancelToken,
    timeout: Duration,
) -> Result<Output, String> {
    match crate::agent::unless_stopped(cancel, tokio::time::timeout(timeout, future)).await {
        None => Err("cancelled".into()),
        Some(Err(_)) => Err("Workflow timed out".into()),
        Some(Ok(result)) => result,
    }
}
pub async fn run(app: &AppHandle, workflow_id: &str, input: Value) -> Result<(), String> {
    let workflow = WORKFLOWS
        .iter()
        .find(|w| w.definition().id == workflow_id)
        .ok_or("Unknown workflow")?;
    workflow.validate(&input)?;
    if crate::recording::busy() {
        return Err("Stop or finish screen recording before starting Visual Analysis".into());
    }
    let state = app.state::<AppState>();
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = format!(
        "insight-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let cancel = {
        let mut analysis = state.analysis.locked();
        if state.settings.locked().external_agent {
            return Err(
                "Visual Analysis uses API mode. Finish the task and select API mode first.".into(),
            );
        }
        if events::agent_state(app).is_running() || state.lease.locked().is_held() {
            return Err("Finish or stop the current task first".into());
        }
        if crate::analysis::state::in_flight(analysis.status)
            && !analysis.id.starts_with("insight-")
        {
            return Err("Another analysis is running".into());
        }
        analysis.cancel.cancel();
        analysis.begin(
            &crate::analysis::report::question_or_default(input["question"].as_str().unwrap_or("")),
            true,
        );
        analysis.frame = None;
        analysis.id = id.clone();
        let settings = state.settings.locked().clone();
        let (_, settings_json) = crate::trace::model_and_snapshot(&state.settings);
        state.trace.begin_run(crate::trace::RunInfo {
            id: id.clone(),
            scope_id: None,
            task: analysis.question.clone(),
            source: workflow.definition().id.into(),
            started_utc: crate::trace::now_utc(),
            plan_json: None,
            model: (!settings.models.analysis.trim().is_empty())
                .then(|| settings.models.analysis.clone()),
            app_version: Some(crate::VERSION.into()),
            settings_json: Some(settings_json),
            session_dir: None,
        });
        state
            .trace
            .user_prompt(&id, input["question"].as_str().unwrap_or(""));
        analysis.cancel.clone()
    };
    crate::analysis::run::publish(app);
    let definition = workflow.definition();
    log::info!(
        "workflow {} ({}) requires {:?}",
        definition.id,
        definition.name,
        definition.capabilities
    );
    let started = std::time::Instant::now();
    let timeout = if input
        .get("sharedVisual")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        1200
    } else {
        90
    };
    let result = bounded(
        workflow.execute(app, input, &cancel, &id),
        &cancel,
        Duration::from_secs(timeout),
    )
    .await;
    let (outcome, summary) = match &result {
        Ok(output) if !cancel.is_cancelled() => {
            ("succeeded", output.analysis.parsed.answer.clone())
        }
        Ok(_) => ("stopped", String::new()),
        Err(error) if error == "cancelled" => ("stopped", String::new()),
        Err(error) => ("failed", error.clone()),
    };
    crate::analysis::run::finish_workflow(app, &id, result, started.elapsed().as_millis() as u64);
    state.trace.end_run(&id, outcome, &summary);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_instance_masks_and_normalizes_oriented_corners() {
        let mut maps = VisualMaps::default();
        collect_specialized(
            &mut maps,
            &serde_json::json!({"data":[{"label":"person","mask":"data:image/png;base64,AA=="}]}),
            "instances",
        );
        assert_eq!(maps.instances.len(), 1);
        collect_specialized(
            &mut maps,
            &serde_json::json!({"usage":{"image_width":200,"image_height":100},"data":[{"label":"boat","oriented_box":{"corners_pixels":[[0,0],[200,0],[200,100],[0,100]]}}]}),
            "oriented",
        );
        assert_eq!(
            maps.oriented[0].corners,
            [[0.0, 0.0], [1000.0, 0.0], [1000.0, 1000.0], [0.0, 1000.0]]
        );
    }
    #[test]
    fn infer_detections_become_screen_annotations() {
        let mut findings = Vec::new();
        append_visual_findings(
            &mut findings,
            &serde_json::json!({"data":[{
                "label":"person", "confidence":0.9, "box":{"x":0.5,"y":0.5,"width":0.4,"height":0.6},
                "pose":[{"name":"left_shoulder","x":0.3,"y":0.4,"confidence":0.9},{"name":"right_shoulder","x":0.7,"y":0.4,"confidence":0.8},{"name":"left_elbow","x":0.2,"y":0.6,"confidence":0.1}]
            }]}),
            "structure",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].bbox, Some([300.0, 200.0, 700.0, 800.0]));
        let pose = findings[0].pose.as_ref().unwrap();
        assert_eq!(pose.keypoints.len(), 2);
        assert_eq!(pose.connections, vec![[0, 1]]);
        assert_eq!(findings[0].group, crate::analysis::report::Group::Structure);
    }
    #[test]
    fn registration_validation_and_explicit_vision_requirements() {
        assert_eq!(WORKFLOWS.len(), 1);
        assert!(
            SCREEN_INSIGHT
                .validate(&serde_json::json!({"target":{"kind":"screen","id":1}}))
                .is_ok()
        );
        assert!(
            SCREEN_INSIGHT
                .validate(&serde_json::json!({"target":{"kind":"video","id":1}}))
                .is_err()
        );
        assert!(
            SCREEN_INSIGHT
                .validate(&serde_json::json!({"target":{"kind":"screen","id":1},"model":"m","resolution":"1280"}))
                .is_ok()
        );
        for resolution in ["720", "1280", "1920", "2560", "native"] {
            assert!(
                SCREEN_INSIGHT
                    .validate(&serde_json::json!({
                        "target":{"kind":"screen","id":1},"resolution":resolution
                    }))
                    .is_ok()
            );
        }
        assert!(
            SCREEN_INSIGHT
                .validate(&serde_json::json!({
                    "target":{"kind":"screen","id":1},"resolution":null
                }))
                .is_ok()
        );
        for resolution in [
            serde_json::json!("huge"),
            serde_json::json!(""),
            serde_json::json!(1920),
        ] {
            assert!(
                SCREEN_INSIGHT
                    .validate(&serde_json::json!({
                        "target":{"kind":"screen","id":1},"resolution":resolution
                    }))
                    .is_err()
            );
        }
        let png = format!("{IMAGE_PREFIX}AA==");
        assert!(
            SCREEN_INSIGHT
                .validate(&serde_json::json!({"image": png, "question": "x"}))
                .is_ok()
        );
        for bad in [
            serde_json::json!({}),
            serde_json::json!({"image": "data:image/jpeg;base64,AA=="}),
            serde_json::json!({"image": png, "target": {"kind":"screen","id":1}}),
        ] {
            assert!(SCREEN_INSIGHT.validate(&bad).is_err());
        }
        assert_eq!(image_bytes(&png).unwrap(), vec![0]);
        assert!(image_bytes("data:image/png;base64,***").is_err());
        let mut settings = crate::settings::Settings::default();
        assert!(vision_model(&settings, &[]).is_err());
        settings.models.analysis = "vision".into();
        let mut model = ModelCapabilities::unknown("vision");
        assert!(vision_model(&settings, &[model.clone()]).is_err());
        model.image_in = TriState::Yes;
        model.tool_calls = TriState::Yes;
        assert_eq!(vision_model(&settings, &[model]).unwrap(), "vision");
    }
    #[tokio::test]
    async fn runner_handles_cancel_timeout_and_failure_without_hiding_errors() {
        let token = CancelToken::default();
        token.cancel();
        assert!(
            bounded(std::future::pending(), &token, Duration::from_secs(1))
                .await
                .unwrap_err()
                .contains("cancelled")
        );
        assert!(
            bounded(
                std::future::pending(),
                &CancelToken::default(),
                Duration::ZERO
            )
            .await
            .unwrap_err()
            .contains("timed out")
        );
        assert_eq!(
            bounded(
                async { Err("Infer busy".into()) },
                &CancelToken::default(),
                Duration::from_secs(1)
            )
            .await
            .unwrap_err(),
            "Infer busy"
        );
    }
}
