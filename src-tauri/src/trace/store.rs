//! Blocking reads and writes on the trace database. The writer thread calls `write_batch`;
//! tooling (the command line) reads through `list_runs` and `get_run`, each on its own read-only
//! connection.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, Row, params, params_from_iter, types::Value};

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

/// Applies the messages in one transaction. A message that fails does not stop the others; the
/// first error is returned after the rest have been written.
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

/// A step or event may arrive for a run whose beginning was dropped; give it a bare row to hang on
/// rather than lose it.
fn ensure_run(db: &Connection, run_id: &str) -> rusqlite::Result<()> {
    db.execute("INSERT OR IGNORE INTO runs (id) VALUES (?1)", [run_id])?;
    Ok(())
}

fn apply(db: &Connection, message: &Msg) -> rusqlite::Result<()> {
    match message {
        Msg::BeginRun(info) => {
            // A run that is still open when another begins was replaced (this also tidies runs
            // left open by a crash); the same label the session folder gets.
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
            db.execute("UPDATE runs SET plan_json = ?2 WHERE id = ?1", params![run_id, plan_json])?;
        }
        Msg::Step(record) => {
            ensure_run(db, &record.run_id)?;
            // A piece overwrites only what it carries.
            let columns = STEP_COLUMNS.join(", ");
            let marks: Vec<String> = (3..3 + STEP_COLUMNS.len()).map(|n| format!("?{n}")).collect();
            let updates: Vec<String> = STEP_COLUMNS.iter().map(|column| format!("{column} = COALESCE(excluded.{column}, steps.{column})")).collect();
            let sql = format!(
                "INSERT INTO steps (run_id, step, {columns}) VALUES (?1, ?2, {}) ON CONFLICT(run_id, step) DO UPDATE SET {}",
                marks.join(", "),
                updates.join(", ")
            );
            let mut values: Vec<Value> = vec![Value::Text(record.run_id.clone()), Value::Integer(i64::from(record.step))];
            values.extend(record.values());
            db.execute(&sql, params_from_iter(values))?;
        }
        Msg::Event { run_id, at_utc, kind, data_json } => {
            ensure_run(db, run_id)?;
            db.execute("INSERT INTO events (run_id, at_utc, kind, data_json) VALUES (?1, ?2, ?3, ?4)", params![run_id, at_utc, kind, data_json])?;
        }
        Msg::EndRun { run_id, ended_utc, outcome, summary } => {
            // Only the first end counts.
            db.execute("UPDATE runs SET ended_utc = ?2, outcome = ?3, summary = ?4 WHERE id = ?1 AND ended_utc IS NULL", params![run_id, ended_utc, outcome, summary])?;
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
        Msg::PurgeSessions(dirs) => {
            for dir in dirs {
                // Foreign keys are on: what hangs on a run goes first.
                db.execute("DELETE FROM verdicts WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM events WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM steps WHERE run_id IN (SELECT id FROM runs WHERE session_dir = ?1)", [dir])?;
                db.execute("DELETE FROM runs WHERE session_dir = ?1", [dir])?;
            }
        }
        #[cfg(test)]
        Msg::Flush(_) => {}
    }
    Ok(())
}

// ---------------------------------------------------------------- reading

/// One line of the run list.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    pub id: String,
    pub started_utc: Option<String>,
    pub ended_utc: Option<String>,
    pub source: Option<String>,
    pub outcome: Option<String>,
    pub task: Option<String>,
    pub steps: i64,
    /// The result of the independent check (`verified`, `contradicted`, ...), if there is one.
    pub verdict: Option<String>,
}

/// A whole run row.
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
    /// The independent check of this run, if one was stored.
    pub verdict: Option<VerdictRecord>,
}

/// Opens the database read-only. A missing file and a newer schema are errors the caller can word.
pub fn open_read(path: &Path) -> Result<Connection, StoreError> {
    if !path.exists() {
        return Err(StoreError::NotFound);
    }
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    db.busy_timeout(std::time::Duration::from_millis(5000))?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > schema::VERSION {
        return Err(StoreError::Newer(version));
    }
    Ok(db)
}

/// The newest runs first, at most `limit`.
pub fn list_runs(path: &Path, limit: usize) -> Result<Vec<RunSummary>, StoreError> {
    list_runs_in(&open_read(path)?, limit)
}

/// Whether this file has the `verdicts` table: a version 1 database that the app has not opened yet does not.
fn has_verdicts(db: &Connection) -> Result<bool, StoreError> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version >= 2)
}

pub fn list_runs_in(db: &Connection, limit: usize) -> Result<Vec<RunSummary>, StoreError> {
    let verdict = if has_verdicts(db)? { "(SELECT v.result FROM verdicts v WHERE v.run_id = r.id)" } else { "NULL" };
    let mut statement = db.prepare(&format!(
        "SELECT r.id, r.started_utc, r.ended_utc, r.source, r.outcome, r.task, (SELECT COUNT(*) FROM steps s WHERE s.run_id = r.id), {verdict}
         FROM runs r ORDER BY r.started_utc DESC, r.rowid DESC LIMIT ?1"
    ))?;
    let rows = statement.query_map([i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
        Ok(RunSummary { id: row.get(0)?, started_utc: row.get(1)?, ended_utc: row.get(2)?, source: row.get(3)?, outcome: row.get(4)?, task: row.get(5)?, steps: row.get(6)?, verdict: row.get(7)? })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// One run with its steps (in order) and events (in order). `None` when there is no such run.
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
    let sql = format!("SELECT run_id, step, {} FROM steps WHERE run_id = ?1 ORDER BY step", STEP_COLUMNS.join(", "));
    let steps = db.prepare(&sql)?.query_map([id], step_from_row)?.collect::<Result<Vec<_>, _>>()?;
    let events = db
        .prepare("SELECT id, at_utc, kind, data_json FROM events WHERE run_id = ?1 ORDER BY id")?
        .query_map([id], |row| Ok(EventRow { id: row.get(0)?, at_utc: row.get(1)?, kind: row.get(2)?, data_json: row.get(3)? }))?
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
    Ok(Some(RunDetail { run, steps, events, verdict }))
}

fn step_from_row(row: &Row<'_>) -> rusqlite::Result<StepRecord> {
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
    })
}
