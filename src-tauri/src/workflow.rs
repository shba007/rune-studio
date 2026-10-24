//! Small static workflow registry. No scheduler, graph or process ownership.
use crate::{
    analysis::{
        run::{ModelAnalysis, analyze_image_with_settings},
        state::StoredFrame,
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
    target: CaptureTarget,
    #[serde(default)]
    question: String,
}
pub fn vision_model(
    settings: &crate::settings::Settings,
    models: &[ModelCapabilities],
) -> Result<String, String> {
    let name = settings.models.analysis.trim();
    if name.is_empty() {
        return Err(
            "Configure an explicit Analysis model in Agents settings before running Screen Insight"
                .into(),
        );
    }
    let model = models
        .iter()
        .find(|m| m.id == name)
        .ok_or("Configured Analysis model is not available from Infer")?;
    if model.image_in != TriState::Yes || model.tool_calls != TriState::Yes {
        return Err(
            "Screen Insight requires a model explicitly advertising image input and tool calling"
                .into(),
        );
    }
    Ok(name.into())
}
impl Workflow for ScreenInsight {
    fn definition(&self) -> Definition {
        Definition {
            id: "screen-insight",
            name: "Screen Insight",
            capabilities: &["screen_capture", "vision_model"],
        }
    }
    fn validate(&self, input: &Value) -> Result<(), String> {
        let input: ScreenInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Choose a screen/window and a question: {e}"))?;
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
            let settings = state.settings.locked().clone();
            let status = crate::backend::check(
                &state.http,
                &settings,
                crate::settings::api_key().as_deref(),
            )
            .await;
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
            let frame = tokio::time::timeout(
                Duration::from_secs(10),
                tauri::async_runtime::spawn_blocking(move || {
                    screenshot::insight_capture(input.target)
                }),
            )
            .await
            .map_err(|_| "Screen/window capture timed out")?
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("Capture permission denied or target unavailable: {e}"))?;
            let image = frame.data_uri();
            let signature = frame.signature();
            let stored = StoredFrame {
                jpeg: std::sync::Arc::new(frame.jpeg),
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
            let analysis =
                analyze_image_with_settings(app, &question, &image, cancel, id, &settings)
                    .await
                    .map_err(|e| e.to_string())?;
            Ok(Output {
                analysis,
                frame: Some(stored),
                captured_utc,
            })
        })
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
                "Screen Insight uses API mode. Finish the session and select API mode first."
                    .into(),
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
    let result = bounded(
        workflow.execute(app, input, &cancel, &id),
        &cancel,
        Duration::from_secs(90),
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
