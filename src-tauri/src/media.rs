use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
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
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailResult {
    pub id: String,
    pub thumbnail_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileActionRequest {
    pub action_type: String,
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

fn get_thumbnail_cache_path(folder_path: &Path, file_hash: &str) -> PathBuf {
    folder_path
        .join(".rune_cache")
        .join("thumbnails")
        .join(format!("{}.jpg", file_hash))
}

fn create_thumbnail(source_path: &Path, dest_path: &Path, media_type: &str) -> Result<(), String> {
    if let Some(parent) = dest_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if media_type == "image" {
        let img = image::open(source_path).map_err(|e| format!("Failed to open image: {}", e))?;
        let thumb = img.thumbnail(360, 360);
        let mut out_file = fs::File::create(dest_path)
            .map_err(|e| format!("Failed to create thumb file: {}", e))?;
        thumb
            .write_to(&mut out_file, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to write thumbnail: {}", e))?;
        return Ok(());
    }

    if media_type == "video" {
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-ss",
                "00:00:01",
                "-i",
                source_path.to_str().unwrap_or(""),
                "-vframes",
                "1",
                "-vf",
                "scale='min(360,iw)':-2",
                "-q:v",
                "4",
                dest_path.to_str().unwrap_or(""),
            ])
            .output();

        if let Ok(out) = status {
            if out.status.success() && dest_path.exists() {
                return Ok(());
            }
        }

        let status_fallback = Command::new("ffmpeg")
            .args([
                "-y",
                "-ss",
                "00:00:00",
                "-i",
                source_path.to_str().unwrap_or(""),
                "-vframes",
                "1",
                "-vf",
                "scale='min(360,iw)':-2",
                "-q:v",
                "4",
                dest_path.to_str().unwrap_or(""),
            ])
            .output();

        if let Ok(out) = status_fallback {
            if out.status.success() && dest_path.exists() {
                return Ok(());
            }
        }

        return Err("FFmpeg thumbnail extraction failed".to_string());
    }

    Err("Unsupported media type for thumbnail".to_string())
}

#[tauri::command]
pub async fn pick_directory() -> Result<Option<String>, String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Select Media Folder")
        .pick_folder()
        .await;

    Ok(folder.map(|handle| handle.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn scan_directory(
    path: String,
    recursive: bool,
) -> Result<Vec<MediaItemPayload>, String> {
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
            .into_iter()
            .filter_entry(|e| {
                if e.depth() > 0 {
                    if let Some(name) = e.file_name().to_str() {
                        if name.starts_with('.') {
                            return false;
                        }
                    }
                }
                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            let path_buf = entry.path();
            if !path_buf.is_file() {
                continue;
            }

            if let Ok(rel) = path_buf.strip_prefix(base_path) {
                if rel
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
                {
                    continue;
                }
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
                let safe_hash =
                    format!("{}_{}", metadata.len(), last_modified).replace(['/', '\\', ':'], "_");
                let thumb_path = get_thumbnail_cache_path(base_path, &safe_hash);

                let thumbnail_path = if thumb_path.exists() {
                    Some(thumb_path.to_string_lossy().to_string())
                } else {
                    None
                };

                results.push(MediaItemPayload {
                    id: format!("{}:{}", full_path, metadata.len()),
                    name: file_name,
                    path: full_path,
                    size: metadata.len(),
                    mime_type: mime_type.to_string(),
                    media_type: media_type.to_string(),
                    last_modified,
                    thumbnail_path,
                });
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("Scanner thread failed: {}", e))?
}

#[tauri::command]
pub async fn generate_thumbnails(
    items: Vec<MediaItemPayload>,
    folder_path: String,
) -> Result<Vec<ThumbnailResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_dir = Path::new(&folder_path)
            .join(".rune_cache")
            .join("thumbnails");
        let _ = fs::create_dir_all(&base_dir);

        let items_arc = Arc::new(Mutex::new(items));
        let results_arc = Arc::new(Mutex::new(Vec::new()));

        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(8);

        let mut handles = Vec::new();

        for _ in 0..num_workers {
            let items_clone = Arc::clone(&items_arc);
            let results_clone = Arc::clone(&results_arc);
            let base_dir_clone = base_dir.clone();

            let handle = std::thread::spawn(move || {
                loop {
                    let next_item = {
                        let mut lock = items_clone.lock().unwrap();
                        lock.pop()
                    };

                    let item = match next_item {
                        Some(it) => it,
                        None => break,
                    };

                    let safe_hash = format!("{}_{}", item.size, item.last_modified)
                        .replace(['/', '\\', ':'], "_");
                    let dest_path = base_dir_clone.join(format!("{}.jpg", safe_hash));

                    let need_generate = !dest_path.exists();
                    let success = if need_generate {
                        create_thumbnail(Path::new(&item.path), &dest_path, &item.media_type)
                            .is_ok()
                    } else {
                        true
                    };

                    if success && dest_path.exists() {
                        let mut res_lock = results_clone.lock().unwrap();
                        res_lock.push(ThumbnailResult {
                            id: item.id,
                            thumbnail_path: dest_path.to_string_lossy().to_string(),
                        });
                    }
                }
            });

            handles.push(handle);
        }

        for h in handles {
            let _ = h.join();
        }

        let final_results = results_arc.lock().unwrap().clone();
        Ok(final_results)
    })
    .await
    .map_err(|e| format!("Thumbnail generator failed: {}", e))?
}

#[tauri::command]
pub async fn upload_media_to_s3(file_path: String, bucket_url: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let src_path = Path::new(&file_path);
        if !src_path.exists() {
            return Err(format!("File does not exist: {}", file_path));
        }

        let ext = src_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let (_, mime_type) = determine_media_type(&ext)
            .ok_or_else(|| format!("Unsupported media type for file: {}", file_path))?;

        let file_name = src_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("media_file");

        let nonce = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let safe_file_name: String = file_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let object_name = format!("{}_{}", nonce, safe_file_name);
        let trimmed_bucket = bucket_url.trim_end_matches('/');
        let target_url = format!("{}/{}", trimmed_bucket, object_name);

        let access_key =
            std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "rustfsadmin".to_string());
        let secret_key =
            std::env::var("S3_SECRET_KEY").unwrap_or_else(|_| "rustfsadmin".to_string());
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let sigv4_arg = format!("aws:amz:{}:s3", region);
        let user_arg = format!("{}:{}", access_key, secret_key);

        // Attempt upload with public-read ACL and fail-with-body
        let output = Command::new("curl")
            .args([
                "-s",
                "-S",
                "--fail-with-body",
                "-X",
                "PUT",
                "--aws-sigv4",
                &sigv4_arg,
                "--user",
                &user_arg,
                "-H",
                &format!("Content-Type: {}", mime_type),
                "-H",
                "x-amz-acl: public-read",
                "-T",
                &file_path,
                &target_url,
            ])
            .output()
            .map_err(|e| format!("Failed to invoke curl: {}", e))?;

        if !output.status.success() {
            // Fallback for unauthenticated local S3 emulator
            let fallback = Command::new("curl")
                .args([
                    "-s",
                    "-S",
                    "--fail-with-body",
                    "-X",
                    "PUT",
                    "-H",
                    &format!("Content-Type: {}", mime_type),
                    "-H",
                    "x-amz-acl: public-read",
                    "-T",
                    &file_path,
                    &target_url,
                ])
                .output()
                .map_err(|e| format!("Upload fallback invocation error: {}", e))?;

            if !fallback.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                let err_body = String::from_utf8_lossy(&output.stdout);
                return Err(format!(
                    "S3 upload failed: {} (Server response: {})",
                    err_msg, err_body
                ));
            }
        }

        Ok(target_url)
    })
    .await
    .map_err(|e| format!("S3 upload thread failed: {}", e))?
}

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
    .map_err(|e| format!("Media processing error: {}", e))?
}

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
                errors.push(format!(
                    "Source file does not exist: {}",
                    action.source_path
                ));
                continue;
            }

            let dest_dir = PathBuf::from(&action.target_directory);
            if let Err(e) = fs::create_dir_all(&dest_dir) {
                errors.push(format!(
                    "Failed to create folder {}: {}",
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
                "MOVE" => fs::rename(&src, &dest_file)
                    .or_else(|_| fs::copy(&src, &dest_file).and_then(|_| fs::remove_file(&src))),
                "COPY" => fs::copy(&src, &dest_file).map(|_| ()),
                unknown => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown action: {}", unknown),
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

#[tauri::command]
pub async fn save_artifact_to_disk(
    folder_path: String,
    file_hash: String,
    intent: String,
    data: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_dir = Path::new(&folder_path)
            .join(".rune_cache")
            .join("assessments");
        fs::create_dir_all(&base_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let safe_hash = file_hash.replace(['/', '\\', ':'], "_");
        let file_path = base_dir.join(format!("{}.{}.json", safe_hash, intent));

        fs::write(file_path, data).map_err(|e| format!("Failed to write artifact: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Artifact write thread failed: {}", e))?
}

#[tauri::command]
pub async fn load_artifact_from_disk(
    folder_path: String,
    file_hash: String,
    intent: String,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let safe_hash = file_hash.replace(['/', '\\', ':'], "_");
        let file_path = Path::new(&folder_path)
            .join(".rune_cache")
            .join("assessments")
            .join(format!("{}.{}.json", safe_hash, intent));

        if !file_path.exists() {
            return Ok(None);
        }

        let content =
            fs::read_to_string(file_path).map_err(|e| format!("Failed to read artifact: {}", e))?;
        Ok(Some(content))
    })
    .await
    .map_err(|e| format!("Artifact read thread failed: {}", e))?
}

#[tauri::command]
pub async fn clear_directory_cache(folder_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cache_dir = Path::new(&folder_path).join(".rune_cache");
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)
                .map_err(|e| format!("Failed to delete .rune_cache: {}", e))?;
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Clear directory cache thread failed: {}", e))?
}

#[tauri::command]
pub async fn delete_artifact_from_disk(
    folder_path: String,
    file_hash: String,
    intent: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let safe_hash = file_hash.replace(['/', '\\', ':'], "_");
        let file_path = Path::new(&folder_path)
            .join(".rune_cache")
            .join("assessments")
            .join(format!("{}.{}.json", safe_hash, intent));
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| format!("Failed to delete artifact: {}", e))?;
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Delete artifact thread failed: {}", e))?
}
