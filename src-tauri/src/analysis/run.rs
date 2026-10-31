use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};

use super::{
    report::{self, Finding, Status},
    stale::{self, Staleness},
    state::{StoredFrame, may_follow, refusal, wants_overlay},
};
use crate::{
    agent::unless_stopped,
    archive,
    error::{AppError, AppResult},
    events::{self, AppState},
    exec::CancelToken,
    model::client::ChatClient,
    observe::screenshot::{self, MODEL_JPEG_QUALITY},
    settings,
    sync::LockExt,
    trace::{self, AnalysisRecord},
};

const CALL_TIMEOUT: Duration = Duration::from_secs(60);

fn overlay_wanted(snapshot: &super::state::Snapshot) -> bool {
    if snapshot
        .monitor
        .is_some_and(|monitor| monitor.id == crate::observe::screenshot::IMAGE_MONITOR)
    {
        return false;
    }
    let visible_geometry = snapshot
        .findings
        .iter()
        .any(|finding| finding.has_geometry() && snapshot.visible_groups.shows(finding.group));
    wants_overlay(snapshot.status, visible_geometry, snapshot.stale)
}

enum Ended {
    Done {
        answer: String,
        findings: Vec<Finding>,
        model: String,
    },
    Failed(String),
    Cancelled,
}

pub(crate) fn publish(app: &AppHandle) {
    let mut snapshot = {
        let state = app.state::<AppState>();
        let analysis = state.analysis.locked();
        analysis.snapshot()
    };
    snapshot.visible_groups = app
        .state::<AppState>()
        .settings
        .locked()
        .visual_analysis
        .clone();
    let wanted = overlay_wanted(&snapshot);
    crate::overlay::sync_analysis(app, wanted);
    let _ = app.emit(events::ANALYSIS_UPDATED, snapshot);
}

fn set_status(app: &AppHandle, status: Status) {
    {
        let state = app.state::<AppState>();
        let mut analysis = state.analysis.locked();
        if !may_follow(analysis.status, status) {
            log::warn!(
                "analysis status {:?} -> {status:?} is not a step the flow takes",
                analysis.status
            );
        }
        analysis.status = status;
    }
    publish(app);
}

pub async fn run(app: &AppHandle, question: &str, fresh: bool) -> AppResult<()> {
    crate::reflect::touch(app, crate::reflect::idle::Input::Analysis);
    let state = app.state::<AppState>();
    let question = report::question_or_default(question);
    let cancel = {
        let mut analysis = state.analysis.locked();
        if state.settings.locked().external_agent {
            return Err(AppError::Invalid("External agent mode is selected. Ask your agent to call rune_analyze; select API mode before starting standalone analysis.".into()));
        }
        if let Some(why) = refusal(
            analysis.status,
            events::agent_state(app).is_running(),
            state.lease.locked().is_held(),
        ) {
            return Err(AppError::Invalid(why.into()));
        }
        if !fresh && analysis.frame.is_none() {
            return Err(AppError::Invalid(
                "There is no screen to ask about yet. Capture it first.".into(),
            ));
        }
        analysis.begin(&question, fresh);
        analysis.cancel.clone()
    };
    publish(app);

    let started = Instant::now();
    let session = archive::begin_session(&question, "analysis");
    let session_id = session
        .as_deref()
        .ok()
        .map(str::to_string)
        .unwrap_or_default();
    {
        let mut analysis = state.analysis.locked();
        analysis.id = session_id.clone();
        analysis.created_utc = trace::now_utc();
    }
    let (ended, screenshot_path) = match session {
        Ok(_) => work(app, &question, fresh, &cancel).await,
        Err(err) => (
            Ended::Failed(format!("could not open the archive: {err}")),
            None,
        ),
    };
    let latency_ms = started.elapsed().as_millis() as u64;
    finish(
        app,
        &question,
        &session_id,
        ended,
        screenshot_path,
        latency_ms,
    );
    Ok(())
}

async fn work(
    app: &AppHandle,
    question: &str,
    fresh: bool,
    cancel: &CancelToken,
) -> (Ended, Option<String>) {
    let state = app.state::<AppState>();
    let mut path = None;
    if fresh {
        let max_edge = state.settings.locked().model_image_resolution.max_edge();
        let capture_edge = state.settings.locked().recording_resolution.max_edge();
        let captured = unless_stopped(
            cancel,
            tauri::async_runtime::spawn_blocking(move || {
                screenshot::capture(None, max_edge, MODEL_JPEG_QUALITY, capture_edge)
            }),
        )
        .await;
        let frame = match captured {
            None => return (Ended::Cancelled, None),
            Some(Ok(Ok(frame))) => frame,
            Some(Ok(Err(err))) => {
                return (
                    Ended::Failed(format!("could not see the screen: {err}")),
                    None,
                );
            }
            Some(Err(err)) => return (Ended::Failed(err.to_string()), None),
        };
        path = Some(frame.path.display().to_string()).filter(|path| !path.is_empty());
        let signature = frame.signature();
        let front = crate::safety::foreground::remember();
        state.analysis.locked().frame = Some(StoredFrame {
            jpeg: Arc::new(frame.jpeg),
            png: Arc::new(frame.png),
            monitor: frame.monitor,
            signature,
            front,
        });
    }
    let Some(frame) = state.analysis.locked().frame.clone() else {
        return (
            Ended::Failed("there is no screen to ask about".into()),
            None,
        );
    };
    if !fresh {
        match archive::save(archive::Kind::Screenshots, "screen", "png", &frame.png) {
            Ok(saved) => path = Some(saved.display().to_string()).filter(|path| !path.is_empty()),
            Err(err) => {
                return (
                    Ended::Failed(format!("could not store the screenshot: {err}")),
                    None,
                );
            }
        }
    }
    set_status(app, Status::Asking);

    let analysis_id = state.analysis.locked().id.clone();
    let image = format!("data:image/jpeg;base64,{}", base64_of(&frame.jpeg));
    let ended = match analyze_image(app, question, &image, cancel, &analysis_id).await {
        Ok(report) => Ended::Done {
            answer: report.parsed.answer,
            findings: report.parsed.findings,
            model: report.model,
        },
        Err(AnalysisError::Cancelled) => Ended::Cancelled,
        Err(AnalysisError::Failed(why)) => Ended::Failed(why),
    };
    (ended, path)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AnalysisError {
    #[error("analysis cancelled")]
    Cancelled,
    #[error("{0}")]
    Failed(String),
}

#[derive(Debug)]
pub struct ModelAnalysis {
    pub parsed: report::ParsedReport,
    pub model: String,
}

/// Shared inference only: never begins/ends an archive session or changes UI state.
pub async fn analyze_image(
    app: &AppHandle,
    question: &str,
    image: &str,
    cancel: &CancelToken,
    run_id: &str,
) -> Result<ModelAnalysis, AnalysisError> {
    let state = app.state::<AppState>();
    let settings = state.settings.locked().clone();
    analyze_image_with_settings(app, question, image, cancel, run_id, &settings).await
}

pub(crate) async fn analyze_image_with_settings(
    app: &AppHandle,
    question: &str,
    image: &str,
    cancel: &CancelToken,
    run_id: &str,
    settings: &settings::Settings,
) -> Result<ModelAnalysis, AnalysisError> {
    let state = app.state::<AppState>();
    let api_key = settings::api_key_for(settings);
    let call_log = state.trace.call_log(run_id, "analysis");
    let analysis_model = settings.model_for_role(crate::model::role::ModelRole::Analysis);
    let profile = crate::workspace_tasks::execution_profile(
        app,
        &settings.base_url,
        api_key.as_deref(),
        analysis_model,
    )
    .await
    .map_err(AnalysisError::Failed)?;
    let client = ChatClient {
        app: Some(app),
        output_limit: settings.output_limit(),
        http: &state.http,
        base_url: &settings.base_url,
        api_key: api_key.as_deref(),
        model: analysis_model,
        profile: Some(&profile),
        sink: Some(&call_log),
        no_swap: false,
        on_loading: None,
    };
    analyze_with_class(
        &client,
        question,
        image,
        cancel,
        CALL_TIMEOUT,
        crate::workspace_tasks::protected_execution_context(app)
            .then_some(crate::life::DataClass::Finance),
    )
    .await
}

#[cfg(test)]
async fn analyze_with_client(
    client: &ChatClient<'_>,
    question: &str,
    image: &str,
    cancel: &CancelToken,
    timeout: Duration,
) -> Result<ModelAnalysis, AnalysisError> {
    analyze_with_class(client, question, image, cancel, timeout, None).await
}
async fn analyze_with_class(
    client: &ChatClient<'_>,
    question: &str,
    image: &str,
    cancel: &CancelToken,
    timeout: Duration,
    data_class: Option<crate::life::DataClass>,
) -> Result<ModelAnalysis, AnalysisError> {
    if cancel.is_cancelled() {
        return Err(AnalysisError::Cancelled);
    }
    let schema = report::schema();
    let mut request = report::request(question, image);
    request.data_class = data_class;
    let call = async {
        tokio::time::timeout(timeout, client.call_json_reply_retrying(&request, &schema)).await
    };
    match unless_stopped(cancel, call).await {
        None => Err(AnalysisError::Cancelled),
        Some(Err(_)) => Err(AnalysisError::Failed(format!(
            "the model did not answer within {} seconds",
            timeout.as_secs()
        ))),
        Some(Ok(Err(err))) => Err(AnalysisError::Failed(err.to_string())),
        Some(Ok(Ok(reply))) => {
            if cancel.is_cancelled() {
                return Err(AnalysisError::Cancelled);
            }
            if !reply.call.arguments["answer"].is_string()
                || !reply.call.arguments["findings"].is_array()
            {
                return Err(AnalysisError::Failed(
                    "the model returned a malformed visual-analysis report".into(),
                ));
            }
            let parsed = report::parse(&reply.call.arguments);
            Ok(ModelAnalysis {
                parsed,
                model: reply
                    .served_model
                    .unwrap_or_else(|| client.model.to_string()),
            })
        }
    }
}

fn base64_of(bytes: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};
    STANDARD.encode(bytes)
}

fn finish(
    app: &AppHandle,
    question: &str,
    id: &str,
    ended: Ended,
    mut screenshot_path: Option<String>,
    latency_ms: u64,
) {
    let state = app.state::<AppState>();
    if state.analysis.locked().id != id {
        return;
    }
    let model = match &ended {
        Ended::Done { model, .. } => model.clone(),
        _ if id.starts_with("insight-") => state.analysis.locked().model.clone(),
        _ => state
            .settings
            .locked()
            .model_for_role(crate::model::role::ModelRole::Analysis)
            .to_owned(),
    };
    let (status, label, error) = {
        let mut analysis = state.analysis.locked();
        if analysis.id != id {
            return;
        }
        let ended = if analysis.cancel.is_cancelled() {
            Ended::Cancelled
        } else {
            ended
        };
        analysis.model = model.clone();
        analysis.latency_ms = latency_ms;
        match ended {
            Ended::Done {
                answer,
                mut findings,
                ..
            } => {
                let capture_error = if id.starts_with("insight-") {
                    analysis.frame.as_ref().and_then(|frame| {
                        match archive::save_completed_insight(id, question, &answer, &frame.png) {
                            Ok(path) => { screenshot_path = Some(path.display().to_string()); None }
                            Err(error) => {
                                let message = format!("Analysis succeeded, but its capture could not be saved: {error}");
                                log::warn!("{message}");
                                Some(message)
                            }
                        }
                    })
                } else {
                    None
                };
                analysis.boxes_rejected = super::boxes::guard(&mut findings);
                analysis.answer = answer;
                analysis.findings = findings;
                analysis.status = Status::Done;
                ("ok", "succeeded", capture_error)
            }
            Ended::Failed(why) => {
                analysis.status = Status::Error;
                analysis.error = Some(why.clone());
                ("error", "failed", Some(why))
            }
            Ended::Cancelled => {
                analysis.status = Status::Cancelled;
                ("cancelled", "stopped", None)
            }
        }
    };
    let record = {
        let analysis = state.analysis.locked();
        AnalysisRecord {
            id: id.to_string(),
            created_utc: analysis.created_utc.clone(),
            question: question.to_string(),
            answer: analysis.answer.clone(),
            findings_json: serde_json::to_string(&analysis.findings).unwrap_or_default(),
            model: Some(model),
            latency_ms: i64::try_from(latency_ms).ok(),
            status: status.into(),
            error,
            screenshot_path,
            monitor_json: analysis
                .frame
                .as_ref()
                .and_then(|frame| serde_json::to_string(&frame.monitor).ok()),
            boxes_rejected: i64::from(analysis.boxes_rejected),
        }
    };
    if !id.is_empty() {
        state.trace.analysis(record);
        archive::end_session(Some(id), label, "");
    }
    publish(app);
    if !id.starts_with("insight-") {
        watch_for_change(app, id.to_string());
    }
}

pub fn cancel(app: &AppHandle) {
    app.state::<AppState>().analysis.locked().cancel.cancel();
}

pub fn select(app: &AppHandle, n: Option<u8>) {
    {
        let state = app.state::<AppState>();
        let mut analysis = state.analysis.locked();
        analysis.selected = n.filter(|n| analysis.findings.iter().any(|finding| finding.n == *n));
    }
    publish(app);
}

pub fn close(app: &AppHandle) {
    app.state::<AppState>().analysis.locked().close();
    publish(app);
}

fn watch_for_change(app: &AppHandle, id: String) {
    let has_boxes = {
        let state = app.state::<AppState>();
        let analysis = state.analysis.locked();
        analysis.status == Status::Done
            && analysis
                .findings
                .iter()
                .any(|finding| finding.has_geometry())
    };
    if !has_boxes || id.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(stale::EVERY_MS)).await;
            let (thumbnail, then) = {
                let state = app.state::<AppState>();
                let analysis = state.analysis.locked();
                if analysis.id != id || analysis.status != Status::Done || analysis.stale {
                    return;
                }
                match &analysis.frame {
                    Some(frame) => (frame.signature.clone(), frame.front),
                    None => return,
                }
            };
            let Ok(Ok(live)) =
                tauri::async_runtime::spawn_blocking(screenshot::live_signature).await
            else {
                continue;
            };
            let now = crate::safety::foreground::remember();
            let verdict = stale::decide(&thumbnail, &live, then, now);
            if verdict != Staleness::Fresh {
                log::info!("analysis {id}: the screen changed ({verdict:?}), the boxes are gone");
                {
                    let state = app.state::<AppState>();
                    let mut analysis = state.analysis.locked();
                    if analysis.id == id {
                        analysis.stale = true;
                    }
                }
                publish(&app);
                return;
            }
        }
    });
}

pub(crate) fn current(app: &AppHandle, id: &str) -> bool {
    let state = app.state::<AppState>();
    let analysis = state.analysis.locked();
    analysis.accepts(id)
}
pub(crate) fn finish_workflow(
    app: &AppHandle,
    id: &str,
    result: Result<crate::workflow::Output, String>,
    latency_ms: u64,
) {
    let state = app.state::<AppState>();
    let (question, cancelled) = {
        let analysis = state.analysis.locked();
        if analysis.id != id {
            return;
        }
        (analysis.question.clone(), analysis.cancel.is_cancelled())
    };
    let ended = if cancelled {
        Ended::Cancelled
    } else {
        match result {
            Ok(output) => {
                let mut analysis = state.analysis.locked();
                if analysis.id != id {
                    return;
                }
                analysis.frame = output.frame;
                analysis.created_utc = output.captured_utc;
                Ended::Done {
                    answer: output.analysis.parsed.answer,
                    findings: output.analysis.parsed.findings,
                    model: output.analysis.model,
                }
            }
            Err(error) if error == "cancelled" => Ended::Cancelled,
            Err(error) => Ended::Failed(error),
        }
    };
    finish(app, &question, id, ended, None, latency_ms);
}

#[cfg(test)]
mod model_tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn progress_overlay_is_visible_before_findings_exist_even_with_groups_hidden() {
        let mut snapshot = super::super::state::AnalysisState::default().snapshot();
        snapshot.visible_groups = crate::settings::VisualAnalysis {
            entities: false,
            regions: false,
            structure: false,
            depth: false,
        };
        for status in [Status::Capturing, Status::Asking] {
            snapshot.status = status;
            assert!(overlay_wanted(&snapshot));
        }
        for status in [Status::Done, Status::Error, Status::Cancelled, Status::Idle] {
            snapshot.status = status;
            assert!(!overlay_wanted(&snapshot));
        }
    }
    use std::io::{BufRead, Read, Write};

    fn server(
        tool: &str,
        arguments: Value,
        delay: Duration,
    ) -> (String, std::sync::mpsc::Receiver<Value>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let message = if tool == "report" {
            json!({"content": arguments.to_string()})
        } else {
            json!({"tool_calls":[{"id":"report-1","type":"function","function":{"name":tool,"arguments":arguments.to_string()}}]})
        };
        let body = json!({"model":"vision-served","choices":[{"message":message}]}).to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            let _ = tx.send(serde_json::from_slice(&bytes).unwrap());
            std::thread::sleep(delay);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Model-Served: vision-served\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes());
        });
        (url, rx)
    }

    async fn ask(
        url: &str,
        cancel: &CancelToken,
        timeout: Duration,
    ) -> Result<ModelAnalysis, AnalysisError> {
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let client = ChatClient {
            app: None,
            output_limit: None,
            http: &http,
            base_url: url,
            api_key: None,
            model: "chosen-analysis-role",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        analyze_with_client(
            &client,
            "Where is Save?",
            "data:image/jpeg;base64,AA",
            cancel,
            timeout,
        )
        .await
    }

    #[tokio::test]
    async fn shared_model_analysis_sends_question_image_and_role_then_bounds_findings() {
        let (url, request) = server(
            "report",
            json!({"answer":"a".repeat(5000),"findings":(0..20).map(|_|json!({"label":"Save","kind":"control","description":"A button","evidence":"Save label","box":[10,10,100,60]})).collect::<Vec<_>>()}),
            Duration::ZERO,
        );
        let report = ask(&url, &CancelToken::default(), Duration::from_secs(3))
            .await
            .unwrap();
        assert_eq!(report.model, "vision-served");
        assert_eq!(report.parsed.answer.chars().count(), report::ANSWER_CAP);
        assert_eq!(report.parsed.findings.len(), report::MAX_FINDINGS);
        let sent = request.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(sent["model"], "chosen-analysis-role");
        assert!(sent["messages"].to_string().contains("Where is Save?"));
        assert!(
            sent["messages"]
                .to_string()
                .contains("data:image/jpeg;base64,AA")
        );
        assert!(
            sent.get("tools").is_none(),
            "Visual Analysis exposes no action or reporting tools"
        );
        assert!(sent.get("tool_choice").is_none());
        assert_eq!(sent["response_format"]["type"], "json_schema");
        assert_eq!(
            sent["response_format"]["json_schema"]["schema"],
            report::schema()
        );
    }

    #[tokio::test]
    async fn wrong_tool_and_malformed_reports_are_errors_not_empty_success() {
        for (tool, arguments) in [
            ("click", json!({"answer":"wrong tool","findings":[]})),
            ("report", json!({"findings":[]})),
            ("report", json!({"answer":"bad","findings":"not an array"})),
        ] {
            let (url, _) = server(tool, arguments, Duration::ZERO);
            assert!(matches!(
                ask(&url, &CancelToken::default(), Duration::from_secs(3)).await,
                Err(AnalysisError::Failed(_))
            ));
        }
    }

    #[tokio::test]
    async fn timeout_and_stop_discard_late_model_replies() {
        let response = json!({"answer":"late","findings":[]});
        let (url, _) = server("report", response.clone(), Duration::from_millis(300));
        assert!(matches!(
            ask(&url, &CancelToken::default(), Duration::from_millis(50)).await,
            Err(AnalysisError::Failed(_))
        ));
        let (url, _) = server("report", response, Duration::from_millis(300));
        let cancel = CancelToken::default();
        let remote = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            remote.cancel();
        });
        let start = Instant::now();
        assert!(matches!(
            ask(&url, &cancel, Duration::from_secs(2)).await,
            Err(AnalysisError::Cancelled)
        ));
        assert!(start.elapsed() < Duration::from_millis(250));
        assert!(matches!(
            ask("invalid-url", &cancel, Duration::from_secs(1)).await,
            Err(AnalysisError::Cancelled)
        ));
    }
    #[tokio::test]
    #[ignore = "requires explicit real Infer model and live desktop capture opt-in"]
    async fn actual_screen_capture_reaches_real_vision_model() {
        let model = std::env::var("RUNE_STUDIO_INSIGHT_TEST_MODEL").expect("explicit test model");
        assert_eq!(
            std::env::var("RUNE_STUDIO_INSIGHT_TEST_SCREEN").as_deref(),
            Ok("primary")
        );
        let mut settings = crate::settings::Settings {
            base_url: std::env::var("RUNE_STUDIO_INSIGHT_TEST_INFER")
                .unwrap_or("http://127.0.0.1:8080".into()),
            ..Default::default()
        };
        settings.models.analysis = model.clone();
        let http = reqwest::Client::new();
        let key = crate::settings::api_key_for(&settings);
        let status = crate::backend::check(&http, &settings, key.as_deref()).await;
        crate::workflow::vision_model(&settings, &status.models)
            .expect("available explicit vision/tool model");
        let target = screenshot::monitors()
            .unwrap()
            .into_iter()
            .find(|m| m.primary)
            .expect("primary screen");
        let frame = screenshot::insight_capture(
            screenshot::CaptureTarget::Screen { id: target.id },
            screenshot::MODEL_MAX_EDGE,
            u32::MAX,
        )
        .unwrap();
        assert!(
            frame.path.as_os_str().is_empty(),
            "Insight never retains the capture"
        );
        let profile = crate::model::profile::resolve_profile(&model, None, None);
        let client = ChatClient {
            app: None,
            output_limit: None,
            http: &http,
            base_url: &settings.base_url,
            api_key: key.as_deref(),
            model: &model,
            profile: Some(&profile),
            sink: None,
            no_swap: false,
            on_loading: None,
        };
        let image = frame.data_uri();
        let captured = crate::trace::now_utc();
        let result = analyze_with_client(
            &client,
            "Identify the most prominent application in one sentence. Do not read secrets.",
            &image,
            &CancelToken::default(),
            CALL_TIMEOUT,
        )
        .await
        .unwrap();
        assert!(!result.parsed.answer.is_empty());
        println!(
            "Captured screen {} at {captured}, {}x{}, model {}: {}",
            target.id, frame.width, frame.height, result.model, result.parsed.answer
        );
    }
}
