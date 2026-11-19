use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{events::AppState, settings};

#[derive(Serialize, Deserialize)]
pub struct CatalogDocument {
    revision: String,
    catalog: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioCatalogDocument {
    revision: String,
    catalog: String,
    connection_id: String,
    connection_revision: u64,
}

async fn fetch_catalog(
    base_url: &str,
    key: Option<&str>,
    document: Option<CatalogDocument>,
) -> Result<CatalogDocument, String> {
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Infer catalog management requires an API key")?;
    validate_url(base_url)?;
    let url = format!("{}/v1/management/catalog", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let request = if let Some(document) = document {
        client.put(url).json(&document)
    } else {
        client.get(url)
    };
    let response = request
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer catalog management")?;
    if !response.status().is_success() {
        let message = match response.status().as_u16() {
            401 => "Infer rejected the API key",
            403 => "Infer catalog management requires an API key",
            409 => "The Infer catalog changed or could not be saved; reload before retrying",
            422 => {
                "Infer rejected the catalog; check model fields and server environment references"
            }
            501 => "This Infer server does not support catalog management",
            _ => "Infer catalog management failed",
        };
        return Err(format!("{message} ({})", response.status()));
    }
    response
        .json()
        .await
        .map_err(|_| "Invalid response from Infer catalog management".into())
}

fn validate_url(base_url: &str) -> Result<(), String> {
    let base = reqwest::Url::parse(base_url).map_err(|_| "Invalid Infer URL")?;
    let loopback = match base.host_str() {
        Some("localhost") => true,
        Some(host) => host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback()),
        None => false,
    };
    if base.scheme() != "https" && !(base.scheme() == "http" && loopback) {
        return Err("Use HTTPS for remote Infer catalog management".into());
    }
    Ok(())
}

pub(crate) async fn verify_infer(base_url: &str, key: Option<&str>) -> Result<(), String> {
    let document = fetch_catalog(base_url, key, None).await?;
    let catalog: serde_json::Value =
        serde_json::from_str(&document.catalog).map_err(|_| "Invalid Infer catalog")?;
    if catalog
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .is_none()
        || !catalog
            .get("models")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err("Unsupported Infer catalog".into());
    }
    Ok(())
}

pub(crate) async fn read_roles(
    base_url: &str,
    key: Option<&str>,
) -> Result<serde_json::Value, String> {
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Infer role management requires an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let response = client
        .get(format!(
            "{}/v1/management/roles",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer role management")?;
    if !response.status().is_success() {
        if matches!(response.status().as_u16(), 404 | 501) {
            return Err(
                "This Infer server does not expose effective roles. Update Infer and retry.".into(),
            );
        }
        return Err(format!(
            "Cannot read effective Infer roles ({})",
            response.status()
        ));
    }
    let roles: serde_json::Value = response.json().await.map_err(|_| "Invalid Infer roles")?;
    if !roles.get("roles").is_some_and(serde_json::Value::is_object) {
        return Err("Invalid Infer roles".into());
    }
    Ok(roles)
}

#[tauri::command]
pub async fn infer_engines(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    read_engines(&settings.base_url, key.as_deref()).await
}

pub(crate) async fn read_model_candidates(
    base_url: &str,
    key: Option<&str>,
    purpose: &str,
) -> Result<serde_json::Value, String> {
    if !matches!(purpose, "all" | "computer-use") {
        return Err("Invalid model candidate purpose".into());
    }
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Infer model candidates require an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let mut response = client
        .get(format!(
            "{}/v1/management/model-candidates?purpose={purpose}",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer model candidates")?;
    if matches!(response.status().as_u16(), 404 | 501) {
        return Err(
            "This Infer server does not expose model candidates. Update Infer and retry.".into(),
        );
    }
    if !response.status().is_success() {
        return Err(format!(
            "Cannot read Infer model candidates ({})",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read Infer model candidates")?
    {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err("Infer model candidates are too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let models: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Infer model candidates")?;
    if !models.is_array() {
        return Err("Invalid Infer model candidates".into());
    }
    Ok(models)
}

pub(crate) async fn model_download_request(
    base_url: &str,
    key: Option<&str>,
    method: reqwest::Method,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let key = key
        .filter(|value| !value.trim().is_empty())
        .ok_or("Infer model downloads require an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let request = client
        .request(
            method,
            format!(
                "{}/v1/management/model-download",
                base_url.trim_end_matches('/')
            ),
        )
        .bearer_auth(key);
    let request = if let Some(body) = body {
        request.json(&body)
    } else {
        request
    };
    let mut response = request
        .send()
        .await
        .map_err(|_| "Cannot reach Infer model downloads")?;
    if matches!(response.status().as_u16(), 404 | 501) {
        return Err(
            "This Infer server does not expose model downloads. Update Infer and retry.".into(),
        );
    }
    if response.status() == reqwest::StatusCode::CONFLICT {
        return Err("The model changed, is unavailable, or another model or engine operation is running. Reload the model list and retry.".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "Infer could not manage the model download ({})",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read Infer model download status")?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err("Infer model download status is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let status: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Infer model download status")?;
    if !status["phase"].is_string() {
        return Err("Invalid Infer model download status".into());
    }
    Ok(status)
}

pub(crate) async fn scan_models_request(
    base_url: &str,
    key: Option<&str>,
    directory: &str,
) -> Result<serde_json::Value, String> {
    let key = key
        .filter(|value| !value.trim().is_empty())
        .ok_or("Infer model scans require an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let mut response = client
        .post(format!(
            "{}/v1/management/model-scan",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .json(&serde_json::json!({"directory":directory}))
        .send()
        .await
        .map_err(|_| "Cannot reach Infer model scan")?;
    if matches!(response.status().as_u16(), 404 | 501) {
        return Err(
            "This Infer server does not expose model scanning. Update Infer and retry.".into(),
        );
    }
    if response.status() == reqwest::StatusCode::BAD_REQUEST {
        return Err("Choose an existing folder on the Infer server.".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "Infer could not scan the model folder ({})",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read Infer model scan")?
    {
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err("Infer model scan is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let found: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Infer model scan")?;
    if !found.is_array() {
        return Err("Invalid Infer model scan".into());
    }
    Ok(found)
}

#[tauri::command]
pub async fn infer_engine_release(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    read_engine_release(&settings.base_url, key.as_deref(), &name).await
}

#[tauri::command]
pub async fn infer_engine_installation(
    state: tauri::State<'_, AppState>,
    name: String,
    start: bool,
) -> Result<serde_json::Value, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    engine_installation_request(&settings.base_url, key.as_deref(), &name, start).await
}

#[tauri::command]
pub async fn infer_engine_update(
    state: tauri::State<'_, AppState>,
    name: String,
    start: bool,
) -> Result<serde_json::Value, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    engine_operation_request(&settings.base_url, key.as_deref(), &name, "update", start).await
}

#[tauri::command]
pub async fn infer_engine_cancel(
    state: tauri::State<'_, AppState>,
    name: String,
    operation: String,
    connection_id: String,
    connection_revision: u64,
) -> Result<serde_json::Value, String> {
    let operation = match operation.as_str() {
        "install" => "installation",
        "update" => "update",
        _ => return Err("Invalid engine operation".into()),
    };
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    if settings.active_connection_id != connection_id
        || settings.connections_revision != connection_revision
    {
        return Err("The active Infer connection changed. Reload engines.".into());
    }
    engine_operation_with_method(
        &settings.base_url,
        key.as_deref(),
        &name,
        operation,
        reqwest::Method::DELETE,
    )
    .await
}

pub(crate) async fn engine_installation_request(
    base_url: &str,
    key: Option<&str>,
    name: &str,
    start: bool,
) -> Result<serde_json::Value, String> {
    engine_operation_request(base_url, key, name, "installation", start).await
}

pub(crate) async fn engine_operation_request(
    base_url: &str,
    key: Option<&str>,
    name: &str,
    operation: &str,
    start: bool,
) -> Result<serde_json::Value, String> {
    engine_operation_with_method(
        base_url,
        key,
        name,
        operation,
        if start {
            reqwest::Method::POST
        } else {
            reqwest::Method::GET
        },
    )
    .await
}

async fn engine_operation_with_method(
    base_url: &str,
    key: Option<&str>,
    name: &str,
    operation: &str,
    method: reqwest::Method,
) -> Result<serde_json::Value, String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("Invalid engine name".into());
    }
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Infer engine operations require an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let url = format!(
        "{}/v1/management/engines/{name}/{operation}",
        base_url.trim_end_matches('/')
    );
    let request = client.request(method.clone(), url);
    let mut response = request
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer engine operation")?;
    if method == reqwest::Method::DELETE && matches!(response.status().as_u16(), 405 | 501) {
        return Err(
            "This Infer server does not support engine cancellation. Update Infer and retry."
                .into(),
        );
    }
    if !response.status().is_success() {
        return Err(format!(
            "Infer could not {} engine operation ({})",
            if method == reqwest::Method::POST {
                "start"
            } else if method == reqwest::Method::DELETE {
                "cancel"
            } else {
                "check"
            },
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read engine operation status")?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err("Infer engine operation response is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let job: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Infer engine operation response")?;
    if job["name"].as_str() != Some(name)
        || job.get("operation").is_some_and(|value| {
            value.as_str()
                != Some(if operation == "installation" {
                    "install"
                } else {
                    "update"
                })
        })
        || !matches!(
            job["status"].as_str(),
            Some("running" | "completed" | "failed" | "cancelled")
        )
        || !job["message"].is_string()
    {
        return Err("Invalid Infer engine operation response".into());
    }
    Ok(job)
}

pub(crate) async fn read_engine_release(
    base_url: &str,
    key: Option<&str>,
    name: &str,
) -> Result<serde_json::Value, String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("Invalid engine name".into());
    }
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Infer release checks require an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let mut response = client
        .post(format!(
            "{}/v1/management/engines/{name}/release-check",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer release check")?;
    if matches!(response.status().as_u16(), 404 | 501) {
        return Err(
            "This Infer server does not support engine release checks. Update Infer and retry."
                .into(),
        );
    }
    if !response.status().is_success() {
        return Err(format!(
            "Infer could not check the engine release ({})",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read Infer release check")?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err("Infer release response is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let release: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Infer release response")?;
    if !release
        .get("latest")
        .is_some_and(serde_json::Value::is_string)
        || !release
            .get("detail")
            .is_some_and(serde_json::Value::is_string)
    {
        return Err("Invalid Infer release response".into());
    }
    Ok(release)
}

pub(crate) async fn read_engines(
    base_url: &str,
    key: Option<&str>,
) -> Result<serde_json::Value, String> {
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Infer engine inventory requires an API key")?;
    validate_url(base_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let mut response = client
        .get(format!(
            "{}/v1/management/engines",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer engine inventory")?;
    if matches!(response.status().as_u16(), 404 | 501) {
        return Err(
            "This Infer server does not expose engine inventory. Update Infer and retry.".into(),
        );
    }
    if !response.status().is_success() {
        return Err(format!(
            "Cannot read Infer engine inventory ({})",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Cannot read Infer engine inventory")?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err("Infer engine inventory is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let inventory: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Infer engine inventory")?;
    if !inventory
        .get("engines")
        .is_some_and(serde_json::Value::is_array)
    {
        return Err("Invalid Infer engine inventory".into());
    }
    Ok(inventory)
}

pub(crate) async fn agent_settings(
    state: &AppState,
) -> Result<(settings::Settings, Option<String>), String> {
    let (mut effective, key) = settings::snapshot_with_api_key(&state.settings);
    let catalog = read_roles(&effective.base_url, key.as_deref()).await?;
    effective.models.agent = configured_role(&catalog, "agent")
        .ok_or("Configure an Agent model in the Infer catalog")?;
    Ok((effective, key))
}

pub(crate) fn configured_role(catalog: &serde_json::Value, role: &str) -> Option<String> {
    catalog
        .get("roles")?
        .get(role)?
        .as_str()
        .filter(|id| !id.trim().is_empty())
        .map(str::to_owned)
}

#[cfg(all(windows, feature = "embedded-infer"))]
pub(crate) async fn select_agent(state: &AppState, model: &str) -> Result<(), String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    select_agent_at(&settings.base_url, key.as_deref(), model).await
}

async fn select_agent_at(base_url: &str, key: Option<&str>, model: &str) -> Result<(), String> {
    let mut document = fetch_catalog(base_url, key, None).await?;
    let mut catalog: serde_json::Value =
        serde_json::from_str(&document.catalog).map_err(|_| "Invalid Infer catalog")?;
    let present_in_raw = catalog
        .get("models")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|models| {
            models
                .iter()
                .any(|entry| entry.get("id").and_then(serde_json::Value::as_str) == Some(model))
        });
    if !present_in_raw {
        let key = key
            .filter(|key| !key.trim().is_empty())
            .ok_or("Infer requires an API key")?;
        validate_url(base_url)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|_| "Cannot create Infer client")?;
        let response = client
            .get(format!("{}/v1/models", base_url.trim_end_matches('/')))
            .bearer_auth(key)
            .send()
            .await
            .map_err(|_| "Cannot read Infer models")?;
        if !response.status().is_success() {
            return Err("Cannot read Infer models".into());
        }
        let models: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Invalid Infer model list")?;
        if !models
            .get("data")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|models| {
                models
                    .iter()
                    .any(|entry| entry.get("id").and_then(serde_json::Value::as_str) == Some(model))
            })
        {
            return Err("The model is not present in the Infer catalog".into());
        }
    }
    let root = catalog.as_object_mut().ok_or("Invalid Infer catalog")?;
    let roles = root.entry("roles").or_insert_with(|| serde_json::json!({}));
    roles
        .as_object_mut()
        .ok_or("Invalid Infer roles")?
        .insert("agent".into(), model.into());
    document.catalog =
        serde_json::to_string_pretty(&catalog).map_err(|_| "Invalid Infer catalog")?;
    fetch_catalog(base_url, key, Some(document)).await?;
    Ok(())
}

#[tauri::command]
pub async fn infer_select_agent(state: State<'_, AppState>, model: String) -> Result<(), String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    select_agent_and_wait(&settings.base_url, key.as_deref(), &model).await
}

pub(crate) async fn select_agent_and_wait(
    base_url: &str,
    key: Option<&str>,
    model: &str,
) -> Result<(), String> {
    select_agent_at(base_url, key, model).await?;
    tokio::time::timeout(std::time::Duration::from_secs(7), async {
        loop {
            let roles = read_roles(base_url, key).await?;
            if configured_role(&roles, "agent").as_deref() == Some(model) {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    })
    .await
    .map_err(|_| {
        "Agent role saved, but Infer has not applied it yet. Restart Infer and retry.".to_string()
    })?
}

#[tauri::command]
pub async fn infer_agent_role(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let catalog = read_roles(&settings.base_url, key.as_deref()).await?;
    Ok(configured_role(&catalog, "agent"))
}

async fn model_request(
    state: &State<'_, AppState>,
    method: reqwest::Method,
    model: &str,
    action: Option<&str>,
    timeout: u64,
) -> Result<reqwest::Response, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Managing models requires an Infer API key")?;
    validate_url(&settings.base_url)?;
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1/management/models/",
        settings.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "Invalid Infer URL")?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| "Invalid Infer URL")?;
        segments.pop_if_empty().push(model);
        if let Some(action) = action {
            segments.push(action);
        }
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    client
        .request(method, url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer".to_string())
}

fn model_error(status: u16, remove: bool) -> String {
    match status {
        401 => "Infer rejected the API key",
        404 | 405 | 501 => "This Infer server cannot do that yet. Update Infer.",
        409 if remove => "This model is used by a role or alias. Choose another model first.",
        409 => "The model is busy. Try again in a moment.",
        422 if remove => "Only reviewed models downloaded by Rune can be removed",
        422 => "Only local models can be loaded or unloaded",
        _ => "Infer could not change the model",
    }
    .into()
}

#[tauri::command]
pub async fn infer_model_remove(
    state: State<'_, AppState>,
    model: String,
) -> Result<Vec<String>, String> {
    let response = model_request(&state, reqwest::Method::DELETE, &model, None, 60).await?;
    if !response.status().is_success() {
        return Err(model_error(response.status().as_u16(), true));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "Invalid response from Infer")?;
    Ok(body
        .get("failed")
        .and_then(serde_json::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
pub async fn infer_model_residency(
    state: State<'_, AppState>,
    model: String,
    load: bool,
) -> Result<(), String> {
    let action = if load { "load" } else { "unload" };
    let response = model_request(&state, reqwest::Method::POST, &model, Some(action), 300).await?;
    if !response.status().is_success() {
        return Err(model_error(response.status().as_u16(), false));
    }
    Ok(())
}

static ICONS: std::sync::Mutex<Option<std::collections::HashMap<String, Option<String>>>> =
    std::sync::Mutex::new(None);

#[tauri::command]
pub async fn infer_model_icon(
    state: State<'_, AppState>,
    model: String,
) -> Result<Option<String>, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let Some(key) = key.filter(|key| !key.trim().is_empty()) else {
        return Ok(None);
    };
    validate_url(&settings.base_url)?;
    let cache_key = format!("{}|{model}", settings.base_url);
    if let Some(known) = ICONS
        .lock()
        .ok()
        .and_then(|cache| cache.as_ref()?.get(&cache_key).cloned())
    {
        return Ok(known);
    }
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1/management/model-icon/",
        settings.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "Invalid Infer URL")?;
    url.path_segments_mut()
        .map_err(|_| "Invalid Infer URL")?
        .pop_if_empty()
        .push(&model);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let Ok(response) = client.get(url).bearer_auth(key).send().await else {
        return Ok(None);
    };
    let icon = if response.status().is_success() {
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 {
            return Ok(None);
        }
        use base64::Engine;
        Some(format!(
            "data:image/svg+xml;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    } else if response.status().as_u16() == 404 {
        None
    } else {
        return Ok(None);
    };
    if let Ok(mut cache) = ICONS.lock() {
        cache
            .get_or_insert_with(Default::default)
            .insert(cache_key, icon.clone());
    }
    Ok(icon)
}

#[tauri::command]
pub async fn infer_analysis_role(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let catalog = read_roles(&settings.base_url, key.as_deref()).await?;
    Ok(configured_role(&catalog, "analysis"))
}

#[tauri::command]
pub async fn infer_visual_roles(
    state: State<'_, AppState>,
) -> Result<settings::VisualAnalysisModels, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let catalog = read_roles(&settings.base_url, key.as_deref()).await?;
    let role = |name| configured_role(&catalog, name).unwrap_or_default();
    Ok(settings::VisualAnalysisModels {
        planner: role("visual_planner"),
        entities: role("visual_entities"),
        regions: role("visual_regions"),
        structure: role("visual_structure"),
        depth: role("visual_depth"),
        instances: role("visual_instances"),
        oriented: role("visual_oriented"),
    })
}

#[tauri::command]
pub async fn infer_catalog_read(
    state: State<'_, AppState>,
) -> Result<StudioCatalogDocument, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let document = fetch_catalog(&settings.base_url, key.as_deref(), None).await?;
    Ok(StudioCatalogDocument {
        revision: document.revision,
        catalog: document.catalog,
        connection_id: settings.active_connection_id,
        connection_revision: settings.connections_revision,
    })
}

#[tauri::command]
pub async fn infer_catalog_write(
    state: State<'_, AppState>,
    document: StudioCatalogDocument,
) -> Result<StudioCatalogDocument, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    if !catalog_connection_matches(&document, &settings) {
        return Err("Infer connection changed; reload the catalog before saving".into());
    }
    let updated = fetch_catalog(
        &settings.base_url,
        key.as_deref(),
        Some(CatalogDocument {
            revision: document.revision,
            catalog: document.catalog,
        }),
    )
    .await?;
    Ok(StudioCatalogDocument {
        revision: updated.revision,
        catalog: updated.catalog,
        connection_id: settings.active_connection_id,
        connection_revision: settings.connections_revision,
    })
}

fn catalog_connection_matches(
    document: &StudioCatalogDocument,
    settings: &settings::Settings,
) -> bool {
    document.connection_id == settings.active_connection_id
        && document.connection_revision == settings.connections_revision
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn catalog_write_rejects_a_changed_connection_or_revision() {
        let mut settings = settings::Settings::default();
        let document = StudioCatalogDocument {
            revision: "r".into(),
            catalog: "{}".into(),
            connection_id: settings.active_connection_id.clone(),
            connection_revision: settings.connections_revision,
        };
        assert!(catalog_connection_matches(&document, &settings));
        settings.active_connection_id = "other".into();
        assert!(!catalog_connection_matches(&document, &settings));
        settings.active_connection_id = document.connection_id.clone();
        settings.connections_revision += 1;
        assert!(!catalog_connection_matches(&document, &settings));
    }

    #[test]
    fn roles_come_from_infer_and_unconfigured_operations_stay_disabled() {
        let catalog =
            serde_json::json!({"roles":{"visual_planner":"planner", "visual_depth":"depth"}});
        assert_eq!(
            configured_role(&catalog, "visual_planner").as_deref(),
            Some("planner")
        );
        assert_eq!(
            configured_role(&catalog, "visual_depth").as_deref(),
            Some("depth")
        );
        assert_eq!(configured_role(&catalog, "visual_entities"), None);
        assert_eq!(configured_role(&catalog, "visual_regions"), None);
    }

    #[tokio::test]
    async fn reads_authenticated_effective_roles() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let count = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
            assert!(request.starts_with("get /v1/management/roles "));
            assert!(request.contains("authorization: bearer test-key"));
            let body = r#"{"roles":{"agent":"resolved-model"}}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let roles = read_roles(&format!("http://{address}"), Some("test-key"))
            .await
            .unwrap();
        assert_eq!(
            configured_role(&roles, "agent").as_deref(),
            Some("resolved-model")
        );
        assert!(
            read_roles(&format!("http://{address}"), None)
                .await
                .is_err()
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn manages_model_downloads_and_scans_through_infer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (method, path, body) in [
                ("get", "/v1/management/model-download", r#"{"phase":""}"#),
                (
                    "post",
                    "/v1/management/model-download",
                    r#"{"phase":"preparing"}"#,
                ),
                (
                    "delete",
                    "/v1/management/model-download",
                    r#"{"phase":"cancelled"}"#,
                ),
                ("post", "/v1/management/model-scan", r#"[]"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 8192];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
                assert!(request.starts_with(&format!("{method} {path} ")));
                assert!(request.contains("authorization: bearer test-key"));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let url = format!("http://{address}");
        assert!(
            model_download_request(&url, None, reqwest::Method::GET, None)
                .await
                .is_err()
        );
        for (method, body) in [
            (reqwest::Method::GET, None),
            (
                reqwest::Method::POST,
                Some(serde_json::json!({"id":"fixture"})),
            ),
            (reqwest::Method::DELETE, None),
        ] {
            model_download_request(&url, Some("test-key"), method, body)
                .await
                .unwrap();
        }
        assert!(
            scan_models_request(&url, Some("test-key"), "C:\\models")
                .await
                .unwrap()
                .is_array()
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn reads_model_candidates_from_authenticated_infer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let count = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
            assert!(
                request.starts_with("get /v1/management/model-candidates?purpose=computer-use ")
            );
            assert!(request.contains("authorization: bearer test-key"));
            let body = r#"[{"id":"fixture","recommended":true}]"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let url = format!("http://{address}");
        assert!(
            read_model_candidates(&url, None, "computer-use")
                .await
                .is_err()
        );
        assert!(
            read_model_candidates(&url, Some("test-key"), "invalid")
                .await
                .is_err()
        );
        let candidates = read_model_candidates(&url, Some("test-key"), "computer-use")
            .await
            .unwrap();
        assert_eq!(candidates[0]["id"], "fixture");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn starts_and_reads_engine_updates_through_infer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for method in ["post", "get", "delete"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
                assert!(
                    request.starts_with(&format!("{method} /v1/management/engines/needle/update "))
                );
                assert!(request.contains("authorization: bearer test-key"));
                let body = if method == "delete" {
                    r#"{"name":"needle","operation":"update","status":"cancelled","message":"Cancelled"}"#
                } else {
                    r#"{"name":"needle","operation":"update","status":"running","message":"Updating engine"}"#
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let url = format!("http://{address}");
        assert!(
            engine_operation_request(&url, Some("test-key"), "../needle", "update", true)
                .await
                .is_err()
        );
        for start in [true, false] {
            assert_eq!(
                engine_operation_request(&url, Some("test-key"), "needle", "update", start)
                    .await
                    .unwrap()["operation"],
                "update"
            );
        }
        assert_eq!(
            engine_operation_with_method(
                &url,
                Some("test-key"),
                "needle",
                "update",
                reqwest::Method::DELETE
            )
            .await
            .unwrap()["status"],
            "cancelled"
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn starts_and_reads_engine_installation_through_infer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for method in ["post", "get"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
                assert!(request.starts_with(&format!(
                    "{method} /v1/management/engines/needle/installation "
                )));
                assert!(request.contains("authorization: bearer test-key"));
                let body = r#"{"name":"needle","status":"running","message":"Installing engine"}"#;
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        let url = format!("http://{address}");
        assert!(
            engine_installation_request(&url, Some("test-key"), "../needle", true)
                .await
                .is_err()
        );
        assert!(
            engine_installation_request(&url, None, "needle", true)
                .await
                .is_err()
        );
        for start in [true, false] {
            assert_eq!(
                engine_installation_request(&url, Some("test-key"), "needle", start)
                    .await
                    .unwrap()["status"],
                "running"
            );
        }
        server.join().unwrap();
    }

    #[tokio::test]
    async fn checks_engine_release_through_authenticated_infer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let count = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
            assert!(request.starts_with("post /v1/management/engines/needle/release-check "));
            assert!(request.contains("authorization: bearer test-key"));
            let body = r#"{"latest":"revision 123","detail":"Verified source"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let url = format!("http://{address}");
        assert!(
            read_engine_release(&url, Some("test-key"), "../needle")
                .await
                .is_err()
        );
        assert!(read_engine_release(&url, None, "needle").await.is_err());
        assert_eq!(
            read_engine_release(&url, Some("test-key"), "needle")
                .await
                .unwrap()["latest"],
            "revision 123"
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn reads_engine_inventory_with_key_and_reports_older_servers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (status, body) in [
                (
                    "200 OK",
                    r#"{"engines":[{"name":"encoder","installed":true,"managed":false,"version":"Built in"}]}"#,
                ),
                ("404 Not Found", ""),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
                assert!(request.starts_with("get /v1/management/engines "));
                assert!(request.contains("authorization: bearer test-key"));
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        let url = format!("http://{address}");
        assert_eq!(
            read_engines(&url, Some("test-key")).await.unwrap()["engines"][0]["name"],
            "encoder"
        );
        assert!(
            read_engines(&url, Some("test-key"))
                .await
                .unwrap_err()
                .contains("Update Infer")
        );
        assert!(read_engines(&url, None).await.is_err());
        server.join().unwrap();
    }

    #[tokio::test]
    async fn verifies_authenticated_infer_catalog_and_rejects_generic_servers() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for valid in [true, false] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).to_ascii_lowercase();
                assert!(request.starts_with("get /v1/management/catalog "));
                assert!(request.contains("authorization: bearer test-key"));
                let body = if valid {
                    r#"{"revision":"r","catalog":"{\"schema_version\":3,\"models\":[]}"}"#
                } else {
                    "{}"
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        let url = format!("http://{address}");
        assert!(verify_infer(&url, Some("test-key")).await.is_ok());
        assert!(verify_infer(&url, Some("test-key")).await.is_err());
        assert!(verify_infer(&url, None).await.is_err());
        server.join().unwrap();
    }
}
