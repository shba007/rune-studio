pub mod files;

use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::Serialize;
use serde_json::{Value, json};

use crate::trace::{
    self,
    store::{self, StoreError},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlightRun {
    pub id: String,
    pub task: String,
    pub source: String,
    pub outcome: Option<String>,
    pub started_utc: Option<String>,
    pub ended_utc: Option<String>,
    pub steps: i64,
    pub verdict: Option<String>,
    pub thumb: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlightHeader {
    pub id: String,
    pub task: String,
    pub source: String,
    pub model: Option<String>,
    pub started_utc: String,
    pub ended_utc: Option<String>,
    pub outcome: Option<String>,
    pub summary: Option<String>,
    pub plan: Option<Value>,
    pub steps: i64,
    pub retries: i64,
    pub duration_secs: Option<i64>,
    pub verdict: Option<Value>,
    pub video: Option<Value>,
    pub session_dir: Option<String>,
    pub analysis: Option<FlightAnalysis>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlightAnalysis {
    pub answer: String,
    pub findings: Value,
    pub model: Option<String>,
    pub captured_utc: String,
    pub capture_file: Option<String>,
    pub error: Option<String>,
}

fn insight_directory(record: &trace::AnalysisRecord) -> Option<PathBuf> {
    let path = Path::new(record.screenshot_path.as_deref()?);
    let parent = path.parent()?;
    (path.is_absolute()
        && path.file_name()? == "capture.png"
        && parent.file_name()? == "screenshots")
        .then(|| parent.parent().map(Path::to_path_buf))
        .flatten()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlightCall {
    pub id: i64,
    pub step: Option<i64>,
    pub kind: String,
    pub attempt: i64,
    pub started_utc: String,
    pub duration_ms: i64,
    pub wait_before_ms: i64,
    pub status: Option<i64>,
    pub outcome: String,
    pub error: Option<String>,
    pub truncated: bool,
    pub model: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub served_model: Option<String>,
    pub locality: Option<String>,
    pub swap_ms: Option<i64>,
}

fn has_table(db: &rusqlite::Connection, name: &str) -> bool {
    db.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |_| Ok(()),
    )
    .is_ok()
}

pub fn runs(db: &Path, limit: usize, before: Option<&str>) -> Result<Vec<FlightRun>, StoreError> {
    let all = store::list_runs(db, limit.saturating_add(200).min(5_000))?;
    Ok(all
        .into_iter()
        .filter(|row| before.is_none_or(|cut| row.started_utc.as_deref().is_some_and(|s| s < cut)))
        .take(limit)
        .map(|row| FlightRun {
            thumb: row
                .session_dir
                .as_deref()
                .zip(row.last_shot.as_deref())
                .and_then(|(dir, shot)| files::relative_to(Path::new(dir), shot))
                .or_else(|| {
                    if row.source.as_deref() != Some("screen-insight") {
                        return None;
                    }
                    store::get_analysis(db, &row.id)
                        .ok()
                        .flatten()
                        .as_ref()
                        .and_then(insight_directory)
                        .map(|_| "screenshots/capture.png".into())
                }),
            id: row.id,
            task: row.task.unwrap_or_default(),
            source: row.source.unwrap_or_default(),
            outcome: row.outcome,
            started_utc: row.started_utc,
            ended_utc: row.ended_utc,
            steps: row.steps,
            verdict: row.verdict,
            active: false,
        })
        .collect())
}

fn retries_of(db: &Path, id: &str) -> i64 {
    let Ok(conn) = store::open_read(db) else {
        return 0;
    };
    if !has_table(&conn, "calls") {
        return 0;
    }
    conn.query_row(
        "SELECT COUNT(*) FROM calls WHERE run_id = ?1 AND outcome = 'retried'",
        [id],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

pub fn header(db: &Path, id: &str) -> Result<Option<FlightHeader>, StoreError> {
    let Some(detail) = store::get_run(db, id)? else {
        return Ok(None);
    };
    let info = &detail.run.info;
    let analysis = if info.source == "screen-insight" {
        store::get_analysis(db, id)?
    } else {
        None
    };
    let session = info
        .session_dir
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| analysis.as_ref().and_then(insight_directory));
    let duration = detail
        .run
        .ended_utc
        .as_deref()
        .and_then(trace::parse_iso)
        .zip(trace::parse_iso(&info.started_utc))
        .map(|(end, start)| i64::try_from(end.saturating_sub(start)).unwrap_or(0));
    let video = info
        .session_dir
        .as_deref()
        .and_then(|dir| std::fs::read_to_string(Path::new(dir).join("session.json")).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|manifest| manifest.get("video").cloned());
    Ok(Some(FlightHeader {
        id: info.id.clone(),
        task: info.task.clone(),
        source: info.source.clone(),
        model: info.model.clone(),
        started_utc: info.started_utc.clone(),
        ended_utc: detail.run.ended_utc.clone(),
        outcome: detail.run.outcome.clone(),
        summary: detail.run.summary.clone(),
        plan: info.plan_json.as_deref().and_then(|text| serde_json::from_str(text).ok()),
        steps: i64::try_from(detail.steps.len()).unwrap_or(i64::MAX),
        retries: retries_of(db, id),
        duration_secs: duration,
        verdict: detail.verdict.as_ref().map(|verdict| {
            json!({ "result": verdict.result, "evidence": verdict.evidence, "mode": verdict.mode, "error": verdict.error })
        }),
        video,
        session_dir: session.as_ref().map(|dir| dir.display().to_string()),
        analysis: analysis.map(|record| FlightAnalysis {
            capture_file: insight_directory(&record).map(|_| "screenshots/capture.png".into()),
            answer: record.answer,
            findings: serde_json::from_str(&record.findings_json).unwrap_or_else(|_| json!([])),
            model: record.model,
            captured_utc: record.created_utc,
            error: record.error,
        }),
    }))
}

pub fn steps(db: &Path, id: &str) -> Result<Vec<Value>, StoreError> {
    let Some(detail) = store::get_run(db, id)? else {
        return Ok(Vec::new());
    };
    let session = detail.run.info.session_dir.clone().map(PathBuf::from);
    Ok(detail
        .steps
        .iter()
        .map(|step| {
            json!({
                "step": step.step, "startedUtc": step.started_utc, "goal": step.goal, "tool": step.tool,
                "argsJson": step.args_json, "reason": step.reason, "riskStated": step.risk_stated,
                "riskFinal": step.risk_final, "riskReasonsJson": step.risk_reasons_json, "gate": step.gate,
                "result": step.result, "error": step.error, "observationPath": step.observation_path,
                "observationFile": session.as_deref().zip(step.observation_path.as_deref()).and_then(|(dir, stored)| files::relative_to(dir, stored)),
                "promptText": step.prompt_text, "thinkingChars": step.thinking_chars,
                "completionTokens": step.completion_tokens,
                "phases": {
                    "observe": step.observe_ms, "screenCheck": step.screen_check_ms, "finishCheck": step.finish_check_ms,
                    "decide": step.decide_ms, "gate": step.gate_ms, "act": step.act_ms, "settle": step.settle_ms,
                },
            })
        })
        .collect())
}

pub fn events(db: &Path, id: &str) -> Result<Vec<Value>, StoreError> {
    let Some(detail) = store::get_run(db, id)? else {
        return Ok(Vec::new());
    };
    Ok(detail.events.iter().map(|event| json!({ "id": event.id, "atUtc": event.at_utc, "kind": event.kind, "dataJson": event.data_json })).collect())
}

pub fn calls(db: &Path, id: &str, step: Option<i64>) -> Result<Vec<FlightCall>, StoreError> {
    let conn = store::open_read(db)?;
    if !has_table(&conn, "calls") {
        return Ok(Vec::new());
    }
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);
    let sql = if version >= 8 {
        "SELECT id, step, kind, attempt, started_utc, duration_ms, wait_before_ms, status, outcome, error, truncated, model, prompt_tokens, completion_tokens, served_model, swap_ms, locality
         FROM calls WHERE run_id = ?1 AND (?2 IS NULL OR step = ?2) ORDER BY id"
    } else if version >= 7 {
        "SELECT id, step, kind, attempt, started_utc, duration_ms, wait_before_ms, status, outcome, error, truncated, model, prompt_tokens, completion_tokens, served_model, swap_ms, NULL
         FROM calls WHERE run_id = ?1 AND (?2 IS NULL OR step = ?2) ORDER BY id"
    } else {
        "SELECT id, step, kind, attempt, started_utc, duration_ms, wait_before_ms, status, outcome, error, truncated, model, prompt_tokens, completion_tokens, NULL, NULL, NULL
         FROM calls WHERE run_id = ?1 AND (?2 IS NULL OR step = ?2) ORDER BY id"
    };
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![id, step], |row| {
        Ok(FlightCall {
            id: row.get(0)?,
            step: row.get(1)?,
            kind: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            attempt: row.get::<_, Option<i64>>(3)?.unwrap_or(1),
            started_utc: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            duration_ms: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
            wait_before_ms: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
            status: row.get(7)?,
            outcome: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            error: row.get(9)?,
            truncated: row.get::<_, Option<bool>>(10)?.unwrap_or(false),
            model: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
            prompt_tokens: row.get(12)?,
            completion_tokens: row.get(13)?,
            served_model: row.get(14)?,
            swap_ms: row.get(15)?,
            locality: row.get(16)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn call_bodies(db: &Path, call_id: i64) -> Result<Option<(String, String)>, StoreError> {
    let conn = store::open_read(db)?;
    if !has_table(&conn, "calls") {
        return Ok(None);
    }
    match conn.query_row(
        "SELECT request_json, response_json FROM calls WHERE id = ?1",
        [call_id],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ))
        },
    ) {
        Ok(bodies) => Ok(Some(bodies)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn session_dir(db: &Path, id: &str) -> Result<Option<PathBuf>, StoreError> {
    let Some(detail) = store::get_run(db, id)? else {
        return Ok(None);
    };
    if let Some(dir) = detail.run.info.session_dir.filter(|dir| !dir.is_empty()) {
        return Ok(Some(PathBuf::from(dir)));
    }
    if detail.run.info.source == "screen-insight" {
        return Ok(store::get_analysis(db, id)?
            .as_ref()
            .and_then(insight_directory));
    }
    Ok(None)
}

pub fn original_prompt(db: &Path, id: &str) -> Result<Option<String>, StoreError> {
    let Some(detail) = store::get_run(db, id)? else {
        return Ok(None);
    };
    Ok(detail
        .events
        .iter()
        .rev()
        .find(|event| event.kind.as_deref() == Some("user_prompt"))
        .and_then(|event| event.data_json.as_deref())
        .and_then(|data| serde_json::from_str::<Value>(data).ok())
        .filter(|value| value["complete"] == true)
        .and_then(|value| value["text"].as_str().map(str::to_owned)))
}

pub fn folder_shared(db: &Path, id: &str, dir: &Path) -> Result<bool, StoreError> {
    let Ok(dir) = dir.canonicalize() else {
        return Ok(false);
    };
    let conn = store::open_read(db)?;
    for (table, key, column) in [
        ("runs", "id", "session_dir"),
        ("analyses", "id", "screenshot_path"),
    ] {
        if !has_table(&conn, table) {
            continue;
        }
        let mut query = conn.prepare(&format!(
            "SELECT {column} FROM {table} WHERE {key} != ?1 AND {column} IS NOT NULL"
        ))?;
        let paths = query.query_map([id], |row| row.get::<_, String>(0))?;
        for path in paths {
            if Path::new(&path?)
                .canonicalize()
                .is_ok_and(|path| path.starts_with(&dir))
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn delete_record(db: &Path, id: &str) -> Result<(), StoreError> {
    let mut conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > trace::schema::VERSION {
        return Err(StoreError::Newer(version));
    }
    let tx = conn.transaction()?;
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM runs WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(StoreError::Sqlite("There is no such session".into()));
    }
    if has_table(&tx, "task_links") {
        store::unlink_task_run(&tx, id)?;
    }
    for (table, key) in [
        ("calls", "run_id"),
        ("verdicts", "run_id"),
        ("events", "run_id"),
        ("steps", "run_id"),
        ("analyses", "id"),
        ("reflections", "id"),
        ("runs", "id"),
    ] {
        if has_table(&tx, table) {
            tx.execute(&format!("DELETE FROM {table} WHERE {key} = ?1"), [id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub const WINDOW: &str = "flight";
pub const OPEN_EVENT: &str = "flight_open";

fn url_safe(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' => char::from(byte).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

pub fn open(app: &tauri::AppHandle, run: Option<&str>) -> tauri::Result<()> {
    use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

    let run = run.filter(|id| !id.is_empty());
    let Some(run) = run else {
        return crate::hub::open(app, Some("artifacts"));
    };
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        window.set_always_on_top(true)?;
        window.show()?;
        window.set_focus()?;
        window.emit(OPEN_EVENT, json!({ "runId": run }))?;
        return Ok(());
    }
    let url = format!("index.html?view=flight&run={}", url_safe(run));
    let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App(url.into()))
        .title("Rune Preview")
        .always_on_top(true)
        .inner_size(1180.0, 760.0)
        .min_inner_size(880.0, 560.0)
        .resizable(true)
        .decorations(false)
        .center()
        .build()?;
    crate::window::protect(&window)?;
    window.set_focus()?;
    Ok(())
}

pub fn reveal(dir: &Path) -> std::io::Result<()> {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests;
