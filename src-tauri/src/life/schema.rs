use rusqlite::Connection;

use super::LifeError;

pub const VERSION: i64 = 1;

pub const CREATE_V1: &str = "
CREATE TABLE IF NOT EXISTS meta (
    version INTEGER PRIMARY KEY,
    created_utc TEXT NOT NULL,
    key_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS consent (
    class TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0,
    changed_utc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS records (
    id TEXT PRIMARY KEY,
    class TEXT NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    source TEXT NOT NULL,
    grade INTEGER NOT NULL,
    occurred_utc TEXT NOT NULL,
    created_utc TEXT NOT NULL,
    supports_json TEXT,
    deleted TEXT
);

CREATE INDEX IF NOT EXISTS idx_records_class ON records(class);
CREATE INDEX IF NOT EXISTS idx_records_occurred ON records(occurred_utc);
CREATE INDEX IF NOT EXISTS idx_records_deleted ON records(deleted);

CREATE TABLE IF NOT EXISTS assumptions (
    id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    class TEXT NOT NULL,
    status TEXT NOT NULL,
    evidence_json TEXT,
    created_utc TEXT NOT NULL,
    changed_utc TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assumptions_class ON assumptions(class);

CREATE TABLE IF NOT EXISTS plans (
    id TEXT PRIMARY KEY,
    current_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS plan_versions (
    id TEXT NOT NULL REFERENCES plans(id),
    version INTEGER NOT NULL,
    body_json TEXT NOT NULL,
    reason TEXT,
    evidence_json TEXT,
    created_utc TEXT NOT NULL,
    approved INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (id, version)
);

CREATE TABLE IF NOT EXISTS audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at_utc TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    subject TEXT NOT NULL,
    detail TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_at ON audit(at_utc);
CREATE INDEX IF NOT EXISTS idx_audit_subject ON audit(subject);
";

#[derive(Debug, PartialEq, Eq)]
pub enum Opened {
    Ready,
    Newer(i64),
}

pub fn open_or_create(db: &mut Connection, key_id: &str) -> Result<Opened, LifeError> {
    db.busy_timeout(std::time::Duration::from_millis(5000))?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
    )?;

    let found: i64 = db.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > VERSION {
        return Ok(Opened::Newer(found));
    }

    if found == 0 {
        let now = now_utc();
        let tx = db.transaction()?;
        tx.execute_batch(CREATE_V1)?;

        tx.execute(
            "INSERT INTO meta (version, created_utc, key_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![VERSION, now, key_id],
        )?;

        // Default: all classes consent OFF
        for class in ["general", "health", "mind", "finance"] {
            tx.execute(
                "INSERT OR IGNORE INTO consent (class, enabled, changed_utc) VALUES (?1, 0, ?2)",
                rusqlite::params![class, now],
            )?;
        }

        tx.execute_batch(&format!("PRAGMA user_version = {VERSION};"))?;
        tx.commit()?;
    }

    Ok(Opened::Ready)
}

fn now_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let s = rem_secs % 60;
    format!("D{days}T{hours:02}:{mins:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_creates_v1_schema_and_defaults() {
        let mut conn = Connection::open_in_memory().unwrap();
        let opened = open_or_create(&mut conn, "test-key-id").unwrap();
        assert_eq!(opened, Opened::Ready);

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);

        let meta_ver: i64 = conn
            .query_row("SELECT version FROM meta", [], |r| r.get(0))
            .unwrap();
        assert_eq!(meta_ver, 1);

        let consent_count: i64 = conn
            .query_row("SELECT count(*) FROM consent", [], |r| r.get(0))
            .unwrap();
        assert_eq!(consent_count, 4);

        // Opening twice is idempotent
        let second_open = open_or_create(&mut conn, "test-key-id").unwrap();
        assert_eq!(second_open, Opened::Ready);
    }

    #[test]
    fn newer_schema_is_left_untouched() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA user_version = 42;", []).unwrap();

        let opened = open_or_create(&mut conn, "test-key-id").unwrap();
        assert_eq!(opened, Opened::Newer(42));
    }
}
