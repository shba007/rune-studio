use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::settings::Settings;

const PROBE_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendStatus {
    pub reachable: bool,
    pub auth_ok: bool,
    pub missing_models: Vec<String>,
    /// What the configured model says it can do (`Chat`, `Vision`, `Audio`, ...). Empty when the
    /// server does not list it or does not report capabilities.
    pub model_capabilities: Vec<String>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
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
        }
    }
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

/// True only when the server listed the model's capabilities and `Audio` is not among them.
/// An empty list means "unknown", never "no audio".
pub fn lacks_audio(capabilities: &[String]) -> bool {
    !capabilities.is_empty()
        && !capabilities
            .iter()
            .any(|capability| capability.eq_ignore_ascii_case("audio"))
}

/// Probes `/health` (open), then `/v1/models` (authenticated) and checks that the
/// models the agent needs are actually served.
pub async fn check(
    client: &reqwest::Client,
    settings: &Settings,
    api_key: Option<&str>,
) -> BackendStatus {
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
        };
    }

    match response.json::<ModelList>().await {
        Ok(list) => {
            let missing_models = [&settings.model]
                .into_iter()
                .filter(|wanted| !list.data.iter().any(|model| &model.id == *wanted))
                .cloned()
                .collect();
            let model_capabilities = list
                .data
                .iter()
                .find(|model| model.id == settings.model)
                .map(|model| model.capabilities.clone())
                .unwrap_or_default();
            BackendStatus {
                reachable: true,
                auth_ok: true,
                missing_models,
                model_capabilities,
                latency_ms: Some(latency_ms),
                error: None,
            }
        }
        Err(err) => BackendStatus {
            reachable: true,
            auth_ok: true,
            missing_models: Vec::new(),
            model_capabilities: Vec::new(),
            latency_ms: Some(latency_ms),
            error: Some(format!("unexpected model list format: {err}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities_of(json: &str, model: &str) -> Vec<String> {
        let list: ModelList = serde_json::from_str(json).unwrap();
        list.data
            .iter()
            .find(|entry| entry.id == model)
            .map(|entry| entry.capabilities.clone())
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
