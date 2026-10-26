//! Durable product identities backed by existing stored execution records.
use std::path::Path;

use rusqlite::OptionalExtension;

use crate::trace::store::{self, StoreError};
pub static MUTATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn has_attempts(conn: &rusqlite::Connection) -> Result<bool, StoreError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='task_attempts')",
        [],
        |r| r.get(0),
    )?)
}

pub fn attempts(db: &Path, task: &str) -> Result<Vec<String>, StoreError> {
    let conn = store::open_read(db)?;
    if !has_attempts(&conn)? {
        return Ok(vec![task.into()]);
    }
    let mut query = conn.prepare("SELECT id FROM runs WHERE id=?1 OR id IN (SELECT run_id FROM task_attempts WHERE task_id=?1) ORDER BY started_utc, rowid")?;
    Ok(query
        .query_map([task], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?)
}

pub fn link_attempt(db: &Path, task: &str, run: &str) -> Result<(), StoreError> {
    let conn = rusqlite::Connection::open(db)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1)",
        [task],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(StoreError::Sqlite(
            "The parent task no longer exists".into(),
        ));
    }
    conn.execute(
        "INSERT OR IGNORE INTO task_attempts(run_id,task_id) VALUES (?1,?2)",
        [run, task],
    )?;
    let owner: String = conn.query_row(
        "SELECT task_id FROM task_attempts WHERE run_id=?1",
        [run],
        |r| r.get(0),
    )?;
    if owner != task {
        return Err(StoreError::Sqlite(
            "This attempt already belongs to another task".into(),
        ));
    }
    Ok(())
}

/// An execution reference resolves to its owning task; legacy tasks retain their ID.
pub fn task_id(db: &Path, execution_id: &str) -> Result<String, StoreError> {
    let conn = store::open_read(db)?;
    let linked: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='task_links')",
        [],
        |row| row.get(0),
    )?;
    if !linked {
        return Ok(execution_id.into());
    }
    let root = conn.query_row(
        "SELECT parent.id FROM task_links l JOIN runs parent ON parent.id=l.request_id WHERE l.run_id=?1",
        [execution_id],
        |row| row.get::<_, String>(0),
    ).optional()?.unwrap_or_else(|| execution_id.into());
    if !has_attempts(&conn)? {
        return Ok(root);
    }
    Ok(conn
        .query_row(
            "SELECT task_id FROM task_attempts WHERE run_id=?1",
            [&root],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(root))
}
