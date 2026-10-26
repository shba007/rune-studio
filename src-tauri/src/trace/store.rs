use std::path::Path;

use rusqlite::{Connection, OpenFlags, Row, params, params_from_iter, types::Value};

use super::AnalysisRecord;
#[cfg(any(feature = "agent-bridge", test))]
use super::ReflectionRecord;
use super::{Msg, RunInfo, STEP_COLUMNS, StepRecord, VerdictRecord, schema};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("there is no trace database yet")]
    NotFound,
    #[error("the trace database was written by a newer Rune Studio (schema {0})")]
    Newer(i64),
    #[error("{0}")]
    Sqlite(String),
}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err.to_string())
    }
}

pub fn write_batch(db: &mut Connection, batch: &[Msg]) -> rusqlite::Result<()> {
    let tx = db.transaction()?;
    let mut first_error = None;
    for message in batch {
        if let Err(err) = apply(&tx, message) {
            first_error.get_or_insert(err);
        }
    }
    tx.commit()?;
    first_error.map_or(Ok(()), Err)
}

fn ensure_run(db: &Connection, run_id: &str) -> rusqlite::Result<()> {
    db.execute("INSERT OR IGNORE INTO runs (id) VALUES (?1)", [run_id])?;
    Ok(())
}

fn apply(db: &Connection, message: &Msg) -> rusqlite::Result<()> {
    match message {
        Msg::BeginRun(info) => {
            db.execute("UPDATE runs SET ended_utc = ?1, outcome = 'replaced' WHERE ended_utc IS NULL AND id != ?2", params![info.started_utc, info.id])?;
            db.execute(
                "INSERT INTO runs (id, scope_id, task, source, started_utc, plan_json, model, app_version, settings_json, session_dir)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET scope_id = excluded.scope_id, task = excluded.task, source = excluded.source, started_utc = excluded.started_utc,
                    plan_json = COALESCE(excluded.plan_json, runs.plan_json), model = excluded.model, app_version = excluded.app_version,
                    settings_json = excluded.settings_json, session_dir = excluded.session_dir",
                params![info.id, info.scope_id, info.task, info.source, info.started_utc, info.plan_json, info.model, info.app_version, info.settings_json, info.session_dir],
            )?;
        }
        Msg::SetPlan { run_id, plan_json } => {
            ensure_run(db, run_id)?;
            db.execute(
                "UPDATE runs SET plan_json = ?2 WHERE id = ?1",
                params![run_id, plan_json],
            )?;
        }
        Msg::Step(record) => {
            ensure_run(db, &record.run_id)?;
            let columns = STEP_COLUMNS.join(", ");
            let marks: Vec<String> = (3..3 + STEP_COLUMNS.len())
                .map(|n| format!("?{n}"))
                .collect();
            let updates: Vec<String> = STEP_COLUMNS
                .iter()
                .map(|column| format!("{column} = COALESCE(excluded.{column}, steps.{column})"))
                .collect();
            let sql = format!(
                "INSERT INTO steps (run_id, step, {columns}) VALUES (?1, ?2, {}) ON CONFLICT(run_id, step) DO UPDATE SET {}",
                marks.join(", "),
                updates.join(", ")
            );
            let mut values: Vec<Value> = vec![
                Value::Text(record.run_id.clone()),
                Value::Integer(i64::from(record.step)),
            ];
            values.extend(record.values());
            db.execute(&sql, params_from_iter(values))?;
        }
        Msg::Event {
            run_id,
            at_utc,
            kind,
            data_json,
        } => {
            ensure_run(db, run_id)?;
            db.execute(
                "INSERT INTO events (run_id, at_utc, kind, data_json) VALUES (?1, ?2, ?3, ?4)",
                params![run_id, at_utc, kind, data_json],
            )?;
        }
        Msg::EndRun {
            run_id,
            ended_utc,
            outcome,
            summary,
        } => {
            let ended = db.execute("UPDATE runs SET ended_utc = ?2, outcome = ?3, summary = ?4 WHERE id = ?1 AND ended_utc IS NULL", params![run_id, ended_utc, outcome, summary])?;
            if ended > 0 {
                db.execute(
                    "UPDATE runs SET
                        step_count = (SELECT COUNT(*) FROM steps WHERE run_id = ?1),
                        retry_count = (SELECT COUNT(*) FROM calls WHERE run_id = ?1 AND outcome = 'retried'),
                        duration_ms = CAST((julianday(ended_utc) - julianday(started_utc)) * 86400000 AS INTEGER)
                     WHERE id = ?1",
                    [run_id],
                )?;
            }
        }
        Msg::Verdict(record) => {
            ensure_run(db, &record.run_id)?;
            db.execute(
                "INSERT INTO verdicts (run_id, mode, result, evidence, error, model, latency_ms, claimed_success, challenged, created_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(run_id) DO UPDATE SET mode = excluded.mode, result = excluded.result, evidence = excluded.evidence, error = excluded.error,
                    model = excluded.model, latency_ms = excluded.latency_ms, claimed_success = excluded.claimed_success,
                    challenged = excluded.challenged, created_utc = excluded.created_utc",
                params![record.run_id, record.mode, record.result, record.evidence, record.error, record.model, record.latency_ms, record.claimed_success, record.challenged, record.created_utc],
            )?;
        }
        Msg::Analysis(record) => {
            db.execute(
                "INSERT INTO analyses (id, created_utc, question, answer, findings_json, model, latency_ms, status, error, screenshot_path, monitor_json, boxes_rejected)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET created_utc = excluded.created_utc, question = excluded.question, answer = excluded.answer,
                    findings_json = excluded.findings_json, model = excluded.model, latency_ms = excluded.latency_ms, status = excluded.status,
                    error = excluded.error, screenshot_path = excluded.screenshot_path, monitor_json = excluded.monitor_json, boxes_rejected = excluded.boxes_rejected",
                params![record.id, record.created_utc, record.question, record.answer, record.findings_json, record.model, record.latency_ms, record.status, record.error, record.screenshot_path, record.monitor_json, record.boxes_rejected],
            )?;
        }
        Msg::Call(record) => {
            db.execute(
                "INSERT INTO calls (run_id, step, kind, attempt, started_utc, duration_ms, wait_before_ms, status, outcome, error, request_json, response_json, truncated, model, prompt_tokens, completion_tokens, served_model, swap_ms, locality)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![record.run_id, record.step, record.kind, record.attempt, record.started_utc, record.duration_ms, record.wait_before_ms, record.status, record.outcome, record.error, record.request_json, record.response_json, record.truncated, record.model, record.prompt_tokens, record.completion_tokens, record.served_model, record.swap_ms, record.locality],
            )?;
        }
        Msg::Reflection(record) => {
            db.execute(
                "INSERT INTO reflections (id, started_utc, ended_utc, outcome, stages) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET started_utc = excluded.started_utc, ended_utc = excluded.ended_utc,
                    outcome = excluded.outcome, stages = excluded.stages",
                params![record.id, record.started_utc, record.ended_utc, record.outcome, record.stages],
            )?;
        }
        Msg::PurgeSessions(dirs) => {
            for dir in dirs {
                db.execute("DELETE FROM calls WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM verdicts WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM events WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM steps WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM runs WHERE session_dir = ?1", [dir])?;
            }
        }
        Msg::Flush(_) => {}
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    pub id: String,
    pub started_utc: Option<String>,
    pub ended_utc: Option<String>,
    pub source: Option<String>,
    pub outcome: Option<String>,
    pub task: Option<String>,
    pub steps: i64,
    pub verdict: Option<String>,
    pub session_dir: Option<String>,
    pub last_shot: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunRow {
    pub info: RunInfo,
    pub ended_utc: Option<String>,
    pub outcome: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventRow {
    pub id: i64,
    pub at_utc: Option<String>,
    pub kind: Option<String>,
    pub data_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunDetail {
    pub run: RunRow,
    pub steps: Vec<StepRecord>,
    pub events: Vec<EventRow>,
    pub verdict: Option<VerdictRecord>,
}

pub fn open_read(path: &Path) -> Result<Connection, StoreError> {
    if !path.exists() {
        return Err(StoreError::NotFound);
    }
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(std::time::Duration::from_millis(5000))?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > schema::VERSION {
        return Err(StoreError::Newer(version));
    }
    Ok(db)
}

pub fn list_runs(path: &Path, limit: usize) -> Result<Vec<RunSummary>, StoreError> {
    list_runs_in(&open_read(path)?, limit)
}

fn has_verdicts(db: &Connection) -> Result<bool, StoreError> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version >= 2)
}

pub fn list_runs_in(db: &Connection, limit: usize) -> Result<Vec<RunSummary>, StoreError> {
    let verdict = if has_verdicts(db)? {
        "(SELECT v.result FROM verdicts v WHERE v.run_id = r.id)"
    } else {
        "NULL"
    };
    let mut statement = db.prepare(&format!(
        "SELECT r.id, r.started_utc, r.ended_utc, r.source, r.outcome, r.task, (SELECT COUNT(*) FROM steps s WHERE s.run_id = r.id), {verdict}, r.session_dir,
                (SELECT s.observation_path FROM steps s WHERE s.run_id = r.id AND s.observation_path IS NOT NULL ORDER BY s.step DESC LIMIT 1)
         FROM runs r ORDER BY r.started_utc DESC, r.rowid DESC LIMIT ?1"
    ))?;
    let rows = statement.query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
        Ok(RunSummary {
            id: row.get(0)?,
            started_utc: row.get(1)?,
            ended_utc: row.get(2)?,
            source: row.get(3)?,
            outcome: row.get(4)?,
            task: row.get(5)?,
            steps: row.get(6)?,
            verdict: row.get(7)?,
            session_dir: row.get(8)?,
            last_shot: row.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_run(path: &Path, id: &str) -> Result<Option<RunDetail>, StoreError> {
    get_run_in(&open_read(path)?, id)
}

pub fn get_run_in(db: &Connection, id: &str) -> Result<Option<RunDetail>, StoreError> {
    let run = db.query_row(
        "SELECT id, scope_id, task, source, started_utc, plan_json, model, app_version, settings_json, session_dir, ended_utc, outcome, summary FROM runs WHERE id = ?1",
        [id],
        |row| {
            Ok(RunRow {
                info: RunInfo {
                    id: row.get(0)?,
                    scope_id: row.get(1)?,
                    task: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    source: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    started_utc: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    plan_json: row.get(5)?,
                    model: row.get(6)?,
                    app_version: row.get(7)?,
                    settings_json: row.get(8)?,
                    session_dir: row.get(9)?,
                },
                ended_utc: row.get(10)?,
                outcome: row.get(11)?,
                summary: row.get(12)?,
            })
        },
    );
    let run = match run {
        Ok(run) => run,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let version: i64 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);
    let sql = if version >= 7 {
        format!(
            "SELECT run_id, step, {} FROM steps WHERE run_id = ?1 ORDER BY step",
            STEP_COLUMNS.join(", ")
        )
    } else {
        format!(
            "SELECT run_id, step, {} FROM steps WHERE run_id = ?1 ORDER BY step",
            STEP_COLUMNS[..STEP_COLUMNS.len() - 1].join(", ")
        )
    };
    let steps = db
        .prepare(&sql)?
        .query_map([id], step_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    let events = db
        .prepare("SELECT id, at_utc, kind, data_json FROM events WHERE run_id = ?1 ORDER BY id")?
        .query_map([id], |row| {
            Ok(EventRow {
                id: row.get(0)?,
                at_utc: row.get(1)?,
                kind: row.get(2)?,
                data_json: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let verdict = if has_verdicts(db)? {
        let found = db.query_row(
            "SELECT mode, result, evidence, error, model, latency_ms, claimed_success, challenged, created_utc FROM verdicts WHERE run_id = ?1",
            [id],
            |row| {
                Ok(VerdictRecord {
                    run_id: id.to_string(),
                    mode: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    result: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    evidence: row.get(2)?,
                    error: row.get(3)?,
                    model: row.get(4)?,
                    latency_ms: row.get(5)?,
                    claimed_success: row.get::<_, Option<bool>>(6)?.unwrap_or_default(),
                    challenged: row.get::<_, Option<bool>>(7)?.unwrap_or_default(),
                    created_utc: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                })
            },
        );
        match found {
            Ok(record) => Some(record),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(err) => return Err(err.into()),
        }
    } else {
        None
    };
    Ok(Some(RunDetail {
        run,
        steps,
        events,
        verdict,
    }))
}

#[cfg(any(feature = "agent-bridge", test))]
pub fn list_analyses(path: &Path, limit: usize) -> Result<Vec<AnalysisRecord>, StoreError> {
    let db = open_read(path)?;
    if !has_analyses(&db)? {
        return Ok(Vec::new());
    }
    let mut statement = db.prepare(&format!(
        "SELECT {} FROM analyses ORDER BY created_utc DESC, rowid DESC LIMIT ?1",
        analysis_columns(&db)?
    ))?;
    let rows = statement.query_map(
        [i64::try_from(limit).unwrap_or(i64::MAX)],
        analysis_from_row,
    )?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(any(feature = "agent-bridge", test))]
pub fn list_reflections(path: &Path, limit: usize) -> Result<Vec<ReflectionRecord>, StoreError> {
    let db = open_read(path)?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 4 {
        return Ok(Vec::new());
    }
    let mut statement = db.prepare(
        "SELECT id, started_utc, ended_utc, outcome, stages FROM reflections ORDER BY started_utc DESC, rowid DESC LIMIT ?1",
    )?;
    let rows = statement.query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
        Ok(ReflectionRecord {
            id: row.get(0)?,
            started_utc: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ended_utc: row.get(2)?,
            outcome: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            stages: row.get::<_, Option<i64>>(4)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get_analysis(path: &Path, id: &str) -> Result<Option<AnalysisRecord>, StoreError> {
    let db = open_read(path)?;
    if !has_analyses(&db)? {
        return Ok(None);
    }
    match db.query_row(
        &format!(
            "SELECT {} FROM analyses WHERE id = ?1",
            analysis_columns(&db)?
        ),
        [id],
        analysis_from_row,
    ) {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

const ANALYSIS_COLUMNS: &str = "id, created_utc, question, answer, findings_json, model, latency_ms, status, error, screenshot_path, monitor_json";

fn analysis_columns(db: &Connection) -> Result<String, StoreError> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(format!(
        "{ANALYSIS_COLUMNS}, {}",
        if version >= 5 { "boxes_rejected" } else { "0" }
    ))
}

fn has_analyses(db: &Connection) -> Result<bool, StoreError> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version >= 3)
}

fn analysis_from_row(row: &Row<'_>) -> rusqlite::Result<AnalysisRecord> {
    let text = |index: usize| {
        row.get::<_, Option<String>>(index)
            .map(Option::unwrap_or_default)
    };
    Ok(AnalysisRecord {
        id: row.get(0)?,
        created_utc: text(1)?,
        question: text(2)?,
        answer: text(3)?,
        findings_json: text(4)?,
        model: row.get(5)?,
        latency_ms: row.get(6)?,
        status: text(7)?,
        error: row.get(8)?,
        screenshot_path: row.get(9)?,
        monitor_json: row.get(10)?,
        boxes_rejected: row.get::<_, Option<i64>>(11)?.unwrap_or_default(),
    })
}

fn step_from_row(row: &Row<'_>) -> rusqlite::Result<StepRecord> {
    let served_model = if row.as_ref().column_count() > 25 {
        row.get(25)?
    } else {
        None
    };
    Ok(StepRecord {
        run_id: row.get(0)?,
        step: row.get(1)?,
        started_utc: row.get(2)?,
        goal: row.get(3)?,
        tool: row.get(4)?,
        args_json: row.get(5)?,
        reason: row.get(6)?,
        risk_stated: row.get(7)?,
        risk_final: row.get(8)?,
        risk_reasons_json: row.get(9)?,
        gate: row.get(10)?,
        result: row.get(11)?,
        error: row.get(12)?,
        observation_path: row.get(13)?,
        elements_hash: row.get(14)?,
        prompt_text: row.get(15)?,
        thinking_chars: row.get(16)?,
        completion_tokens: row.get(17)?,
        observe_ms: row.get(18)?,
        screen_check_ms: row.get(19)?,
        finish_check_ms: row.get(20)?,
        decide_ms: row.get(21)?,
        gate_ms: row.get(22)?,
        act_ms: row.get(23)?,
        settle_ms: row.get(24)?,
        served_model,
    })
}
