mod media;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());

    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            media::pick_directory,
            media::scan_directory,
            media::generate_thumbnails,
            media::get_media_base64,
            media::upload_media_to_s3,
            media::execute_file_actions,
            media::save_artifact_to_disk,
            media::load_artifact_from_disk,
            media::clear_directory_cache,
            media::delete_artifact_from_disk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
