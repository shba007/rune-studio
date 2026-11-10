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
    select_agent(&state, &model).await?;
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    tokio::time::timeout(std::time::Duration::from_secs(7), async {
        loop {
            let roles = read_roles(&settings.base_url, key.as_deref()).await?;
            if configured_role(&roles, "agent").as_deref() == Some(model.as_str()) {
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

#[tauri::command]
pub async fn infer_model_remove(
    state: State<'_, AppState>,
    model: String,
) -> Result<Vec<String>, String> {
    let (settings, key) = settings::snapshot_with_api_key(&state.settings);
    let key = key
        .filter(|key| !key.trim().is_empty())
        .ok_or("Removing a model requires an Infer API key")?;
    validate_url(&settings.base_url)?;
    let mut url = reqwest::Url::parse(&format!(
        "{}/v1/management/models/",
        settings.base_url.trim_end_matches('/')
    ))
    .map_err(|_| "Invalid Infer URL")?;
    url.path_segments_mut()
        .map_err(|_| "Invalid Infer URL")?
        .pop_if_empty()
        .push(&model);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|_| "Cannot create Infer client")?;
    let response = client
        .delete(url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Cannot reach Infer")?;
    let status = response.status();
    if !status.is_success() {
        let message = match status.as_u16() {
            401 => "Infer rejected the API key",
            404 => "Model is not installed",
            409 => "This model is used by a role or alias. Choose another model first.",
            422 => "Only reviewed models downloaded by Rune can be removed",
            501 | 405 => "This Infer server cannot remove models yet. Update Infer.",
            _ => "Infer could not remove the model",
        };
        return Err(message.into());
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "Invalid response from Infer")?;
    let failed: Vec<String> = body
        .get("failed")
        .and_then(serde_json::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Ok(failed)
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
