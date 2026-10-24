use std::str::FromStr;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::settings::Settings;

pub const DEFAULT_TOGGLE: &str = "Alt+Q";
pub const DEFAULT_STOP: &str = "Escape";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkeys {
    pub toggle: Shortcut,
    pub stop: Shortcut,
}

impl Hotkeys {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            toggle: parse_or_default(&settings.toggle_hotkey, DEFAULT_TOGGLE),
            stop: parse_or_default(&settings.stop_hotkey, DEFAULT_STOP),
        }
    }

    pub fn validate(settings: &Settings) -> Result<(), String> {
        for (label, text) in [
            ("Summon", &settings.toggle_hotkey),
            ("Stop", &settings.stop_hotkey),
        ] {
            Shortcut::from_str(text).map_err(|err| format!("{label} hotkey '{text}': {err}"))?;
        }
        if settings
            .toggle_hotkey
            .eq_ignore_ascii_case(&settings.stop_hotkey)
        {
            return Err("Summon and Stop hotkeys must differ".into());
        }
        Ok(())
    }
}

fn parse_or_default(text: &str, default: &str) -> Shortcut {
    Shortcut::from_str(text).unwrap_or_else(|err| {
        log::warn!("invalid hotkey '{text}' ({err}), using {default}");
        Shortcut::from_str(default).expect("default hotkey must parse")
    })
}

pub fn apply(app: &AppHandle, previous: Option<&Hotkeys>, next: &Hotkeys) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    if let Some(previous) = previous {
        for shortcut in [previous.toggle, previous.stop] {
            let _ = shortcuts.unregister(shortcut);
        }
    }

    let mut failures = Vec::new();
    for shortcut in [next.toggle, next.stop] {
        if let Err(err) = shortcuts.register(shortcut) {
            log::warn!("could not register {shortcut}: {err}");
            failures.push(format!("{shortcut} is unavailable ({err})"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}
