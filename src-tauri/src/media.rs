use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItemPayload {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
    pub media_type: String,
    pub last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileActionRequest {
    pub action_type: String, // "MOVE" | "COPY"
    pub source_path: String,
    pub target_directory: String,
    pub target_file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResultPayload {
    pub success_count: usize,
    pub errors: Vec<String>,
}

fn determine_media_type(ext: &str) -> Option<(&'static str, &'static str)> {
    match ext {
        "jpg" | "jpeg" => Some(("image", "image/jpeg")),
        "png" => Some(("image", "image/png")),
        "webp" => Some(("image", "image/webp")),
        "gif" => Some(("image", "image/gif")),
        "bmp" => Some(("image", "image/bmp")),
        "avif" => Some(("image", "image/avif")),
        "mp4" => Some(("video", "video/mp4")),
        "webm" => Some(("video", "video/webm")),
        "mov" => Some(("video", "video/quicktime")),
        "mkv" => Some(("video", "video/x-matroska")),
        _ => None,
    }
}

/// Opens native folder picker dialog
#[tauri::command]
pub async fn pick_directory() -> Result<Option<String>, String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Select Media Folder")
        .pick_folder()
        .await;

    Ok(folder.map(|handle| handle.path().to_string_lossy().to_string()))
}

/// Recursively scans directory for image and video files
#[tauri::command]
pub async fn scan_directory(path: String, recursive: bool) -> Result<Vec<MediaItemPayload>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_path = Path::new(&path);
        if !base_path.exists() || !base_path.is_dir() {
            return Err(format!("Invalid directory path: {}", path));
        }

        let mut results = Vec::new();
        let max_depth = if recursive { usize::MAX } else { 1 };

        let walker = WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter();

        for entry in walker.filter_map(|e| e.ok()) {
            let path_buf = entry.path();
            if !path_buf.is_file() {
                continue;
            }

            let ext = path_buf
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if let Some((media_type, mime_type)) = determine_media_type(&ext) {
                let metadata = match fs::metadata(path_buf) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let last_modified = metadata
                    .modified()
                    .unwrap_or(UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let file_name = path_buf
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let full_path = path_buf.to_string_lossy().to_string();

                results.push(MediaItemPayload {
                    id: format!("{}:{}", full_path, metadata.len()),
                    name: file_name,
                    path: full_path,
                    size: metadata.len(),
                    mime_type: mime_type.to_string(),
                    media_type: media_type.to_string(),
                    last_modified,
                });
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("Scanner thread failed: {}", e))?
}

/// Reads file and returns standard base64 data URL
#[tauri::command]
pub async fn get_media_base64(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let file_path = Path::new(&path);
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let (_, mime_type) = determine_media_type(&ext)
            .ok_or_else(|| format!("Unsupported media type for file: {}", path))?;

        let bytes = fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
        let encoded = BASE64.encode(&bytes);

        Ok(format!("data:{};base64,{}", mime_type, encoded))
    })
    .await
    .map_err(|e| format!("Base64 thread failed: {}", e))?
}

/// Executes safe non-destructive file operations
#[tauri::command]
pub async fn execute_file_actions(
    actions: Vec<FileActionRequest>,
) -> Result<ActionResultPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut success_count = 0;
        let mut errors = Vec::new();

        for action in actions {
            let src = PathBuf::from(&action.source_path);
            if !src.exists() {
                errors.push(format!("Source file does not exist: {}", action.source_path));
                continue;
            }

            let dest_dir = PathBuf::from(&action.target_directory);
            if let Err(e) = fs::create_dir_all(&dest_dir) {
                errors.push(format!(
                    "Failed to create target folder {}: {}",
                    action.target_directory, e
                ));
                continue;
            }

            let file_name = action
                .target_file_name
                .as_deref()
                .or_else(|| src.file_name().and_then(|s| s.to_str()))
                .unwrap_or("file");

            let mut dest_file = dest_dir.join(file_name);

            // Avoid accidental overwrites by appending a counter
            if dest_file.exists() {
                let stem = Path::new(file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");
                let ext = Path::new(file_name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                let mut counter = 1;
                while dest_file.exists() {
                    let new_name = if ext.is_empty() {
                        format!("{}_{}", stem, counter)
                    } else {
                        format!("{}_{}.{}", stem, counter, ext)
                    };
                    dest_file = dest_dir.join(new_name);
                    counter += 1;
                }
            }

            let result = match action.action_type.as_str() {
                "MOVE" => fs::rename(&src, &dest_file).or_else(|_| {
                    // Fallback to copy + remove for cross-device moves
                    fs::copy(&src, &dest_file).and_then(|_| fs::remove_file(&src))
                }),
                "COPY" => fs::copy(&src, &dest_file).map(|_| ()),
                unknown => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown action type: {}", unknown),
                )),
            };

            match result {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!(
                    "Failed to {} '{}': {}",
                    action.action_type, action.source_path, e
                )),
            }
        }

        Ok(ActionResultPayload {
            success_count,
            errors,
        })
    })
    .await
    .map_err(|e| format!("Action executor error: {}", e))?
}