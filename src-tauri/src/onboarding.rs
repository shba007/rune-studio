//! First-run onboarding state, separate from the user's connection settings.
use std::io::Write;
use tauri::{AppHandle, Manager};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
const WINDOW: &str = "onboarding";
const MARKER: &[u8] = b"Rune Studio onboarding v1\n";

#[tauri::command]
pub fn onboarding_platform() -> &'static str {
    std::env::consts::OS
}

fn marker_exists(path: &std::path::Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err("Onboarding marker conflicts with an existing path".into())
        }
        Ok(_) => {
            if std::fs::read(path).map_err(|error| error.to_string())? != MARKER {
                return Err("Onboarding marker conflicts with existing data".into());
            }
            Ok(true)
        }
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn prepare(app: &AppHandle) -> Result<bool, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    prepare_in(&dir)
}

#[cfg(any(not(any(target_os = "android", target_os = "ios")), test))]
fn prepare_in(dir: &std::path::Path) -> Result<bool, String> {
    let pending = dir.join("onboarding.pending");
    let complete = dir.join("onboarding.complete");
    if marker_exists(&complete)? {
        return Ok(false);
    }
    if marker_exists(&pending)? {
        return Ok(true);
    }
    if dir.join("settings.json").exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
    {
        Ok(mut file) => file.write_all(MARKER).map_err(|error| error.to_string())?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            marker_exists(&pending)?;
        }
        Err(error) => return Err(error.to_string()),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_setup_resumes_and_existing_users_are_not_interrupted() {
        let root = tempfile::tempdir().unwrap();
        let fresh = root.path().join("fresh");
        assert!(prepare_in(&fresh).unwrap());
        std::fs::write(fresh.join("settings.json"), b"{}").unwrap();
        assert!(prepare_in(&fresh).unwrap());
        std::fs::write(fresh.join("onboarding.complete"), MARKER).unwrap();
        assert!(!prepare_in(&fresh).unwrap());
        let existing = root.path().join("existing");
        std::fs::create_dir_all(&existing).unwrap();
        std::fs::write(existing.join("settings.json"), b"{}").unwrap();
        assert!(!prepare_in(&existing).unwrap());
        let conflict = root.path().join("conflict");
        std::fs::create_dir_all(&conflict).unwrap();
        std::fs::write(conflict.join("onboarding.pending"), b"keep").unwrap();
        assert!(prepare_in(&conflict).is_err());
        assert_eq!(
            std::fs::read(conflict.join("onboarding.pending")).unwrap(),
            b"keep"
        );
    }

    #[test]
    fn completion_can_be_retried_without_overwriting_conflicting_data() {
        let root = tempfile::tempdir().unwrap();
        assert!(finish_in(root.path()).is_err());
        assert!(prepare_in(root.path()).unwrap());
        finish_in(root.path()).unwrap();
        finish_in(root.path()).unwrap();
        std::fs::write(root.path().join("onboarding.complete"), b"keep").unwrap();
        assert!(finish_in(root.path()).is_err());
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW) {
        window.show()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(
        app,
        WINDOW,
        WebviewUrl::App("index.html?view=onboarding".into()),
    )
    .title("Welcome to Rune Studio")
    .inner_size(1100.0, 700.0)
    .min_inner_size(760.0, 520.0)
    .resizable(true)
    .decorations(false)
    .center()
    .build()?;
    crate::window::protect(&window)?;
    window.set_focus()
}

#[tauri::command]
pub fn onboarding_complete(app: AppHandle) -> Result<(), String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    finish_in(&dir)?;
    crate::window::summon(&app);
    Ok(())
}

fn finish_in(dir: &std::path::Path) -> Result<(), String> {
    let pending = dir.join("onboarding.pending");
    let complete = dir.join("onboarding.complete");
    if !marker_exists(&pending)? {
        if marker_exists(&complete)? {
            return Ok(());
        }
        return Err("Onboarding is not active".into());
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&complete)
    {
        Ok(mut file) => file.write_all(MARKER).map_err(|error| error.to_string())?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            marker_exists(&complete)?;
        }
        Err(error) => return Err(error.to_string()),
    }
    std::fs::remove_file(pending).map_err(|error| error.to_string())?;
    Ok(())
}
