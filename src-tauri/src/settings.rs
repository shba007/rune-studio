use std::fs;
use std::path::PathBuf;

use crate::sync::LockExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{error::AppResult, hotkeys, model::role::ModelRole};

pub(crate) const KEYRING_SERVICE: &str = "dev.shirsendu-bairagi.rune-studio";
pub const DEFAULT_CONNECTION: &str = "default";
const DEFAULT_AGENT_MODEL: &str = "qwen3.5-4b-q4";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub selected_models: Option<Vec<String>>,
    #[serde(default)]
    pub additional_models: Vec<String>,
    #[serde(default)]
    pub output_limit: Option<u32>,
}
fn default_connection(base_url: &str) -> Connection {
    Connection {
        id: DEFAULT_CONNECTION.into(),
        name: "Model connection".into(),
        base_url: base_url.into(),
        credential_ref: None,
        selected_models: None,
        additional_models: Vec::new(),
        output_limit: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRoles {
    #[serde(default = "default_agent_model")]
    pub agent: String,
    #[serde(default)]
    pub verifier: String,
    #[serde(default)]
    pub analysis: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default = "default_decision_model")]
    pub decision: String,
    #[serde(default)]
    pub advisor: String,
}

fn default_agent_model() -> String {
    DEFAULT_AGENT_MODEL.into()
}

fn default_decision_model() -> String {
    "liquid-d1-3b".into()
}

impl Default for ModelRoles {
    fn default() -> Self {
        Self {
            agent: default_agent_model(),
            verifier: String::new(),
            analysis: String::new(),
            voice: String::new(),
            decision: default_decision_model(),
            advisor: String::new(),
        }
    }
}

impl ModelRoles {
    pub fn model_for_role(&self, role: ModelRole) -> &str {
        match role {
            ModelRole::Agent => {
                if !self.agent.is_empty() {
                    &self.agent
                } else {
                    DEFAULT_AGENT_MODEL
                }
            }
            ModelRole::Verifier => {
                if !self.verifier.is_empty() {
                    &self.verifier
                } else {
                    self.model_for_role(ModelRole::Agent)
                }
            }
            ModelRole::Analysis => {
                if !self.analysis.is_empty() {
                    &self.analysis
                } else {
                    self.model_for_role(ModelRole::Agent)
                }
            }
            ModelRole::Voice => {
                if !self.voice.is_empty() {
                    &self.voice
                } else {
                    self.model_for_role(ModelRole::Agent)
                }
            }
            ModelRole::Decision => {
                if !self.decision.is_empty() {
                    &self.decision
                } else {
                    self.model_for_role(ModelRole::Agent)
                }
            }
            ModelRole::Advisor => {
                if !self.advisor.is_empty() {
                    &self.advisor
                } else {
                    self.model_for_role(ModelRole::Agent)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub base_url: String,
    pub connections: Vec<Connection>,
    pub active_connection_id: String,
    pub connections_revision: u64,
    pub model: String,
    pub models: ModelRoles,
    pub max_steps: u32,
    pub toggle_hotkey: String,
    pub stop_hotkey: String,
    pub external_agent: bool,
    pub external_agent_since: u64,
    pub show_thinking: bool,
    pub ask_every_action: bool,
    pub input_classification: InputClassification,
    pub developer_mode: bool,
    pub fast_checks: bool,
    pub decision_model: String,
    pub decision_url: String,
    pub archive_keep_days: u32,
    pub verify_mode: VerifyMode,
    pub record_session_video: RecordVideo,
    pub model_image_resolution: ImageResolution,
    pub recording_resolution: ImageResolution,
    pub visual_analysis: VisualAnalysis,
    pub visual_analysis_models: VisualAnalysisModels,
    pub offer_review: OfferReview,
    pub demo_mode: bool,
    pub reflection_enabled: bool,
    pub idle_minutes: u32,
    pub skip_swap_for_optional_roles: bool,
    pub first_call_idle_allowance_secs: u32,
    pub life_enabled: bool,
    pub life_consents: std::collections::HashMap<String, bool>,
    pub retention_days: std::collections::HashMap<String, u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SettingsDeHelper {
    pub base_url: String,
    pub connections: Option<Vec<Connection>>,
    pub active_connection_id: Option<String>,
    pub connections_revision: u64,
    pub models: Option<ModelRoles>,
    pub max_steps: u32,
    pub toggle_hotkey: String,
    pub stop_hotkey: String,
    pub external_agent: bool,
    pub external_agent_since: u64,
    pub show_thinking: bool,
    pub ask_every_action: bool,
    pub input_classification: InputClassification,
    pub developer_mode: bool,
    pub fast_checks: bool,
    pub decision_url: String,
    pub archive_keep_days: u32,
    pub verify_mode: VerifyMode,
    pub record_session_video: RecordVideo,
    pub model_image_resolution: ImageResolution,
    pub recording_resolution: ImageResolution,
    pub visual_analysis: VisualAnalysis,
    pub visual_analysis_models: VisualAnalysisModels,
    pub offer_review: OfferReview,
    pub demo_mode: bool,
    pub reflection_enabled: bool,
    pub idle_minutes: u32,
    pub skip_swap_for_optional_roles: bool,
    pub first_call_idle_allowance_secs: u32,
    pub life_enabled: bool,
    pub life_consents: std::collections::HashMap<String, bool>,
    pub retention_days: std::collections::HashMap<String, u32>,
}

impl Default for SettingsDeHelper {
    fn default() -> Self {
        let def = Settings::default();
        Self {
            base_url: def.base_url,
            connections: None,
            active_connection_id: None,
            connections_revision: 0,
            models: None,
            max_steps: def.max_steps,
            toggle_hotkey: def.toggle_hotkey,
            stop_hotkey: def.stop_hotkey,
            external_agent: def.external_agent,
            external_agent_since: def.external_agent_since,
            show_thinking: def.show_thinking,
            ask_every_action: def.ask_every_action,
            input_classification: def.input_classification,
            developer_mode: def.developer_mode,
            fast_checks: def.fast_checks,
            decision_url: def.decision_url,
            archive_keep_days: def.archive_keep_days,
            verify_mode: def.verify_mode,
            record_session_video: def.record_session_video,
            model_image_resolution: def.model_image_resolution,
            recording_resolution: def.recording_resolution,
            visual_analysis: def.visual_analysis,
            visual_analysis_models: def.visual_analysis_models,
            offer_review: def.offer_review,
            demo_mode: def.demo_mode,
            reflection_enabled: def.reflection_enabled,
            idle_minutes: def.idle_minutes,
            skip_swap_for_optional_roles: def.skip_swap_for_optional_roles,
            first_call_idle_allowance_secs: def.first_call_idle_allowance_secs,
            life_enabled: def.life_enabled,
            life_consents: def.life_consents,
            retention_days: def.retention_days,
        }
    }
}

impl<'de> Deserialize<'de> for Settings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = SettingsDeHelper::deserialize(deserializer)?;
        let models = helper.models.ok_or_else(|| {
            serde::de::Error::custom("Unsupported settings format: missing model roles")
        })?;
        let synced_model = models.agent.clone();
        let synced_decision = models.decision.clone();
        let mut connections = helper.connections.ok_or_else(|| {
            serde::de::Error::custom("Unsupported settings format: missing connections")
        })?;
        let active_connection_id = helper.active_connection_id.ok_or_else(|| {
            serde::de::Error::custom("Unsupported settings format: missing active connection")
        })?;
        if connections.is_empty()
            || connections.len() > 100
            || connections.iter().any(|c| {
                !valid_connection_id(&c.id)
                    || c.name.trim().is_empty()
                    || c.name.len() > 160
                    || c.base_url.len() > 2048
                    || c.output_limit.is_some_and(|n| n == 0 || n > 1_000_000)
                    || c.additional_models.len() > 1000
                    || c.selected_models
                        .as_ref()
                        .is_some_and(|ids| ids.len() > 1000)
                    || c.credential_ref
                        .as_ref()
                        .is_some_and(|key| !valid_connection_id(key))
            })
            || connections
                .iter()
                .filter(|c| c.id == active_connection_id)
                .count()
                != 1
            || connections
                .iter()
                .enumerate()
                .any(|(i, c)| connections[..i].iter().any(|p| p.id == c.id))
            || connections.iter().enumerate().any(|(i, c)| {
                c.credential_ref.as_ref().is_some_and(|key| {
                    connections[..i]
                        .iter()
                        .any(|p| p.credential_ref.as_ref() == Some(key))
                })
            })
        {
            return Err(serde::de::Error::custom("Invalid saved model connections"));
        }
        if let Some(active) = connections
            .iter_mut()
            .find(|c| c.id == active_connection_id)
        {
            active.base_url = helper.base_url.clone();
        }
        Ok(Settings {
            base_url: helper.base_url,
            connections,
            active_connection_id,
            connections_revision: helper.connections_revision,
            model: synced_model,
            models,
            max_steps: helper.max_steps,
            toggle_hotkey: helper.toggle_hotkey,
            stop_hotkey: helper.stop_hotkey,
            external_agent: helper.external_agent,
            external_agent_since: helper.external_agent_since,
            show_thinking: helper.show_thinking,
            ask_every_action: helper.ask_every_action,
            input_classification: helper.input_classification,
            developer_mode: helper.developer_mode,
            fast_checks: helper.fast_checks,
            decision_model: synced_decision,
            decision_url: helper.decision_url,
            archive_keep_days: helper.archive_keep_days,
            verify_mode: helper.verify_mode,
            record_session_video: helper.record_session_video,
            model_image_resolution: helper.model_image_resolution,
            recording_resolution: helper.recording_resolution,
            visual_analysis: helper.visual_analysis,
            visual_analysis_models: helper.visual_analysis_models,
            offer_review: helper.offer_review,
            demo_mode: helper.demo_mode,
            reflection_enabled: helper.reflection_enabled,
            idle_minutes: helper.idle_minutes,
            skip_swap_for_optional_roles: helper.skip_swap_for_optional_roles,
            first_call_idle_allowance_secs: helper.first_call_idle_allowance_secs,
            life_enabled: helper.life_enabled,
            life_consents: helper.life_consents,
            retention_days: helper.retention_days,
        })
    }
}

impl Settings {
    pub fn output_limit(&self) -> Option<u32> {
        None
    }

    pub fn model_for_role(&self, role: ModelRole) -> &str {
        self.models.model_for_role(role)
    }
}

pub fn valid_connection_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
pub fn validate_connection(name: &str, base_url: &str) -> AppResult<()> {
    if name.trim().is_empty() || name.chars().count() > 80 || base_url.len() > 2048 {
        return Err(crate::error::AppError::Invalid(
            "Enter a connection name (up to 80 characters) and a valid server URL".into(),
        ));
    }
    let url = reqwest::Url::parse(base_url.trim()).map_err(|_| {
        crate::error::AppError::Invalid("Enter a valid http or https server URL".into())
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(crate::error::AppError::Invalid(
            "Use an http or https URL without embedded credentials, query or fragment".into(),
        ));
    }
    if url.path().trim_end_matches('/').ends_with("/v1") {
        return Err(crate::error::AppError::Invalid(
            "Enter the server root without /v1; Studio adds the API path".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputClassification {
    AlwaysConfirm,
    #[default]
    #[serde(other)]
    Automatic,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResolution {
    #[serde(rename = "720")]
    Small,
    #[serde(rename = "2560")]
    ExtraLarge,
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "1280")]
    Balanced,
    #[default]
    #[serde(rename = "1920", other)]
    Large,
}
impl ImageResolution {
    pub fn max_edge(self) -> u32 {
        match self {
            Self::Small => 720,
            Self::Balanced => 1280,
            Self::Large => 1920,
            Self::ExtraLarge => 2560,
            Self::Native => u32::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordVideo {
    Motion,
    #[default]
    #[serde(other)]
    Off,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OfferReview {
    Failures,
    Never,
    #[default]
    #[serde(other)]
    Always,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifyMode {
    Off,
    Blocking,
    #[default]
    #[serde(other)]
    Background,
}

impl Default for Settings {
    fn default() -> Self {
        let models = ModelRoles::default();
        Self {
            base_url: "http://127.0.0.1:8080".into(),
            connections: vec![default_connection("http://127.0.0.1:8080")],
            active_connection_id: DEFAULT_CONNECTION.into(),
            connections_revision: 0,
            model: models.agent.clone(),
            models,
            max_steps: 30,
            toggle_hotkey: hotkeys::DEFAULT_TOGGLE.into(),
            stop_hotkey: hotkeys::DEFAULT_STOP.into(),
            external_agent: false,
            external_agent_since: 0,
            show_thinking: false,
            ask_every_action: false,
            input_classification: InputClassification::Automatic,
            developer_mode: false,
            fast_checks: false,
            decision_model: "liquid-d1-3b".into(),
            decision_url: String::new(),
            archive_keep_days: 0,
            verify_mode: VerifyMode::default(),
            record_session_video: RecordVideo::default(),
            model_image_resolution: ImageResolution::Large,
            recording_resolution: ImageResolution::Native,
            visual_analysis: VisualAnalysis::default(),
            visual_analysis_models: VisualAnalysisModels::default(),
            offer_review: OfferReview::default(),
            demo_mode: false,
            reflection_enabled: true,
            idle_minutes: 10,
            skip_swap_for_optional_roles: true,
            first_call_idle_allowance_secs: 30,
            life_enabled: false,
            life_consents: {
                let mut m = std::collections::HashMap::new();
                m.insert("general".into(), false);
                m.insert("health".into(), false);
                m.insert("mind".into(), false);
                m.insert("finance".into(), false);
                m
            },
            retention_days: {
                let mut m = std::collections::HashMap::new();
                m.insert("general".into(), 0);
                m.insert("health".into(), 0);
                m.insert("mind".into(), 0);
                m.insert("finance".into(), 0);
                m
            },
        }
    }
}

impl Settings {
    #[cfg(any(feature = "agent-bridge", test))]
    pub fn life_consent(&self, class: &str) -> bool {
        self.life_enabled && self.life_consents.get(class).copied().unwrap_or(false)
    }

    #[cfg(any(feature = "agent-bridge", test))]
    pub fn retention_days_for(&self, class: &str) -> u32 {
        self.retention_days.get(class).copied().unwrap_or(0)
    }
}

#[cfg(feature = "agent-bridge")]
pub fn path_without_app() -> Option<PathBuf> {
    Some(
        PathBuf::from(std::env::var_os("APPDATA")?)
            .join(KEYRING_SERVICE)
            .join("settings.json"),
    )
}

#[cfg(feature = "agent-bridge")]
pub fn read_file(path: &std::path::Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

#[cfg(feature = "agent-bridge")]
pub fn write_file(path: &std::path::Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?,
    )
}

fn settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_config_dir()?.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    let mut settings =
        match settings_path(app).and_then(|path| fs::read_to_string(path).map_err(Into::into)) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => {
                #[cfg(all(windows, feature = "embedded-infer"))]
                let mut settings = Settings::default();
                #[cfg(not(all(windows, feature = "embedded-infer")))]
                let settings = Settings::default();
                #[cfg(all(windows, feature = "embedded-infer"))]
                if settings_path(app).is_ok_and(|path| !path.exists()) {
                    use_local_infer_by_default(&mut settings);
                }
                settings
            }
        };
    normalize_embedded_connection(&mut settings);
    settings
}

#[cfg(any(all(windows, feature = "embedded-infer"), test))]
fn use_local_infer_by_default(settings: &mut Settings) {
    let local = Connection {
        id: "studio-embedded-infer".into(),
        name: "Studio local Infer".into(),
        base_url: "http://127.0.0.1:0".into(),
        credential_ref: None,
        selected_models: None,
        additional_models: Vec::new(),
        output_limit: None,
    };
    settings.base_url = local.base_url.clone();
    settings.active_connection_id = local.id.clone();
    settings.connections.push(local);
}

fn normalize_embedded_connection(settings: &mut Settings) {
    if let Some(connection) = settings.connections.iter_mut().find(|connection| {
        connection.id == "studio-embedded-infer" && connection.name == "Studio local Infer"
    }) {
        connection.base_url = "http://127.0.0.1:0".into();
        if settings.active_connection_id == connection.id {
            settings.base_url = connection.base_url.clone();
        }
    }
}

fn atomic_save(path: &std::path::Path, settings: &Settings) -> AppResult<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    if path.exists() {
        let existing = fs::read(path)?;
        serde_json::from_slice::<Settings>(&existing).map_err(|_| {
            crate::error::AppError::Invalid(
                "Existing settings cannot be read safely; they were not replaced".into(),
            )
        })?;
    }
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|e| {
        crate::error::AppError::Invalid(format!("Cannot create a temporary settings file: {e}"))
    })?;
    let suffix = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let temporary = path.with_file_name(format!(".settings-{suffix}.tmp"));
    let mut created = false;
    let result = (|| -> AppResult<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        created = true;
        file.write_all(serde_json::to_string_pretty(settings)?.as_bytes())?;
        file.sync_all()?;
        drop(file);
        replace_file(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() && created {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(())
}
#[cfg(windows)]
fn replace_file(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
        core::PCWSTR,
    };
    let source: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(std::io::Error::other)
}
#[cfg(not(windows))]
fn replace_file(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    fs::rename(from, to)
}
pub fn save(app: &AppHandle, settings: &Settings) -> AppResult<()> {
    atomic_save(&settings_path(app)?, settings)
}
pub(crate) fn ensure_current(app: &AppHandle, current: &Settings) -> AppResult<()> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(());
    }
    let mut disk: Settings = serde_json::from_slice(&fs::read(path)?).map_err(|_| {
        crate::error::AppError::Invalid(
            "Existing settings cannot be read safely; they were not replaced".into(),
        )
    })?;
    normalize_embedded_connection(&mut disk);
    if disk.connections_revision != current.connections_revision
        || disk.active_connection_id != current.active_connection_id
        || disk.connections != current.connections
    {
        return Err(crate::error::AppError::Invalid(
            "Saved connections changed outside this window. Reload before saving.".into(),
        ));
    }
    Ok(())
}

fn connection_key_entry(id: &str) -> keyring::Result<crate::credentials::Entry> {
    crate::credentials::Entry::new(KEYRING_SERVICE, &format!("rune-model-connection-{id}"))
}
pub fn api_key_for(settings: &Settings) -> Option<String> {
    settings
        .connections
        .iter()
        .find(|c| c.id == settings.active_connection_id)
        .and_then(api_key_for_connection)
}
pub fn snapshot_with_api_key(settings: &std::sync::Mutex<Settings>) -> (Settings, Option<String>) {
    let current = settings.locked();
    let mut effective = current.clone();
    if effective.active_connection_id == "studio-embedded-infer" {
        effective.base_url = "http://127.0.0.1:0".into();
        #[cfg(all(windows, feature = "embedded-infer"))]
        if let Some((url, key)) = crate::embedded_infer::route() {
            effective.base_url = url;
            return (effective, Some(key));
        }
        return (effective, None);
    }
    (effective, api_key_for(&current))
}
pub fn api_key_for_connection(connection: &Connection) -> Option<String> {
    if connection.id == "studio-embedded-infer" {
        #[cfg(all(windows, feature = "embedded-infer"))]
        return crate::embedded_infer::route().map(|(_, key)| key);
        #[cfg(not(all(windows, feature = "embedded-infer")))]
        return None;
    }
    connection
        .credential_ref
        .as_deref()
        .and_then(|slot| {
            connection_key_entry(slot)
                .and_then(|entry| entry.get_password())
                .ok()
        })
        .filter(|key| !key.is_empty())
}
pub fn stored_api_key_for_slot(id: &str) -> AppResult<Option<String>> {
    let entry = connection_key_entry(id)?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
pub fn set_api_key_for_slot(id: &str, key: &str) -> AppResult<()> {
    if !valid_connection_id(id) {
        return Err(crate::error::AppError::Invalid(
            "Invalid connection ID".into(),
        ));
    }
    let entry = connection_key_entry(id)?;
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    } else {
        Ok(entry.set_password(key)?)
    }
}

#[cfg(test)]
fn settings_fixture_value(overrides: serde_json::Value) -> Result<Settings, serde_json::Error> {
    let mut value = serde_json::to_value(Settings::default())?;
    if let Some(fields) = overrides.as_object() {
        value
            .as_object_mut()
            .expect("settings object")
            .extend(fields.clone());
    }
    serde_json::from_value(value)
}

#[cfg(test)]
fn settings_fixture(text: &str) -> Result<Settings, serde_json::Error> {
    settings_fixture_value(serde_json::from_str(text)?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_connection_output_caps_do_not_override_infer_configuration() {
        let mut settings = super::Settings::default();
        settings.connections[0].output_limit = Some(123);
        assert_eq!(settings.output_limit(), None);
    }

    #[test]
    fn unsupported_settings_are_rejected_without_migration() {
        let old = r#"{"baseUrl":"http://127.0.0.1:8080","model":"old"}"#;
        assert!(serde_json::from_str::<super::Settings>(old).is_err());
    }
    #[test]
    fn first_run_local_default_keeps_the_external_connection() {
        use super::*;
        let mut settings = Settings::default();
        use_local_infer_by_default(&mut settings);
        assert_eq!(settings.active_connection_id, "studio-embedded-infer");
        assert_eq!(settings.base_url, "http://127.0.0.1:0");
        assert!(
            settings
                .connections
                .iter()
                .any(|connection| connection.id == DEFAULT_CONNECTION)
        );
    }
    #[test]
    fn saved_embedded_connection_never_retains_a_live_port() {
        use super::*;
        let mut settings = Settings::default();
        settings.connections[0].id = "studio-embedded-infer".into();
        settings.connections[0].name = "Studio local Infer".into();
        settings.connections[0].base_url = "http://127.0.0.1:12345".into();
        settings.active_connection_id = "studio-embedded-infer".into();
        settings.base_url = "http://127.0.0.1:12345".into();
        normalize_embedded_connection(&mut settings);
        assert_eq!(settings.base_url, "http://127.0.0.1:0");
        assert_eq!(settings.connections[0].base_url, settings.base_url);
    }
    #[test]
    fn default_agent_model_applies_to_new_settings_and_empty_agent_role() {
        use super::*;
        let settings = Settings::default();
        assert_eq!(settings.model, "qwen3.5-4b-q4");
        assert_eq!(settings.models.agent, "qwen3.5-4b-q4");
        let empty = ModelRoles {
            agent: String::new(),
            ..ModelRoles::default()
        };
        assert_eq!(empty.model_for_role(ModelRole::Agent), "qwen3.5-4b-q4");
        let saved: Settings =
            settings_fixture(r#"{"model":"custom","models":{"agent":"custom"}}"#).unwrap();
        assert_eq!(saved.models.agent, "custom");
        let selected: Settings =
            settings_fixture(r#"{"model":"old-legacy","models":{"agent":"qwen3.5-4b-q4"}}"#)
                .unwrap();
        assert_eq!(selected.models.agent, "qwen3.5-4b-q4");
    }
    #[test]
    fn invalid_settings_are_never_overwritten_and_staged_credentials_leave_old_key_intact() {
        use super::*;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, b"{broken").unwrap();
        set_api_key_for_slot(DEFAULT_CONNECTION, "original-secret").unwrap();
        set_api_key_for_slot("conn-staged-test", "new-secret").unwrap();
        assert!(atomic_save(&path, &Settings::default()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{broken");
        assert_eq!(
            stored_api_key_for_slot(DEFAULT_CONNECTION)
                .unwrap()
                .as_deref(),
            Some("original-secret")
        );
        set_api_key_for_slot("conn-staged-test", "").unwrap();
        set_api_key_for_slot(DEFAULT_CONNECTION, "").unwrap();
    }
    #[test]
    fn connection_validation_rejects_credentials_and_invalid_registry() {
        use super::*;
        assert!(validate_connection("Local", "http://127.0.0.1:8080").is_ok());
        for value in [
            "http://user:secret@example.com",
            "http://127.0.0.1:8080/v1",
            "file:///tmp/a",
            "https://example.com/?key=secret",
        ] {
            assert!(validate_connection("Local", value).is_err());
        }
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value["connections"][0]["credentialRef"] = serde_json::json!("bad/slot");
        assert!(serde_json::from_value::<Settings>(value).is_err());
    }
    #[test]
    fn classification_preference_defaults_and_round_trips() {
        use super::{InputClassification, Settings};
        for json in ["{}", r#"{"inputClassification":"unknown"}"#] {
            let settings: Settings = settings_fixture(json).unwrap();
            assert_eq!(
                settings.input_classification,
                InputClassification::Automatic
            );
        }
        let settings: Settings =
            settings_fixture(r#"{"inputClassification":"always_confirm"}"#).unwrap();
        let encoded = serde_json::to_value(&settings).unwrap();
        assert_eq!(encoded["inputClassification"], "always_confirm");
    }
    use super::*;

    #[test]
    fn capture_resolution_defaults_and_separate_settings_round_trip() {
        let old: Settings = settings_fixture("{}").unwrap();
        assert_eq!(old.model_image_resolution.max_edge(), 1920);
        assert_eq!(old.recording_resolution, ImageResolution::Native);
        for (name, edge) in [
            ("720", 720),
            ("1280", 1280),
            ("1920", 1920),
            ("2560", 2560),
            ("native", u32::MAX),
        ] {
            let settings: Settings = settings_fixture_value(serde_json::json!({
                "modelImageResolution": name, "recordingResolution": "720"
            }))
            .unwrap();
            assert_eq!(settings.model_image_resolution.max_edge(), edge);
            assert_eq!(settings.recording_resolution.max_edge(), 720);
            let saved = serde_json::to_value(settings).unwrap();
            assert_eq!(saved["modelImageResolution"], name);
            assert_eq!(saved["recordingResolution"], "720");
        }
        let invalid: Settings = settings_fixture(r#"{"modelImageResolution":"invalid"}"#).unwrap();
        assert_eq!(invalid.model_image_resolution.max_edge(), 1920);
    }

    #[test]
    fn role_resolution_with_explicit_roles() {
        let new_fixture = r#"{
            "models": {
                "agent": "agent-model",
                "verifier": "verifier-model",
                "analysis": "",
                "voice": "voice-model",
                "decision": "",
                "advisor": ""
            }
        }"#;

        let settings: Settings = settings_fixture(new_fixture).unwrap();
        assert_eq!(settings.model_for_role(ModelRole::Agent), "agent-model");
        assert_eq!(
            settings.model_for_role(ModelRole::Verifier),
            "verifier-model"
        );
        assert_eq!(settings.model_for_role(ModelRole::Analysis), "agent-model");
        assert_eq!(settings.model_for_role(ModelRole::Voice), "voice-model");
        assert_eq!(settings.model_for_role(ModelRole::Decision), "agent-model");
        assert_eq!(settings.model_for_role(ModelRole::Advisor), "agent-model");
    }

    #[test]
    fn self_reflect_is_on_by_default_with_ten_idle_minutes() {
        let defaults = Settings::default();
        assert!(defaults.reflection_enabled);
        assert_eq!(defaults.idle_minutes, 10);
        let loaded: Settings =
            settings_fixture(r#"{ "reflectionEnabled": false, "idleMinutes": 3 }"#).unwrap();
        assert!(!loaded.reflection_enabled);
        assert_eq!(loaded.idle_minutes, 3);
        assert_eq!(settings_fixture("{}").unwrap().idle_minutes, 10);
    }

    #[test]
    fn demo_mode_is_off_by_default_and_saved_preferences_are_preserved() {
        assert!(!Settings::default().demo_mode);
        assert!(!settings_fixture("{}").unwrap().demo_mode);
        let saved = settings_fixture(
            r#"{"demoMode":true,"modelImageResolution":"720","recordingResolution":"1280"}"#,
        )
        .unwrap();
        assert!(saved.demo_mode);
        assert_eq!(saved.model_image_resolution, ImageResolution::Small);
        assert_eq!(saved.recording_resolution, ImageResolution::Balanced);
        assert!(
            !settings_fixture(r#"{ "demoMode": false }"#)
                .unwrap()
                .demo_mode
        );
    }

    #[test]
    fn the_video_and_review_settings_default_to_off_and_always_and_unknown_words_read_as_those() {
        let defaults = Settings::default();
        assert_eq!(
            (defaults.record_session_video, defaults.offer_review),
            (RecordVideo::Off, OfferReview::Always)
        );
        let set: Settings =
            settings_fixture(r#"{ "recordSessionVideo": "motion", "offerReview": "failures" }"#)
                .unwrap();
        assert_eq!(
            (set.record_session_video, set.offer_review),
            (RecordVideo::Motion, OfferReview::Failures)
        );
        let never: Settings = settings_fixture(r#"{ "offerReview": "never" }"#).unwrap();
        assert_eq!(never.offer_review, OfferReview::Never);
        let typo: Settings = settings_fixture(
            r#"{ "recordSessionVideo": "alwayz", "offerReview": "sometimes", "maxSteps": 7 }"#,
        )
        .unwrap();
        assert_eq!(
            (typo.record_session_video, typo.offer_review, typo.max_steps),
            (RecordVideo::Off, OfferReview::Always, 7)
        );
    }

    #[test]
    fn the_verify_mode_defaults_to_background() {
        assert_eq!(VerifyMode::default(), VerifyMode::Background);
        assert_eq!(Settings::default().verify_mode, VerifyMode::Background);
    }

    #[test]
    fn the_three_modes_are_spelled_in_lower_case_and_an_unknown_word_reads_as_background() {
        for (text, mode) in [
            ("off", VerifyMode::Off),
            ("background", VerifyMode::Background),
            ("blocking", VerifyMode::Blocking),
        ] {
            let settings: Settings =
                settings_fixture(&format!(r#"{{ "verifyMode": "{text}" }}"#)).unwrap();
            assert_eq!(settings.verify_mode, mode, "{text}");
            assert_eq!(serde_json::to_value(mode).unwrap(), text);
        }
        let typo: Settings = settings_fixture(r#"{ "verifyMode": "blokcing", "maxSteps": 7 }"#)
            .expect("still loads");
        assert_eq!(
            (typo.verify_mode, typo.max_steps),
            (VerifyMode::Background, 7)
        );
        assert_eq!(
            serde_json::to_value(Settings::default()).unwrap()["verifyMode"],
            "background"
        );
    }

    #[test]
    fn life_settings_defaults_and_configuration() {
        let defaults = Settings::default();
        assert!(!defaults.life_enabled);
        assert!(!defaults.life_consent("health"));
        assert_eq!(defaults.retention_days_for("health"), 0);

        let loaded: Settings = settings_fixture("{}").unwrap();
        assert!(!loaded.life_enabled);
        assert!(!loaded.life_consent("general"));
        assert!(!loaded.life_consent("finance"));
        assert_eq!(loaded.retention_days_for("general"), 0);

        let custom_json = r#"{
            "lifeEnabled": true,
            "lifeConsents": { "health": true, "mind": false },
            "retentionDays": { "health": 30, "mind": 90 }
        }"#;
        let custom: Settings = settings_fixture(custom_json).unwrap();
        assert!(custom.life_enabled);
        assert!(custom.life_consent("health"));
        assert!(!custom.life_consent("mind"));
        assert_eq!(custom.retention_days_for("health"), 30);
        assert_eq!(custom.retention_days_for("mind"), 90);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VisualAnalysisModels {
    pub planner: String,
    pub entities: String,
    pub regions: String,
    pub structure: String,
    pub depth: String,
    pub instances: String,
    pub oriented: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VisualAnalysis {
    pub entities: bool,
    pub regions: bool,
    pub structure: bool,
    pub depth: bool,
}
impl Default for VisualAnalysis {
    fn default() -> Self {
        Self {
            entities: true,
            regions: true,
            structure: true,
            depth: true,
        }
    }
}
impl VisualAnalysis {
    pub fn shows(&self, group: crate::analysis::report::Group) -> bool {
        use crate::analysis::report::Group;
        match group {
            Group::Entities => self.entities,
            Group::Regions => self.regions,
            Group::Structure => self.structure,
            Group::Depth => self.depth,
        }
    }
}

#[cfg(test)]
mod visual_analysis_tests {
    use super::*;
    #[test]
    fn visual_analysis_models_default_empty_and_round_trip() {
        let mut settings: Settings = settings_fixture("{}").unwrap();
        assert_eq!(
            settings.visual_analysis_models,
            VisualAnalysisModels::default()
        );
        settings.visual_analysis_models.planner = "vision-model".into();
        settings.visual_analysis_models.entities = "detector".into();
        let restored: Settings =
            settings_fixture(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            restored.visual_analysis_models,
            settings.visual_analysis_models
        );
    }
    #[test]
    fn default_settings_show_all_groups_and_hidden_preferences_round_trip() {
        let mut settings: Settings = settings_fixture("{}").unwrap();
        assert_eq!(settings.visual_analysis, VisualAnalysis::default());
        settings.visual_analysis.depth = false;
        let saved = serde_json::to_string(&settings).unwrap();
        let restored: Settings = settings_fixture(&saved).unwrap();
        assert!(
            !restored
                .visual_analysis
                .shows(crate::analysis::report::Group::Depth)
        );
        assert!(
            restored
                .visual_analysis
                .shows(crate::analysis::report::Group::Entities)
        );
    }
}
