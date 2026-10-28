//! Tracks in-flight model requests for the orb without exposing prompts or keys.
use serde::Serialize;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: u64,
    pub model: String,
    pub reported: bool,
}
static NEXT: AtomicU64 = AtomicU64::new(1);
static ACTIVE: Mutex<Vec<Activity>> = Mutex::new(Vec::new());

#[tauri::command]
pub fn model_activity() -> Vec<Activity> {
    ACTIVE
        .lock()
        .map(|active| active.clone())
        .unwrap_or_default()
}

fn publish(app: &AppHandle, active: &[Activity]) {
    let _ = app.emit("model_activity", active);
}

pub struct Guard {
    app: AppHandle,
    id: u64,
}
impl Guard {
    pub fn start(app: &AppHandle, model: &str) -> Self {
        let guard = Self {
            app: app.clone(),
            id: NEXT.fetch_add(1, Ordering::Relaxed),
        };
        if let Ok(mut active) = ACTIVE.lock() {
            active.push(Activity {
                id: guard.id,
                model: model.to_owned(),
                reported: false,
            });
            publish(app, &active);
        }
        guard
    }
    pub fn reported(&self, model: &str) {
        if model.is_empty() {
            return;
        }
        if let Ok(mut active) = ACTIVE.lock()
            && let Some(item) = active.iter_mut().find(|item| item.id == self.id)
        {
            item.model = model.to_owned();
            item.reported = true;
            publish(&self.app, &active);
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut active) = ACTIVE.lock() {
            active.retain(|item| item.id != self.id);
            publish(&self.app, &active);
        }
    }
}
