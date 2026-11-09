pub use rune_infer::model_management::Downloads;

fn models(
    memory: Option<u64>,
    gpu: bool,
    disk: Option<u64>,
) -> Result<Vec<rune_infer::model_management::Candidate>, String> {
    let ids = [
        "qwen3.8-27b",
        "qwen3.5-4b-q4",
        "holo3.1-4b-q4",
        "holo3.1-9b-q4",
        "holo4-27b-q4",
    ]
    .map(str::to_owned);
    let holo_ids: Vec<_> = ids
        .iter()
        .filter(|id| id.starts_with("holo"))
        .cloned()
        .collect();
    let mut models = rune_infer::model_management::candidates(&ids, memory, gpu, disk)
        .map_err(|error| format!("Cannot load model list: {error:#}"))?;
    let recommendations = rune_infer::model_management::candidates(&holo_ids, memory, gpu, disk)
        .map_err(|error| format!("Cannot load model recommendations: {error:#}"))?;
    for model in &mut models {
        model.recommended = recommendations
            .iter()
            .any(|candidate| candidate.id == model.id && candidate.recommended);
    }
    Ok(models)
}

pub fn candidates() -> Result<Vec<rune_infer::model_management::Candidate>, String> {
    let root = rune_infer::home::rune_home().map_err(|error| error.to_string())?;
    if !root.is_absolute() {
        return Err("RUNE_HOME must be an absolute path".into());
    }
    let (memory, gpu) = rune_infer::model_management::detected_memory();
    let disk = rune_infer::model_management::available_disk(&root);
    models(memory, gpu, disk)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub id: String,
    pub name: String,
    pub bytes: u64,
    pub path: String,
}

type Listed = (String, u64, std::path::PathBuf);

fn find_files(dir: &std::path::Path) -> Vec<Listed> {
    let mut found = Vec::new();
    let mut pending = vec![(dir.to_path_buf(), 0u8)];
    let mut visited = 0usize;
    while let Some((folder, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            visited += 1;
            if visited > 50_000 {
                return found;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if depth < 5 {
                    pending.push((entry.path(), depth + 1));
                }
            } else if kind.is_file()
                && let Ok(meta) = entry.metadata()
            {
                found.push((
                    entry.file_name().to_string_lossy().to_ascii_lowercase(),
                    meta.len(),
                    entry.path(),
                ));
            }
        }
    }
    found
}

fn registry_model(id: &str) -> Result<rune_infer::model_registry::LocalModel, String> {
    rune_infer::model_registry::bundled()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|model| model.id == id)
        .ok_or_else(|| "Model is not available".to_string())
}

type Matched = Vec<(rune_infer::model_registry::File, std::path::PathBuf)>;

fn matches_in(model: &rune_infer::model_registry::LocalModel, files: &[Listed]) -> Option<Matched> {
    let version = model.versions.get(&model.latest)?;
    version
        .files
        .iter()
        .map(|file| {
            files
                .iter()
                .find(|(name, size, _)| {
                    *name == file.filename.to_ascii_lowercase() && *size == file.size_bytes
                })
                .map(|(_, _, path)| (file.clone(), path.clone()))
        })
        .collect()
}

pub fn scan(dir: &std::path::Path) -> Result<Vec<Found>, String> {
    if !dir.is_absolute() || !dir.is_dir() {
        return Err("Choose an existing folder".into());
    }
    let files = find_files(dir);
    let mut found = Vec::new();
    for candidate in candidates()? {
        let model = registry_model(&candidate.id)?;
        if let Some(matched) = matches_in(&model, &files)
            && let Some((_, path)) = matched.iter().find(|(file, _)| file.id == "model")
        {
            found.push(Found {
                id: candidate.id,
                name: candidate.name,
                bytes: candidate.bytes,
                path: path.display().to_string(),
            });
        }
    }
    Ok(found)
}

pub fn stage(root: &std::path::Path, id: &str, dir: &std::path::Path) -> Result<(), String> {
    let model = registry_model(id)?;
    let matched = matches_in(&model, &find_files(dir))
        .ok_or("This folder no longer contains the complete model files")?;
    let first = matched
        .first()
        .ok_or("Model has no files")?
        .0
        .filename
        .clone();
    let relative = model
        .catalog
        .as_ref()
        .and_then(|catalog| catalog.local.path.clone())
        .or_else(|| {
            model
                .install_dir
                .as_ref()
                .map(|install| format!("{install}/{first}"))
        })
        .ok_or("Model has no install location")?;
    let mut target = root.join("models");
    let parts: Vec<_> = relative.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if part.is_empty() || *part == "." || *part == ".." {
            return Err("Invalid model install location".into());
        }
        target.push(part);
    }
    std::fs::create_dir_all(&target).map_err(|error| error.to_string())?;
    for (file, source) in matched {
        if target.join(&file.filename).exists() {
            continue;
        }
        let hash = rune_infer::inference::engines::download::file_sha256(&source)
            .map_err(|error| error.to_string())?;
        if !hash.eq_ignore_ascii_case(&file.sha256) {
            return Err(format!(
                "{} does not match the reviewed model file",
                file.filename
            ));
        }
        let staged = target.join(format!(
            ".rune-model-download-{}-{}.part",
            file.id,
            file.sha256.to_ascii_lowercase()
        ));
        if staged.exists() {
            std::fs::remove_file(&staged).map_err(|error| error.to_string())?;
        }
        if std::fs::hard_link(&source, &staged).is_err() {
            std::fs::copy(&source, &staged).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn recommends_a_holo_model_even_when_qwen_also_fits() {
        let models = super::models(Some(32 * 1024 * 1024 * 1024), true, None).unwrap();
        let selected: Vec<_> = models.iter().filter(|model| model.recommended).collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "holo4-27b-q4");
    }

    #[test]
    fn offers_only_the_supported_local_candidates() {
        let models = super::models(Some(8 * 1024 * 1024 * 1024), true, None).unwrap();
        let mut ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            [
                "holo3.1-4b-q4",
                "holo3.1-9b-q4",
                "holo4-27b-q4",
                "qwen3.5-4b-q4",
                "qwen3.8-27b"
            ]
        );
        assert_eq!(
            models.iter().find(|model| model.recommended).unwrap().id,
            "holo3.1-4b-q4"
        );
    }
}
