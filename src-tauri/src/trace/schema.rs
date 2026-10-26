use rusqlite::Connection;

pub const VERSION: i64 = 10;

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

pub const CREATE_V4: &str = "
CREATE TABLE reflections (
    id TEXT PRIMARY KEY,
    started_utc TEXT,
    ended_utc TEXT,
    outcome TEXT,
    stages INTEGER
);
";

pub const CREATE_V5: &str = "
ALTER TABLE analyses ADD COLUMN boxes_rejected INTEGER;
";

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

const CREATE_V7: &str = "
ALTER TABLE calls ADD COLUMN served_model TEXT;
ALTER TABLE calls ADD COLUMN swap_ms INTEGER;
ALTER TABLE steps ADD COLUMN served_model TEXT;
";

#[derive(Debug, PartialEq, Eq)]
pub enum Opened {
    Ready,
    Newer(i64),
}

pub fn open(db: &Connection) -> rusqlite::Result<Opened> {
    let found: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > VERSION {
        return Ok(Opened::Newer(found));
    }
    db.busy_timeout(std::time::Duration::from_millis(5000))?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
    )?;
    let mut batch = String::new();
    for (version, ddl) in [
        (1, CREATE_V1),
        (2, CREATE_V2),
        (3, CREATE_V3),
        (4, CREATE_V4),
        (5, CREATE_V5),
        (6, CREATE_V6),
        (7, CREATE_V7),
        (8, "ALTER TABLE calls ADD COLUMN locality TEXT;"),
        (
            9,
            "CREATE TABLE IF NOT EXISTS task_history (request_id TEXT NOT NULL, revision INTEGER NOT NULL, at_ms INTEGER NOT NULL, snapshot_json TEXT NOT NULL, PRIMARY KEY(request_id, revision)); CREATE TABLE IF NOT EXISTS task_links (run_id TEXT PRIMARY KEY, request_id TEXT NOT NULL, task_id INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS task_links_request ON task_links(request_id);",
        ),
        (
            10,
            "CREATE TABLE IF NOT EXISTS task_attempts (run_id TEXT PRIMARY KEY, task_id TEXT NOT NULL); CREATE INDEX task_attempts_task ON task_attempts(task_id);",
        ),
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
