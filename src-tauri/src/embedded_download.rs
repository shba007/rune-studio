//! A single, user-started local model installation backed by Infer's registry.
use crate::sync::LockExt;
use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    id: String,
    name: String,
    license: String,
    bytes: u64,
    available_bytes: Option<u64>,
    revision: String,
    manifest: String,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    phase: String,
    model: Option<String>,
    filename: Option<String>,
    downloaded: u64,
    total: Option<u64>,
    error: Option<String>,
}

#[derive(Default)]
pub struct Downloads {
    state: Arc<Mutex<Status>>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
}

impl Downloads {
    pub fn status(&self) -> Status {
        self.state.locked().clone()
    }
    pub fn completed_model(&self) -> Option<String> {
        let state = self.state.locked();
        (state.phase == "complete")
            .then(|| state.model.clone())
            .flatten()
    }
    pub fn cancel(&self) {
        if let Some(cancel) = self.cancel.locked().as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn start(
        &self,
        model_id: String,
        revision: String,
        manifest: String,
    ) -> Result<(), String> {
        if !matches!(model_id.as_str(), "qwen3.8-27b" | "qwen3.5-4b-q4") {
            return Err("Choose one of the available computer-use models".into());
        }
        let mut cancel_slot = self.cancel.locked();
        if cancel_slot.is_some() {
            return Err("A model download is already in progress".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *cancel_slot = Some(cancel.clone());
        *self.state.locked() = Status {
            phase: "preparing".into(),
            model: Some(model_id.clone()),
            ..Status::default()
        };
        let state = self.state.clone();
        // The marker is cleared by the completion command once the worker reports a terminal state.
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                install(
                    &model_id,
                    &revision,
                    &manifest,
                    cancel.clone(),
                    state.clone(),
                )
            }))
            .unwrap_or_else(|_| Err("Model installer stopped unexpectedly".into()));
            let mut status = state.locked();
            status.phase = if result.is_ok() {
                "complete"
            } else if cancel.load(Ordering::Relaxed) {
                "cancelled"
            } else {
                "failed"
            }
            .into();
            status.error = result.err();
        });
        Ok(())
    }
    pub fn acknowledge(&self) {
        let phase = self.state.locked().phase.clone();
        if matches!(phase.as_str(), "complete" | "cancelled" | "failed") {
            self.cancel.locked().take();
        }
    }
}

pub fn candidates() -> Result<Vec<Candidate>, String> {
    let models = rune_infer::model_registry::bundled()
        .map_err(|error| format!("Cannot load model list: {error:#}"))?;
    let available = available_disk_bytes();
    Ok(models
        .into_iter()
        .filter(|model| matches!(model.id.as_str(), "qwen3.8-27b" | "qwen3.5-4b-q4"))
        .filter_map(|model| {
            model.versions.get(&model.latest).and_then(|version| {
                Some(Candidate {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    license: model.license.clone(),
                    bytes: version
                        .files
                        .iter()
                        .try_fold(0u64, |total, file| total.checked_add(file.size_bytes))?,
                    available_bytes: available,
                    revision: model.latest.clone(),
                    manifest: manifest_for(version),
                })
            })
        })
        .collect())
}

fn available_disk_bytes() -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::core::PCWSTR;
    if std::env::var_os("RUNE_HOME")
        .filter(|value| !value.is_empty())
        .is_some_and(|value| !std::path::Path::new(&value).is_absolute())
    {
        return None;
    }
    let mut path = rune_infer::home::rune_home().ok()?;
    while !path.exists() {
        path = path.parent()?.to_path_buf();
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    unsafe {
        GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut available), None, None).ok()?;
    }
    Some(available)
}

fn manifest_for(version: &rune_infer::model_registry::Version) -> String {
    version
        .files
        .iter()
        .map(|file| {
            format!(
                "{}:{}:{}:{}:{}",
                file.id, file.filename, file.url, file.sha256, file.size_bytes
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn install(
    id: &str,
    revision: &str,
    manifest: &str,
    cancel: Arc<AtomicBool>,
    state: Arc<Mutex<Status>>,
) -> Result<(), String> {
    if std::env::var_os("RUNE_HOME")
        .filter(|value| !value.is_empty())
        .is_some_and(|value| !std::path::Path::new(&value).is_absolute())
    {
        return Err("RUNE_HOME must be an absolute path".into());
    }
    let root = rune_infer::home::rune_home().map_err(|error| error.to_string())?;
    let model = rune_infer::model_registry::bundled()
        .map_err(|error| format!("Cannot load model list: {error:#}"))?
        .into_iter()
        .find(|model| model.id == id)
        .ok_or("Selected model is no longer available")?;
    if model.latest != revision
        || model
            .versions
            .get(revision)
            .is_none_or(|version| manifest_for(version) != manifest)
    {
        return Err(
            "The model changed since you reviewed it. Reload the list and review it again.".into(),
        );
    }
    let report = |state: Arc<Mutex<Status>>| {
        move |progress: rune_infer::inference::engines::download::DownloadProgress| {
            let mut status = state.locked();
            status.filename = Some(progress.filename);
            status.downloaded = progress.downloaded;
            status.total = progress.total;
        }
    };
    state.locked().phase = "downloading".into();
    rune_infer::model_registry::install_observed(
        &model,
        &root,
        &root.join("config/server.json"),
        &root.join("config/catalog.json"),
        cancel.clone(),
        report(state.clone()),
    )
    .map_err(|error| format!("Model installation failed: {error:#}"))?;
    if cancel.load(Ordering::Relaxed) {
        return Err("Installation cancelled".into());
    }
    {
        let mut status = state.locked();
        status.phase = "engine".into();
        status.filename = None;
        status.downloaded = 0;
        status.total = None;
    }
    rune_infer::embedding::install_llama_observed(&root, cancel, report(state))
        .map_err(|error| format!("Engine preparation failed: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_registry_offers_the_reviewed_local_candidates() {
        let models = candidates().expect("the bundled Infer registry must be valid");
        assert!(models.iter().any(|model| model.id == "qwen3.8-27b"));
        assert!(models.iter().any(|model| model.id == "qwen3.5-4b-q4"));
    }

    #[test]
    fn refuses_unreviewed_model_before_starting_worker() {
        let downloads = Downloads::default();
        assert!(
            downloads
                .start("unreviewed".into(), "v1".into(), "manifest".into())
                .is_err()
        );
        assert!(downloads.cancel.locked().is_none());
        assert!(downloads.status().phase.is_empty());
    }

    #[test]
    fn only_a_completed_download_can_be_selected() {
        let downloads = Downloads::default();
        {
            let mut status = downloads.state.locked();
            status.phase = "downloading".into();
            status.model = Some("qwen3.8-27b".into());
        }
        assert!(downloads.completed_model().is_none());
        downloads.state.locked().phase = "complete".into();
        assert_eq!(downloads.completed_model().as_deref(), Some("qwen3.8-27b"));
        downloads.state.locked().phase = "cancelled".into();
        assert!(downloads.completed_model().is_none());
    }

    #[test]
    fn reviewed_manifest_changes_when_download_content_or_source_changes() {
        let mut version = rune_infer::model_registry::Version {
            release_number: 1,
            files: vec![rune_infer::model_registry::File {
                id: "model".into(),
                filename: "model.gguf".into(),
                url: "https://example.org/model".into(),
                sha256: "a".repeat(64),
                size_bytes: 42,
            }],
        };
        let reviewed = manifest_for(&version);
        version.files[0].sha256 = "b".repeat(64);
        assert_ne!(reviewed, manifest_for(&version));
        version.files[0].sha256 = "a".repeat(64);
        version.files[0].url = "https://example.org/other".into();
        assert_ne!(reviewed, manifest_for(&version));
    }
}
