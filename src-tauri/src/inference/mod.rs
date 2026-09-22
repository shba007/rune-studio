pub mod types;
pub mod traits;
pub mod engines;
pub mod registry;

use std::sync::Mutex;
use tauri::State;
use registry::ModelRegistry;
use types::{InferenceTaskRequest, InferenceTaskResponse};

pub struct AppState {
    pub registry: Mutex<ModelRegistry>,
}

#[tauri::command]
pub async fn execute_inference_task(
    engine_id: Option<String>,
    task: InferenceTaskRequest,
    state: State<'_, AppState>,
) -> Result<InferenceTaskResponse, String> {
    // 1. Resolve engine and release the registry lock immediately
    let engine = {
        let registry = state
            .registry
            .lock()
            .map_err(|e| format!("Registry lock poisoned: {}", e))?;

        registry
            .resolve_engine(engine_id.as_deref(), &task)
            .ok_or_else(|| "No compatible inference engine found for this task".to_string())?
    };

    // 2. Only move the owned `task` and `Arc<dyn InferenceEngine>` into the background thread
    tauri::async_runtime::spawn_blocking(move || {
        engine
            .execute(&task)
            .map_err(|e| format!("Engine execution error: {}", e))
    })
    .await
    .map_err(|e| format!("Async runtime join error: {}", e))?
}