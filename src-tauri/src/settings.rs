use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{error::AppResult, hotkeys};

pub(crate) const KEYRING_SERVICE: &str = "dev.shirsendu-bairagi.rune-studio";
const KEYRING_USER: &str = "rune-infer-api-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub base_url: String,
    pub model: String,
    pub max_steps: u32,
    pub toggle_hotkey: String,
    pub stop_hotkey: String,
    pub external_agent: bool,
    pub external_agent_since: u64,
    pub show_thinking: bool,
    pub ask_every_action: bool,
    pub developer_mode: bool,
    pub fast_checks: bool,
    pub decision_model: String,
    pub decision_url: String,
    pub archive_keep_days: u32,
    pub verify_mode: VerifyMode,
    pub record_session_video: RecordVideo,
    pub offer_review: OfferReview,
    pub demo_mode: bool,
    pub reflection_enabled: bool,
    pub idle_minutes: u32,
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
        Self {
            base_url: "http://127.0.0.1:8080".into(),
            model: "gemma-4-12b-it".into(),
            max_steps: 30,
            toggle_hotkey: hotkeys::DEFAULT_TOGGLE.into(),
            stop_hotkey: hotkeys::DEFAULT_STOP.into(),
            external_agent: false,
            external_agent_since: 0,
            show_thinking: false,
            ask_every_action: false,
            developer_mode: false,
            fast_checks: false,
            decision_model: "liquid-d1-3b".into(),
            decision_url: String::new(),
            archive_keep_days: 0,
            verify_mode: VerifyMode::default(),
            record_session_video: RecordVideo::default(),
            offer_review: OfferReview::default(),
            demo_mode: true,
            reflection_enabled: true,
            idle_minutes: 10,
        }
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

fn key_entry() -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
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
    use super::*;

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
    fn demo_mode_is_on_by_default_and_a_file_without_it_loads() {
        assert!(Settings::default().demo_mode);
        assert!(serde_json::from_str::<Settings>("{}").unwrap().demo_mode);
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
}
