//! The trace database's tables and how an older file is brought up to date.

use rusqlite::Connection;

/// The version this code writes. `PRAGMA user_version` holds the version of a database file.
pub const VERSION: i64 = 6;

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

/// Version 3: visual analyses (see `analysis`). One row per analysis; the id is the archive session id.
pub const CREATE_V3: &str = "
CREATE TABLE analyses (
    id TEXT PRIMARY KEY,
    created_utc TEXT,
    question TEXT,
    answer TEXT,
    findings_json TEXT,
    model TEXT,
    latency_ms INTEGER,
    status TEXT,
    error TEXT,
    screenshot_path TEXT,
    monitor_json TEXT
);

CREATE INDEX analyses_created ON analyses(created_utc);
";

/// Version 4: idle reflection runs (see `reflect`). One row per run; the job is a stub until R6.
pub const CREATE_V4: &str = "
CREATE TABLE reflections (
    id TEXT PRIMARY KEY,
    started_utc TEXT,
    ended_utc TEXT,
    outcome TEXT,
    stages INTEGER
);
";

/// Version 5: how many boxes the quality guard dropped from an analysis (see `analysis::boxes`).
pub const CREATE_V5: &str = "
ALTER TABLE analyses ADD COLUMN boxes_rejected INTEGER;
";

/// Version 6: every model attempt (`calls`, with redacted request and response bodies) and run
/// counters filled when a run ends. `calls.run_id` has no foreign key: analyses use their session id.
pub const CREATE_V6: &str = "
CREATE TABLE calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    step INTEGER,
    kind TEXT,
    attempt INTEGER,
    started_utc TEXT,
    duration_ms INTEGER,
    wait_before_ms INTEGER,
    status INTEGER,
    outcome TEXT,
    error TEXT,
    request_json TEXT,
    response_json TEXT,
    truncated INTEGER,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER
);
CREATE INDEX calls_run ON calls(run_id, step);
ALTER TABLE runs ADD COLUMN step_count INTEGER;
ALTER TABLE runs ADD COLUMN retry_count INTEGER;
ALTER TABLE runs ADD COLUMN duration_ms INTEGER;
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
    db.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
    )?;
    // One transaction per step, so a crash cannot leave half a schema behind.
    let mut batch = String::new();
    for (version, ddl) in [
        (1, CREATE_V1),
        (2, CREATE_V2),
        (3, CREATE_V3),
        (4, CREATE_V4),
        (5, CREATE_V5),
        (6, CREATE_V6),
    ] {
        if found < version {
            batch.push_str(ddl);
        }
    }
    if !batch.is_empty() {
        db.execute_batch(&format!(
            "BEGIN; {batch} PRAGMA user_version = {VERSION}; COMMIT;"
        ))?;
    }
    Ok(Opened::Ready)
}
