//! Saved model endpoints. Credentials are stored in the system keyring by stable ID.
use crate::{
    backend,
    error::{AppError, AppResult},
    events::{self, AppState},
    settings::{self, Connection, Settings},
    sync::LockExt,
};
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    id: String,
    name: String,
    base_url: String,
    active: bool,
    has_api_key: bool,
    selected_models: Option<Vec<String>>,
    additional_models: Vec<String>,
    output_limit: Option<u32>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    revision: u64,
    active_id: String,
    connections: Vec<ConnectionView>,
}
fn catalog(settings: &Settings) -> Catalog {
    Catalog {
        revision: settings.connections_revision,
        active_id: settings.active_connection_id.clone(),
        connections: settings
            .connections
            .iter()
            .map(|c| ConnectionView {
                id: c.id.clone(),
                name: c.name.clone(),
                base_url: c.base_url.clone(),
                active: c.id == settings.active_connection_id,
                has_api_key: settings::api_key_for_connection(c).is_some(),
                selected_models: c.selected_models.clone(),
                additional_models: c.additional_models.clone(),
                output_limit: c.output_limit,
            })
            .collect(),
    }
}
fn check_revision(settings: &Settings, revision: u64) -> AppResult<()> {
    if settings.connections_revision != revision {
        return Err(AppError::Invalid(
            "Connections changed in another window. Reload before saving.".into(),
        ));
    }
    Ok(())
}
fn select_active(settings: &mut Settings, id: &str) -> AppResult<()> {
    let connection = settings
        .connections
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::Invalid("Connection no longer exists".into()))?;
    settings.base_url = connection.base_url.clone();
    settings.active_connection_id = id.into();
    Ok(())
}
fn remove_from_catalog(settings: &mut Settings, id: &str) -> AppResult<()> {
    if settings.connections.len() <= 1 {
        return Err(AppError::Invalid(
            "Keep at least one model connection.".into(),
        ));
    }
    if !settings.connections.iter().any(|c| c.id == id) {
        return Err(AppError::Invalid("Connection no longer exists".into()));
    }
    let was_active = settings.active_connection_id == id;
    settings.connections.retain(|c| c.id != id);
    if was_active {
        let id = settings.connections[0].id.clone();
        select_active(settings, &id)?;
    }
    Ok(())
}
pub(crate) fn ensure_idle(
    app: &AppHandle,
    state: &AppState,
    analysis: crate::analysis::report::Status,
) -> AppResult<()> {
    let active = crate::tasks::connection_change_blocked();
    let state_label = events::agent_state(app);
    if state_label.is_running()
        || matches!(
            state_label,
            events::AgentState::Listening | events::AgentState::Reflecting
        )
        || state.lease.locked().is_held()
        || state.mic.locked().is_some()
        || crate::analysis::state::in_flight(analysis)
        || crate::intake::busy()
        || active
    {
        return Err(AppError::Invalid("Finish or stop active and queued work before changing the active connection or its credential.".into()));
    }
    Ok(())
}
fn next_id() -> AppResult<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| AppError::Invalid(e.to_string()))?;
    Ok(format!(
        "conn-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
#[tauri::command]
pub fn connections_state(state: State<'_, AppState>) -> Catalog {
    catalog(&state.settings.locked())
}

#[tauri::command]
pub fn connections_models_save(
    app: AppHandle,
    state: State<'_, AppState>,
    revision: u64,
    id: String,
    selected_models: Vec<String>,
    additional_models: Vec<String>,
    output_limit: Option<u32>,
) -> AppResult<Catalog> {
    let valid_ids = |ids: &[String]| {
        ids.len() <= 1000
            && ids
                .iter()
                .all(|id| !id.trim().is_empty() && id.len() <= 256 && id.trim() == id)
    };
    if output_limit == Some(0)
        || output_limit.is_some_and(|n| n > 1_000_000)
        || !valid_ids(&additional_models)
        || !valid_ids(&selected_models)
    {
        return Err(AppError::Invalid(
            "Invalid model IDs or output limit".into(),
        ));
    }
    let analysis = state.analysis.locked();
    let mut current = state.settings.locked();
    check_revision(&current, revision)?;
    settings::ensure_current(&app, &current)?;
    if current.active_connection_id == id {
        ensure_idle(&app, &state, analysis.status)?;
    }
    let mut next = current.clone();
    let connection = next
        .connections
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::Invalid("Connection no longer exists".into()))?;
    connection.selected_models = Some(selected_models);
    connection.additional_models = additional_models;
    connection.output_limit = output_limit;
    next.connections_revision = next
        .connections_revision
        .checked_add(1)
        .ok_or_else(|| AppError::Invalid("Connection revision limit reached".into()))?;
    settings::save(&app, &next)?;
    *current = next;
    let view = catalog(&current);
    drop(current);
    drop(analysis);
    crate::hub::changed(&app);
    Ok(view)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri exposes these as individual command fields.
pub fn connections_save(
    app: AppHandle,
    state: State<'_, AppState>,
    revision: u64,
    id: Option<String>,
    name: String,
    base_url: String,
    key: Option<String>,
    clear_key: bool,
) -> AppResult<Catalog> {
    settings::validate_connection(&name, &base_url)?;
    if clear_key && key.as_ref().is_some_and(|s| !s.trim().is_empty()) {
        return Err(AppError::Invalid(
            "Choose either a replacement key or removal".into(),
        ));
    }
    if key.as_ref().is_some_and(|s| s.len() > 16_384) {
        return Err(AppError::Invalid("API key is too long".into()));
    }
    let analysis = state.analysis.locked();
    let mut current = state.settings.locked();
    check_revision(&current, revision)?;
    settings::ensure_current(&app, &current)?;
    let mut next = current.clone();
    let new = id.is_none();
    let id = match id {
        Some(id) => id,
        None => next_id()?,
    };
    let old_slot = current
        .connections
        .iter()
        .find(|c| c.id == id)
        .and_then(|c| c.credential_ref.clone());
    if !settings::valid_connection_id(&id) {
        return Err(AppError::Invalid("Invalid connection ID".into()));
    }
    let active = id == next.active_connection_id;
    let changed_url = next
        .connections
        .iter()
        .find(|c| c.id == id)
        .is_some_and(|c| c.base_url != base_url.trim());
    if active && (changed_url || clear_key || key.as_ref().is_some_and(|s| !s.trim().is_empty())) {
        ensure_idle(&app, &state, analysis.status)?;
    }
    if new {
        if next.connections.len() >= 100 {
            return Err(AppError::Invalid(
                "At most 100 connections can be saved".into(),
            ));
        }
        next.connections.push(Connection {
            id: id.clone(),
            name: name.trim().into(),
            base_url: base_url.trim().into(),
            credential_ref: None,
            selected_models: None,
            additional_models: Vec::new(),
            output_limit: None,
        });
    } else {
        let item = next
            .connections
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| AppError::Invalid("Connection no longer exists".into()))?;
        item.name = name.trim().into();
        item.base_url = base_url.trim().into();
        if active {
            next.base_url = item.base_url.clone();
        }
    }
    next.connections_revision = next
        .connections_revision
        .checked_add(1)
        .ok_or_else(|| AppError::Invalid("Connection revision limit reached".into()))?;
    let replacement = key.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let new_slot = if let Some(key) = replacement {
        let slot = next_id()?;
        if settings::stored_api_key_for_slot(&slot)?.is_some() {
            return Err(AppError::Invalid(
                "A credential slot already exists; try again".into(),
            ));
        }
        settings::set_api_key_for_slot(&slot, key)?;
        Some(slot)
    } else {
        None
    };
    if clear_key || new_slot.is_some() {
        let item = next
            .connections
            .iter_mut()
            .find(|c| c.id == id)
            .expect("validated connection exists");
        item.credential_ref = if clear_key && id == settings::LEGACY_CONNECTION {
            Some("legacy-env".into())
        } else {
            new_slot.clone()
        };
    }
    if let Err(error) = settings::save(&app, &next) {
        if let Some(slot) = &new_slot
            && let Err(restore) = settings::set_api_key_for_slot(slot, "")
        {
            return Err(AppError::Invalid(format!(
                "Settings were not saved and the unused new credential could not be removed: {restore}"
            )));
        }
        return Err(error);
    }
    *current = next;
    if active && (changed_url || clear_key || replacement.is_some()) {
        state.capabilities.invalidate();
    }
    let view = catalog(&current);
    drop(current);
    drop(analysis);
    crate::hub::changed(&app);
    if (clear_key || new_slot.is_some())
        && let Some(slot) = old_slot.filter(|slot| slot != "legacy-env")
        && let Err(error) = settings::set_api_key_for_slot(&slot, "")
    {
        return Err(AppError::Invalid(format!(
            "Connection saved, but its previous credential could not be removed: {error}"
        )));
    }
    Ok(view)
}

#[tauri::command]
pub fn connections_activate(
    app: AppHandle,
    state: State<'_, AppState>,
    revision: u64,
    id: String,
) -> AppResult<Catalog> {
    let analysis = state.analysis.locked();
    let mut current = state.settings.locked();
    check_revision(&current, revision)?;
    settings::ensure_current(&app, &current)?;
    if current.active_connection_id == id {
        return Ok(catalog(&current));
    }
    ensure_idle(&app, &state, analysis.status)?;
    let mut next = current.clone();
    select_active(&mut next, &id)?;
    next.connections_revision = next
        .connections_revision
        .checked_add(1)
        .ok_or_else(|| AppError::Invalid("Connection revision limit reached".into()))?;
    settings::save(&app, &next)?;
    *current = next;
    state.capabilities.invalidate();
    let view = catalog(&current);
    drop(current);
    drop(analysis);
    crate::hub::changed(&app);
    Ok(view)
}

#[tauri::command]
pub fn connections_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    revision: u64,
    id: String,
) -> AppResult<Catalog> {
    let analysis = state.analysis.locked();
    let mut current = state.settings.locked();
    check_revision(&current, revision)?;
    settings::ensure_current(&app, &current)?;
    let slot = current
        .connections
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::Invalid("Connection no longer exists".into()))?
        .credential_ref
        .clone();
    let active = current.active_connection_id == id;
    if active {
        ensure_idle(&app, &state, analysis.status)?;
    }
    let mut next = current.clone();
    remove_from_catalog(&mut next, &id)?;
    next.connections_revision = next
        .connections_revision
        .checked_add(1)
        .ok_or_else(|| AppError::Invalid("Connection revision limit reached".into()))?;
    settings::save(&app, &next)?;
    *current = next;
    if active {
        state.capabilities.invalidate();
    }
    let view = catalog(&current);
    drop(current);
    drop(analysis);
    crate::hub::changed(&app);
    if let Some(slot) = slot.filter(|slot| slot != "legacy-env")
        && let Err(error) = settings::set_api_key_for_slot(&slot, "")
    {
        return Err(AppError::Invalid(format!(
            "Connection was removed, but its stored credential could not be deleted: {error}"
        )));
    }
    Ok(view)
}

#[tauri::command]
pub async fn connections_test(
    state: State<'_, AppState>,
    id: String,
    revision: u64,
) -> AppResult<backend::BackendStatus> {
    let (probe, key) = {
        let current = state.settings.locked();
        check_revision(&current, revision)?;
        let connection = current
            .connections
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| AppError::Invalid("Connection no longer exists".into()))?;
        let mut probe = current.clone();
        probe.base_url = connection.base_url.clone();
        (probe, settings::api_key_for_connection(connection))
    };
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    Ok(backend::check(&client, &probe, key.as_deref()).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switch_and_active_deletion_preserve_shared_roles_and_choose_remaining_connection() {
        let mut s = Settings::default();
        s.models.agent = "shared-model".into();
        s.connections.push(Connection {
            id: "conn-two".into(),
            name: "Second".into(),
            base_url: "http://127.0.0.1:9000".into(),
            credential_ref: None,
            selected_models: None,
            additional_models: Vec::new(),
            output_limit: None,
        });
        select_active(&mut s, "conn-two").unwrap();
        assert_eq!(s.base_url, "http://127.0.0.1:9000");
        assert_eq!(s.models.agent, "shared-model");
        remove_from_catalog(&mut s, "conn-two").unwrap();
        assert_eq!(s.active_connection_id, settings::LEGACY_CONNECTION);
        assert_eq!(s.base_url, "http://127.0.0.1:8080");
        assert_eq!(s.models.agent, "shared-model");
        assert!(remove_from_catalog(&mut s, settings::LEGACY_CONNECTION).is_err());
        assert!(select_active(&mut s, "missing").is_err());
        assert!(check_revision(&s, 1).is_err());
    }
}
