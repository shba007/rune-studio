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
    observe::screenshot::{self, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
    settings,
    sync::LockExt,
    trace::{self, AnalysisRecord},
};

const CALL_TIMEOUT: Duration = Duration::from_secs(60);

enum Ended {
    Done {
        answer: String,
        findings: Vec<Finding>,
    },
    Failed(String),
    Cancelled,
}

fn publish(app: &AppHandle) {
    let (snapshot, wanted) = {
        let state = app.state::<AppState>();
        let analysis = state.analysis.locked();
        let boxes_on_screen = analysis
            .findings
            .iter()
            .any(|finding| finding.bbox.is_some());
        (
            analysis.snapshot(),
            wants_overlay(analysis.status, boxes_on_screen, analysis.stale),
        )
    };
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
        let captured = unless_stopped(
            cancel,
            tauri::async_runtime::spawn_blocking(|| {
                screenshot::capture(None, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY)
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
        match archive::save(archive::Kind::Screenshots, "screen", "jpg", &frame.jpeg) {
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

    let (settings, api_key) = (state.settings.locked().clone(), settings::api_key());
    let analysis_id = state.analysis.locked().id.clone();
    let call_log = state.trace.call_log(&analysis_id, "analysis");
    let client = ChatClient {
        http: &state.http,
        base_url: &settings.base_url,
        api_key: api_key.as_deref(),
        model: &settings.model,
        profile: None,
        sink: Some(&call_log),
    };
    let image = format!("data:image/jpeg;base64,{}", base64_of(&frame.jpeg));
    let tools = report::tools();
    let request = report::request(question, &image, &tools);
    let call = async {
        tokio::time::timeout(
            CALL_TIMEOUT,
            client.call_retrying(&request, &mut |_: &str| {}),
        )
        .await
    };
    let ended = match unless_stopped(cancel, call).await {
        None => Ended::Cancelled,
        Some(Err(_)) => Ended::Failed(format!(
            "the model did not answer within {} seconds",
            CALL_TIMEOUT.as_secs()
        )),
        Some(Ok(Err(err))) => Ended::Failed(err.to_string()),
        Some(Ok(Ok(reply))) if reply.call.name != "report" => Ended::Failed(format!(
            "the model answered with \"{}\" instead of a report",
            reply.call.name
        )),
        Some(Ok(Ok(reply))) => {
            let parsed = report::parse(&reply.call.arguments);
            Ended::Done {
                answer: parsed.answer,
                findings: parsed.findings,
            }
        }
    };
    (ended, path)
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
    screenshot_path: Option<String>,
    latency_ms: u64,
) {
    let state = app.state::<AppState>();
    let model = state.settings.locked().model.clone();
    let (status, label, error) = {
        let mut analysis = state.analysis.locked();
        analysis.model = model.clone();
        analysis.latency_ms = latency_ms;
        match ended {
            Ended::Done {
                answer,
                mut findings,
            } => {
                analysis.boxes_rejected = super::boxes::guard(&mut findings);
                analysis.answer = answer;
                analysis.findings = findings;
                analysis.status = Status::Done;
                ("ok", "succeeded", None)
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
    watch_for_change(app, id.to_string());
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
                .any(|finding| finding.bbox.is_some())
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
