//! Durable product identities backed by existing stored execution records.
use std::path::Path;

use rusqlite::OptionalExtension;

use crate::trace::store::{self, StoreError};

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
    Ok(conn.query_row(
        "SELECT parent.id FROM task_links l JOIN runs parent ON parent.id=l.request_id WHERE l.run_id=?1",
        [execution_id],
        |row| row.get::<_, String>(0),
    ).optional()?.unwrap_or_else(|| execution_id.into()))
}
