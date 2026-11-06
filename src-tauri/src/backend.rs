use std::time::{Duration, Instant};

use serde::Serialize;

use crate::{
    model::capabilities::{ModelCapabilities, parse_models_response},
    settings::Settings,
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    pub reachable: bool,
    pub auth_ok: bool,
    pub missing_models: Vec<String>,
    pub model_capabilities: Vec<String>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelCapabilities>,
    pub memory: crate::model::capabilities::ServerMemory,
}

impl BackendStatus {
    fn unreachable(error: String) -> Self {
        Self {
            reachable: false,
            auth_ok: false,
            missing_models: Vec::new(),
            model_capabilities: Vec::new(),
            latency_ms: None,
            error: Some(error),
            models: Vec::new(),
            memory: Default::default(),
        }
    }
}

pub fn lacks_audio(capabilities: &[String]) -> bool {
    !capabilities.is_empty()
        && !capabilities
            .iter()
            .any(|capability| capability.eq_ignore_ascii_case("audio"))
}

pub async fn check(
    client: &reqwest::Client,
    settings: &Settings,
    api_key: Option<&str>,
) -> BackendStatus {
    if settings.active_connection_id == "studio-embedded-infer"
        && settings.base_url == "http://127.0.0.1:0"
    {
        return BackendStatus::unreachable(
            "Local Infer is not running. Download or select a model, then start it.".into(),
        );
    }
    let base = settings.base_url.trim_end_matches('/');

    let started = Instant::now();
    let health = client
        .get(format!("{base}/health"))
        .timeout(PROBE_TIMEOUT)
        .send()
        .await;
    let latency_ms = started.elapsed().as_millis() as u64;
    match health {
        Ok(response) if response.status().is_success() => {}
        Ok(response) => {
            return BackendStatus::unreachable(format!(
                "health check returned {}",
                response.status()
            ));
        }
        Err(err) => return BackendStatus::unreachable(format!("cannot reach {base}: {err}")),
    }

    let mut request = client
        .get(format!("{base}/v1/models"))
        .timeout(PROBE_TIMEOUT);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => return BackendStatus::unreachable(format!("model list failed: {err}")),
    };

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return BackendStatus {
            reachable: true,
            auth_ok: false,
            missing_models: Vec::new(),
            model_capabilities: Vec::new(),
            latency_ms: Some(latency_ms),
            error: Some("API key rejected".into()),
            models: Vec::new(),
            memory: Default::default(),
        };
    }
    if !status.is_success() {
        return BackendStatus {
            reachable: true,
            auth_ok: true,
            missing_models: Vec::new(),
            model_capabilities: Vec::new(),
            latency_ms: Some(latency_ms),
            error: Some(format!("model list returned {status}")),
            models: Vec::new(),
            memory: Default::default(),
        };
    }

    let body = response.text().await.unwrap_or_default();
    let models = parse_models_response(&body);

    let missing_models = missing_models(settings, &models);

    let agent_model = settings.model_for_role(crate::model::role::ModelRole::Agent);
    let model_capabilities = models
        .iter()
        .find(|m| m.id == agent_model)
        .map(|m| m.raw_capabilities.clone())
        .unwrap_or_default();

    let models = studio_model_choices(models);

    BackendStatus {
        reachable: true,
        auth_ok: true,
        missing_models,
        model_capabilities,
        latency_ms: Some(latency_ms),
        error: None,
        models,
        memory: crate::model::capabilities::parse_server_memory(&body),
    }
}

fn studio_model_choices(models: Vec<ModelCapabilities>) -> Vec<ModelCapabilities> {
    models
        .into_iter()
        .filter(|model| !model.id.eq_ignore_ascii_case("holo3.1-0.8b-q4"))
        .collect()
}

fn missing_models(settings: &Settings, models: &[ModelCapabilities]) -> Vec<String> {
    let mut configured_roles = vec![
        settings.model_for_role(crate::model::role::ModelRole::Agent),
        settings.model_for_role(crate::model::role::ModelRole::Verifier),
        settings.model_for_role(crate::model::role::ModelRole::Analysis),
        settings.model_for_role(crate::model::role::ModelRole::Voice),
    ];
    if settings.fast_checks && settings.decision_url.trim().is_empty() {
        configured_roles.push(settings.model_for_role(crate::model::role::ModelRole::Decision));
    }
    configured_roles
        .into_iter()
        .filter(|wanted| !wanted.is_empty() && !models.iter().any(|m| m.id == *wanted))
        .map(|s| s.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_model_is_not_offered_in_discovered_choices() {
        let models = vec![
            ModelCapabilities::unknown("HOLO3.1-0.8B-Q4"),
            ModelCapabilities::unknown("holo3.1-4b-q4"),
        ];
        let visible = studio_model_choices(models);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, "holo3.1-4b-q4");
    }

    #[tokio::test]
    async fn stopped_local_infer_has_an_actionable_status() {
        let settings = Settings {
            active_connection_id: "studio-embedded-infer".into(),
            base_url: "http://127.0.0.1:0".into(),
            ..Settings::default()
        };
        let status = check(&reqwest::Client::new(), &settings, None).await;
        assert!(!status.reachable);
        assert!(status.error.unwrap().contains("Download or select a model"));
    }

    #[test]
    fn optional_decision_model_is_only_required_when_fast_checks_use_this_server() {
        let mut settings = Settings::default();
        let models = vec![ModelCapabilities::unknown(&settings.models.agent)];
        assert!(missing_models(&settings, &models).is_empty());
        settings.fast_checks = true;
        assert_eq!(missing_models(&settings, &models), ["liquid-d1-3b"]);
        settings.decision_url = "http://127.0.0.1:9999".into();
        assert!(missing_models(&settings, &models).is_empty());
    }

    fn capabilities_of(json: &str, model: &str) -> Vec<String> {
        let models = parse_models_response(json);
        models
            .into_iter()
            .find(|entry| entry.id == model)
            .map(|entry| entry.raw_capabilities)
            .unwrap_or_default()
    }

    #[test]
    fn capabilities_are_read_when_present_and_empty_when_not() {
        let json = r#"{ "data": [
            { "id": "omni", "capabilities": ["Chat", "Vision", "Audio", "Omni", "Tool Call"] },
            { "id": "plain" }
        ] }"#;
        assert_eq!(
            capabilities_of(json, "omni"),
            ["Chat", "Vision", "Audio", "Omni", "Tool Call"]
        );
        assert!(
            capabilities_of(json, "plain").is_empty(),
            "the field is optional"
        );
        assert!(capabilities_of(json, "unlisted").is_empty());
    }

    #[test]
    fn only_a_listed_model_without_audio_lacks_audio() {
        let list = |items: &[&str]| {
            items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        };
        assert!(
            !lacks_audio(&[]),
            "no information is not a missing capability"
        );
        assert!(!lacks_audio(&list(&["Chat", "Audio"])));
        assert!(!lacks_audio(&list(&["chat", "audio"])));
        assert!(lacks_audio(&list(&["Chat", "Vision"])));
    }
}
