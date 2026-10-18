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
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

impl BackendStatus {
    fn unreachable(error: String) -> Self {
        Self {
            reachable: false,
            auth_ok: false,
            missing_models: Vec::new(),
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
}

/// Probes `/health` (open), then `/v1/models` (authenticated) and checks that the
/// models the agent needs are actually served.
pub async fn check(client: &reqwest::Client, settings: &Settings, api_key: Option<&str>) -> BackendStatus {
    let base = settings.base_url.trim_end_matches('/');

    let started = Instant::now();
    let health = client.get(format!("{base}/health")).timeout(PROBE_TIMEOUT).send().await;
    let latency_ms = started.elapsed().as_millis() as u64;
    match health {
        Ok(response) if response.status().is_success() => {}
        Ok(response) => {
            return BackendStatus::unreachable(format!("health check returned {}", response.status()));
        }
        Err(err) => return BackendStatus::unreachable(format!("cannot reach {base}: {err}")),
    }

    let mut request = client.get(format!("{base}/v1/models")).timeout(PROBE_TIMEOUT);
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
            latency_ms: Some(latency_ms),
            error: Some("API key rejected".into()),
        };
    }
    if !status.is_success() {
        return BackendStatus {
            reachable: true,
            auth_ok: true,
            missing_models: Vec::new(),
            latency_ms: Some(latency_ms),
            error: Some(format!("model list returned {status}")),
        };
    }

    match response.json::<ModelList>().await {
        Ok(list) => {
            let missing_models = [&settings.vision_model, &settings.asr_model]
                .into_iter()
                .filter(|wanted| !list.data.iter().any(|model| &model.id == *wanted))
                .cloned()
                .collect();
            BackendStatus {
                reachable: true,
                auth_ok: true,
                missing_models,
                latency_ms: Some(latency_ms),
                error: None,
            }
        }
        Err(err) => BackendStatus {
            reachable: true,
            auth_ok: true,
            missing_models: Vec::new(),
            latency_ms: Some(latency_ms),
            error: Some(format!("unexpected model list format: {err}")),
        },
    }
}
