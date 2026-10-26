use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{error::AppResult, hotkeys, model::role::ModelRole};

pub(crate) const KEYRING_SERVICE: &str = "dev.shirsendu-bairagi.rune-studio";
const KEYRING_USER: &str = "rune-infer-api-key";

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
    "gemma-4-12b-it".into()
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
                    "gemma-4-12b-it"
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
    pub model: Option<String>,
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
    pub decision_model: Option<String>,
    pub decision_url: String,
    pub archive_keep_days: u32,
    pub verify_mode: VerifyMode,
    pub record_session_video: RecordVideo,
    pub model_image_resolution: ImageResolution,
    pub recording_resolution: ImageResolution,
    pub visual_analysis: VisualAnalysis,
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
            model: None,
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
            decision_model: None,
            decision_url: def.decision_url,
            archive_keep_days: def.archive_keep_days,
            verify_mode: def.verify_mode,
            record_session_video: def.record_session_video,
            model_image_resolution: def.model_image_resolution,
            recording_resolution: def.recording_resolution,
            visual_analysis: def.visual_analysis,
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
        let mut models = helper.models.unwrap_or_default();
        if let Some(ref m) = helper.model
            && !m.is_empty()
            && (models.agent.is_empty() || models.agent == "gemma-4-12b-it")
        {
            models.agent = m.clone();
        }
        if let Some(ref d) = helper.decision_model
            && !d.is_empty()
            && (models.decision.is_empty() || models.decision == "liquid-d1-3b")
        {
            models.decision = d.clone();
        }
        let synced_model = models.agent.clone();
        let synced_decision = models.decision.clone();
        Ok(Settings {
            base_url: helper.base_url,
            model: synced_model,
            models,
            max_steps: helper.max_steps,
            toggle_hotkey: helper.toggle_hotkey,
            stop_hotkey: if matches!(
                helper.stop_hotkey.to_ascii_lowercase().as_str(),
                "ctrl+alt+escape" | "ctrl+alt+esc"
            ) {
                hotkeys::DEFAULT_STOP.into()
            } else {
                helper.stop_hotkey
            },
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
    pub fn model_for_role(&self, role: ModelRole) -> &str {
        self.models.model_for_role(role)
    }
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
    settings_path(app)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> AppResult<()> {
    let path = settings_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

fn key_entry() -> keyring::Result<crate::credentials::Entry> {
    crate::credentials::Entry::new(KEYRING_SERVICE, KEYRING_USER)
}

pub fn api_key() -> Option<String> {
    key_entry()
        .and_then(|entry| entry.get_password())
        .ok()
        .or_else(|| std::env::var("RUNE_API_KEY").ok())
        .filter(|key| !key.is_empty())
}

pub fn set_api_key(key: &str) -> AppResult<()> {
    let entry = key_entry()?;
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
mod tests {
    #[test]
    fn classification_preference_defaults_and_round_trips() {
        use super::{InputClassification, Settings};
        for json in ["{}", r#"{"inputClassification":"unknown"}"#] {
            let settings: Settings = serde_json::from_str(json).unwrap();
            assert_eq!(
                settings.input_classification,
                InputClassification::Automatic
            );
        }
        let settings: Settings =
            serde_json::from_str(r#"{"inputClassification":"always_confirm"}"#).unwrap();
        let encoded = serde_json::to_value(&settings).unwrap();
        assert_eq!(encoded["inputClassification"], "always_confirm");
    }
    use super::*;

    #[test]
    fn capture_resolution_defaults_migrate_and_separate_settings_round_trip() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.model_image_resolution.max_edge(), 1920);
        assert_eq!(old.recording_resolution, ImageResolution::Native);
        for (name, edge) in [
            ("720", 720),
            ("1280", 1280),
            ("1920", 1920),
            ("2560", 2560),
            ("native", u32::MAX),
        ] {
            let settings: Settings = serde_json::from_value(serde_json::json!({
                "modelImageResolution": name, "recordingResolution": "720"
            }))
            .unwrap();
            assert_eq!(settings.model_image_resolution.max_edge(), edge);
            assert_eq!(settings.recording_resolution.max_edge(), 720);
            let saved = serde_json::to_value(settings).unwrap();
            assert_eq!(saved["modelImageResolution"], name);
            assert_eq!(saved["recordingResolution"], "720");
        }
        let invalid: Settings =
            serde_json::from_str(r#"{"modelImageResolution":"invalid"}"#).unwrap();
        assert_eq!(invalid.model_image_resolution.max_edge(), 1920);
    }

    #[test]
    fn old_stop_shortcut_migrates_to_escape_and_custom_shortcuts_survive() {
        for old in ["Ctrl+Alt+Escape", "Ctrl+Alt+Esc"] {
            let settings: Settings =
                serde_json::from_value(serde_json::json!({"stopHotkey":old})).unwrap();
            assert_eq!(settings.stop_hotkey, "Escape");
        }
        let settings: Settings =
            serde_json::from_value(serde_json::json!({"stopHotkey":"Ctrl+Shift+S"})).unwrap();
        assert_eq!(settings.stop_hotkey, "Ctrl+Shift+S");
    }

    #[test]
    fn migration_from_old_settings_with_model_and_decision_model() {
        let old_fixture = r#"{
            "baseUrl": "http://127.0.0.1:8080",
            "model": "my-custom-agent",
            "decisionModel": "my-custom-decision",
            "maxSteps": 25,
            "fastChecks": true
        }"#;

        let settings: Settings = serde_json::from_str(old_fixture).unwrap();
        assert_eq!(settings.models.agent, "my-custom-agent");
        assert_eq!(settings.models.decision, "my-custom-decision");
        assert_eq!(settings.model, "my-custom-agent");
        assert_eq!(settings.decision_model, "my-custom-decision");
        assert_eq!(settings.model_for_role(ModelRole::Agent), "my-custom-agent");
        assert_eq!(
            settings.model_for_role(ModelRole::Decision),
            "my-custom-decision"
        );
        assert_eq!(
            settings.model_for_role(ModelRole::Verifier),
            "my-custom-agent"
        );
        assert_eq!(
            settings.model_for_role(ModelRole::Analysis),
            "my-custom-agent"
        );
        assert_eq!(settings.model_for_role(ModelRole::Voice), "my-custom-agent");

        let serialized = serde_json::to_string(&settings).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(json_val["model"], "my-custom-agent");
        assert_eq!(json_val["decisionModel"], "my-custom-decision");
        assert_eq!(json_val["models"]["agent"], "my-custom-agent");
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

        let settings: Settings = serde_json::from_str(new_fixture).unwrap();
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
            serde_json::from_str(r#"{ "reflectionEnabled": false, "idleMinutes": 3 }"#).unwrap();
        assert!(!loaded.reflection_enabled);
        assert_eq!(loaded.idle_minutes, 3);
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap().idle_minutes,
            10
        );
    }

    #[test]
    fn demo_mode_is_off_by_default_and_saved_preferences_are_preserved() {
        assert!(!Settings::default().demo_mode);
        assert!(!serde_json::from_str::<Settings>("{}").unwrap().demo_mode);
        let saved = serde_json::from_str::<Settings>(
            r#"{"demoMode":true,"modelImageResolution":"720","recordingResolution":"1280"}"#,
        )
        .unwrap();
        assert!(saved.demo_mode);
        assert_eq!(saved.model_image_resolution, ImageResolution::Small);
        assert_eq!(saved.recording_resolution, ImageResolution::Balanced);
        assert!(
            !serde_json::from_str::<Settings>(r#"{ "demoMode": false }"#)
                .unwrap()
                .demo_mode
        );
    }

    #[test]
    fn an_old_file_with_analysis_boxes_still_loads_and_the_key_is_ignored() {
        let loaded: Settings =
            serde_json::from_str(r#"{ "analysisBoxes": true, "maxSteps": 7 }"#).unwrap();
        assert_eq!(loaded.max_steps, 7);
        let value = serde_json::to_value(Settings::default()).unwrap();
        assert!(
            value.get("analysisBoxes").is_none(),
            "it is no longer written back"
        );
    }

    #[test]
    fn the_video_and_review_settings_default_to_off_and_always_and_unknown_words_read_as_those() {
        let defaults = Settings::default();
        assert_eq!(
            (defaults.record_session_video, defaults.offer_review),
            (RecordVideo::Off, OfferReview::Always)
        );
        let set: Settings = serde_json::from_str(
            r#"{ "recordSessionVideo": "motion", "offerReview": "failures" }"#,
        )
        .unwrap();
        assert_eq!(
            (set.record_session_video, set.offer_review),
            (RecordVideo::Motion, OfferReview::Failures)
        );
        let never: Settings = serde_json::from_str(r#"{ "offerReview": "never" }"#).unwrap();
        assert_eq!(never.offer_review, OfferReview::Never);
        let typo: Settings = serde_json::from_str(
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
    fn a_settings_file_without_verify_mode_still_loads_with_background() {
        let old = r#"{ "baseUrl": "http://127.0.0.1:9", "model": "m", "maxSteps": 12, "askEveryAction": true }"#;
        let settings: Settings = serde_json::from_str(old).expect("an older settings file loads");
        assert_eq!(settings.verify_mode, VerifyMode::Background);
        assert_eq!(
            (
                settings.base_url.as_str(),
                settings.max_steps,
                settings.ask_every_action
            ),
            ("http://127.0.0.1:9", 12, true),
            "the rest of the file is kept"
        );
        assert_eq!(
            serde_json::from_str::<Settings>("{}").unwrap().verify_mode,
            VerifyMode::Background
        );
    }

    #[test]
    fn the_three_modes_are_spelled_in_lower_case_and_an_unknown_word_reads_as_background() {
        for (text, mode) in [
            ("off", VerifyMode::Off),
            ("background", VerifyMode::Background),
            ("blocking", VerifyMode::Blocking),
        ] {
            let settings: Settings =
                serde_json::from_str(&format!(r#"{{ "verifyMode": "{text}" }}"#)).unwrap();
            assert_eq!(settings.verify_mode, mode, "{text}");
            assert_eq!(serde_json::to_value(mode).unwrap(), text);
        }
        let typo: Settings = serde_json::from_str(r#"{ "verifyMode": "blokcing", "maxSteps": 7 }"#)
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
    fn life_settings_defaults_and_backward_compatibility() {
        let defaults = Settings::default();
        assert!(!defaults.life_enabled);
        assert!(!defaults.life_consent("health"));
        assert_eq!(defaults.retention_days_for("health"), 0);

        let old_json = r#"{ "baseUrl": "http://127.0.0.1:8080", "model": "gemma" }"#;
        let loaded: Settings = serde_json::from_str(old_json).unwrap();
        assert!(!loaded.life_enabled);
        assert!(!loaded.life_consent("general"));
        assert!(!loaded.life_consent("finance"));
        assert_eq!(loaded.retention_days_for("general"), 0);

        let custom_json = r#"{
            "lifeEnabled": true,
            "lifeConsents": { "health": true, "mind": false },
            "retentionDays": { "health": 30, "mind": 90 }
        }"#;
        let custom: Settings = serde_json::from_str(custom_json).unwrap();
        assert!(custom.life_enabled);
        assert!(custom.life_consent("health"));
        assert!(!custom.life_consent("mind"));
        assert_eq!(custom.retention_days_for("health"), 30);
        assert_eq!(custom.retention_days_for("mind"), 90);
    }
}

/// Display preferences only: never remove findings from requests or history.
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
    fn old_settings_show_all_groups_and_hidden_preferences_round_trip() {
        let mut settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.visual_analysis, VisualAnalysis::default());
        settings.visual_analysis.depth = false;
        let saved = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&saved).unwrap();
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
