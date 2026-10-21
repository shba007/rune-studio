use std::{fs::OpenOptions, io::Write, time::SystemTime};

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

pub fn record(app: &AppHandle, entry: Value) {
    let Ok(dir) = app.path().app_data_dir() else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let mut line = json!({ "at": at });
    if let (Value::Object(base), Value::Object(extra)) = (&mut line, entry) {
        base.extend(extra);
    }
    let written = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("audit.log"))
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(err) = written {
        log::warn!("could not write the audit log: {err}");
    }
}
