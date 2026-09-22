mod inference;

use inference::AppState;
use inference::registry::ModelRegistry;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    // Auto-discovers all available models from weights/ and registers them
    let registry = ModelRegistry::auto_discover();

    builder
        .manage(AppState {
            registry: Mutex::new(registry),
        })
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
        .invoke_handler(tauri::generate_handler![inference::execute_inference_task,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
