use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{error::AppResult, hotkeys};

/// The app identifier from tauri.conf.json: names the settings folder and the credential entries.
pub(crate) const KEYRING_SERVICE: &str = "dev.shirsendu-bairagi.rune-studio";
const KEYRING_USER: &str = "rune-infer-api-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub base_url: String,
    /// The one omni model behind every call: it sees the screen, hears the user, plans and acts.
    /// (Older settings files had separate vision, speech and voice models; those keys are ignored.)
    pub model: String,
    pub max_steps: u32,
    pub toggle_hotkey: String,
    pub stop_hotkey: String,
    /// Whether an outside agent may connect (only has an effect in `agent-bridge` builds).
    pub external_agent: bool,
    /// When `rune-studio-cli enable` last ran (Unix seconds). A newer value re-arms an agent that Stop
    /// locked out: running the command again is the user saying "yes, let it back in".
    pub external_agent_since: u64,
    /// Let the model think and show that thinking (slower: thinking is off by default).
    pub show_thinking: bool,
    /// Ask before every action, not only the risky ones.
    pub ask_every_action: bool,
    /// Optional helper, off by default: the decision model takes quick checks (skip planning on simple tasks, spot a finished
    /// task, a second opinion on risk). Everything still works without it, only slower.
    pub fast_checks: bool,
    pub decision_model: String,
    /// Where the decision model is served (it is a rune-infer endpoint, not part of the omni
    /// model's server). Empty means unavailable.
    pub decision_url: String,
    /// Delete session folders (screenshots, recordings) older than this many days, and their rows in
    /// the trace database, at startup. 0 keeps everything.
    pub archive_keep_days: u32,
    /// What happens with the independent check of a finished task: `off`, `background` (the default:
    /// the user sees Done at once and the check runs afterwards) or `blocking` (the agent gets one more
    /// chance when the check contradicts it). No UI yet: it is edited in `settings.json`.
    pub verify_mode: VerifyMode,
    /// Draw the numbered boxes on the screen analysis picture. Off by default: the chat model's boxes
    /// were right 1 time in 8 in the field test, so the findings list and its evidence are shown alone.
    pub analysis_boxes: bool,
}

/// What happens with the independent check of a finished task (`agent::verify`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifyMode {
    /// Nothing runs.
    Off,
    /// Before the agent finishes a task it claims succeeded, the check runs; a contradiction gives
    /// the agent one more chance. Adds a model call to the task's time.
    Blocking,
    /// The user sees "Done" at once and the check runs afterwards. It never changes the outcome.
    /// Also what any value this build does not know reads as.
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
            fast_checks: false,
            decision_model: "liquid-d1-3b".into(),
            decision_url: String::new(),
            archive_keep_days: 0,
            verify_mode: VerifyMode::default(),
            analysis_boxes: false,
        }
    }
}

/// The settings file, found without a running app (for the command line).
#[cfg(feature = "agent-bridge")]
pub fn path_without_app() -> Option<PathBuf> {
    Some(
        PathBuf::from(std::env::var_os("APPDATA")?)
            .join(KEYRING_SERVICE)
            .join("settings.json"),
    )
}

/// Reads settings from a file; missing or unreadable means defaults.
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

/// Missing or unreadable settings fall back to defaults so a bad file never blocks startup.
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

/// The credential store wins; `RUNE_API_KEY` (from `.env`) is a dev-only fallback.
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
    fn analysis_boxes_are_off_by_default_and_a_file_without_the_setting_loads() {
        assert!(!Settings::default().analysis_boxes);
        let loaded: Settings = serde_json::from_str(r#"{ "maxSteps": 7 }"#).unwrap();
        assert!(!loaded.analysis_boxes);
        let on: Settings = serde_json::from_str(r#"{ "analysisBoxes": true }"#).unwrap();
        assert!(on.analysis_boxes);
        assert_eq!(
            serde_json::to_value(Settings::default()).unwrap()["analysisBoxes"],
            false
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
        // A typo must not throw the rest of the settings away.
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
