//! The trace database's tables and how an older file is brought up to date.

use rusqlite::Connection;

/// The version this code writes. `PRAGMA user_version` holds the version of a database file.
pub const VERSION: i64 = 2;

/// Version 1.
pub const CREATE_V1: &str = "
CREATE TABLE runs (
    id TEXT PRIMARY KEY,
    scope_id INTEGER,
    task TEXT,
    source TEXT,
    started_utc TEXT,
    ended_utc TEXT,
    outcome TEXT,
    summary TEXT,
    plan_json TEXT,
    model TEXT,
    app_version TEXT,
    settings_json TEXT,
    session_dir TEXT
);

CREATE TABLE steps (
    run_id TEXT NOT NULL REFERENCES runs(id),
    step INTEGER NOT NULL,
    started_utc TEXT,
    goal INTEGER,
    tool TEXT,
    args_json TEXT,
    reason TEXT,
    risk_stated TEXT,
    risk_final TEXT,
    risk_reasons_json TEXT,
    gate TEXT,
    result TEXT,
    error TEXT,
    observation_path TEXT,
    elements_hash TEXT,
    prompt_text TEXT,
    thinking_chars INTEGER,
    completion_tokens INTEGER,
    observe_ms INTEGER,
    screen_check_ms INTEGER,
    finish_check_ms INTEGER,
    decide_ms INTEGER,
    gate_ms INTEGER,
    act_ms INTEGER,
    settle_ms INTEGER,
    PRIMARY KEY (run_id, step)
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES runs(id),
    at_utc TEXT,
    kind TEXT,
    data_json TEXT
);

CREATE INDEX steps_run ON steps(run_id);
CREATE INDEX events_run ON events(run_id);
CREATE INDEX runs_started ON runs(started_utc);
";

/// Version 2: the independent check of a finished task (see `agent::verify`). One row per run.
pub const CREATE_V2: &str = "
CREATE TABLE verdicts (
    run_id TEXT PRIMARY KEY REFERENCES runs(id),
    mode TEXT,
    result TEXT,
    evidence TEXT,
    error TEXT,
    model TEXT,
    latency_ms INTEGER,
    claimed_success INTEGER,
    challenged INTEGER,
    created_utc TEXT
);
";

/// How opening a database went.
#[derive(Debug, PartialEq, Eq)]
pub enum Opened {
    /// At the current version (created or migrated if it needed to be).
    Ready,
    /// Written by a newer build. It is left exactly as it is.
    Newer(i64),
}

/// Applies the pragmas and brings the file to `VERSION`. A database from a newer build is not touched.
pub fn open(db: &Connection) -> rusqlite::Result<Opened> {
    let found: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > VERSION {
        return Ok(Opened::Newer(found));
    }
    db.busy_timeout(std::time::Duration::from_millis(5000))?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")?;
    // One transaction per step, so a crash cannot leave half a schema behind.
    if found < 1 {
        db.execute_batch(&format!("BEGIN; {CREATE_V1} {CREATE_V2} PRAGMA user_version = 2; COMMIT;"))?;
    } else if found < 2 {
        db.execute_batch(&format!("BEGIN; {CREATE_V2} PRAGMA user_version = 2; COMMIT;"))?;
    }
    Ok(Opened::Ready)
}
