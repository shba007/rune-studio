use rusqlite::{Connection, OpenFlags, Row, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::LifeError;
use super::crypto::LifeCrypto;
use super::record::{
    Assumption, AssumptionStatus, AuditEntry, DataClass, NewRecord, Record, RecordKind,
    RecordSource, validate_source_grade,
};
use super::schema::{self, Opened};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSummary {
    pub records_count: usize,
    pub assumptions_count: usize,
    pub files_written: Vec<String>,
}

pub struct LifeStore {
    conn: Mutex<Connection>,
    crypto: LifeCrypto,
    path: Option<PathBuf>,
}

impl LifeStore {
    /// Open or create a life database at the given path.
    pub fn open(path: &Path, crypto: LifeCrypto) -> Result<Self, LifeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        match schema::open_or_create(&mut conn, crypto.key_id())? {
            Opened::Ready => Ok(Self {
                conn: Mutex::new(conn),
                crypto,
                path: Some(path.to_path_buf()),
            }),
            Opened::Newer(v) => Err(LifeError::NewerVersion(v)),
        }
    }

    /// Recovery action when encryption key is lost:
    /// Re-initializes the life store from scratch ONLY if explicit confirmation "START FRESH" is provided.
    pub fn reset_start_fresh(path: &Path, confirmation: &str) -> Result<Self, LifeError> {
        if confirmation != "START FRESH" {
            return Err(LifeError::Crypto(
                "Reset refused: requires confirmation 'START FRESH'".to_string(),
            ));
        }

        // Delete old keyring credential
        LifeCrypto::delete_keyring_key()?;

        // Remove old database files
        if path.exists() {
            fs::remove_file(path)?;
        }
        let wal = path.with_extension("sqlite3-wal");
        if wal.exists() {
            let _ = fs::remove_file(wal);
        }
        let shm = path.with_extension("sqlite3-shm");
        if shm.exists() {
            let _ = fs::remove_file(shm);
        }

        // Create fresh crypto and fresh store
        let crypto = LifeCrypto::from_keyring_or_create()?;
        Self::open(path, crypto)
    }

    /// Open an in-memory life database with the provided crypto.
    pub fn in_memory(crypto: LifeCrypto) -> Result<Self, LifeError> {
        let mut conn = Connection::open_in_memory()?;
        match schema::open_or_create(&mut conn, crypto.key_id())? {
            Opened::Ready => Ok(Self {
                conn: Mutex::new(conn),
                crypto,
                path: None,
            }),
            Opened::Newer(v) => Err(LifeError::NewerVersion(v)),
        }
    }

    /// Open an in-memory life database with random test crypto.
    pub fn in_memory_random() -> Result<Self, LifeError> {
        Self::in_memory(LifeCrypto::in_memory_random())
    }

    pub fn crypto(&self) -> &LifeCrypto {
        &self.crypto
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    // --- Consent Management ---

    pub fn get_consent(&self, class: DataClass) -> Result<bool, LifeError> {
        let conn = self.conn.lock().unwrap();
        let enabled: i64 = conn
            .query_row(
                "SELECT enabled FROM consent WHERE class = ?1",
                params![class.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(enabled != 0)
    }

    pub fn set_consent(
        &self,
        class: DataClass,
        enabled: bool,
        actor: &str,
    ) -> Result<(), LifeError> {
        let conn = self.conn.lock().unwrap();
        let now = now_utc();
        let val = if enabled { 1 } else { 0 };
        conn.execute(
            "INSERT INTO consent (class, enabled, changed_utc) VALUES (?1, ?2, ?3)
             ON CONFLICT(class) DO UPDATE SET enabled = excluded.enabled, changed_utc = excluded.changed_utc",
            params![class.as_str(), val, now],
        )?;

        self.log_audit_locked(
            &conn,
            &now,
            actor,
            "set_consent",
            &format!("class:{}", class.as_str()),
            Some(&format!("enabled={}", enabled)),
        )?;
        Ok(())
    }

    pub fn list_consents(&self) -> Result<HashMap<DataClass, bool>, LifeError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT class, enabled FROM consent")?;
        let rows = stmt.query_map([], |row| {
            let class_str: String = row.get(0)?;
            let enabled: i64 = row.get(1)?;
            Ok((class_str, enabled != 0))
        })?;

        let mut map = HashMap::new();
        for item in rows {
            let (class_str, val) = item?;
            if let Ok(class) = class_str.parse::<DataClass>() {
                map.insert(class, val);
            }
        }
        Ok(map)
    }

    // --- Records ---

    pub fn write_record(&self, new_record: NewRecord, actor: &str) -> Result<Record, LifeError> {
        validate_source_grade(&new_record.source, new_record.grade)?;

        let conn = self.conn.lock().unwrap();

        // 1. Consent check: write refused if consent is off
        let consent_enabled: i64 = conn
            .query_row(
                "SELECT enabled FROM consent WHERE class = ?1",
                params![new_record.class.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if consent_enabled == 0 {
            return Err(LifeError::ClassConsentDenied(new_record.class));
        }

        let now = now_utc();
        let id = new_record.id.unwrap_or_else(|| generate_id("rec"));
        let occurred_utc = new_record.occurred_utc.unwrap_or_else(|| now.clone());

        // 2. Encrypt body if sensitive class
        let stored_body = if new_record.class.is_sensitive() {
            self.crypto.encrypt(&new_record.body)?
        } else {
            new_record.body.clone()
        };

        conn.execute(
            "INSERT INTO records (id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json, deleted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)",
            params![
                id,
                new_record.class.as_str(),
                new_record.kind.as_str(),
                stored_body,
                new_record.source.as_str(),
                new_record.grade,
                occurred_utc,
                now,
                new_record.supports_json,
            ],
        )?;

        // 3. Audit log (never log content of record body!)
        let audit_detail = format!(
            "class={}, kind={}, grade={}, source={}",
            new_record.class.as_str(),
            new_record.kind.as_str(),
            new_record.grade,
            new_record.source.as_str()
        );
        self.log_audit_locked(
            &conn,
            &now,
            actor,
            "write_record",
            &format!("record:{}", id),
            Some(&audit_detail),
        )?;

        Ok(Record {
            id,
            class: new_record.class,
            kind: new_record.kind,
            body: new_record.body, // return plaintext to caller
            source: new_record.source,
            grade: new_record.grade,
            occurred_utc,
            created_utc: now,
            supports_json: new_record.supports_json,
            deleted: None,
        })
    }

    pub fn get_record(&self, id: &str) -> Result<Option<Record>, LifeError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json, deleted
             FROM records WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let record = self.row_to_record(row)?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    pub fn list_records(
        &self,
        class: Option<DataClass>,
        include_deleted: bool,
    ) -> Result<Vec<Record>, LifeError> {
        let conn = self.conn.lock().unwrap();
        let sql = match (class, include_deleted) {
            (Some(c), false) => format!(
                "SELECT id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json, deleted
                 FROM records WHERE class = '{}' AND deleted IS NULL ORDER BY occurred_utc DESC",
                c.as_str()
            ),
            (Some(c), true) => format!(
                "SELECT id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json, deleted
                 FROM records WHERE class = '{}' ORDER BY occurred_utc DESC",
                c.as_str()
            ),
            (None, false) => {
                "SELECT id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json, deleted
                 FROM records WHERE deleted IS NULL ORDER BY occurred_utc DESC"
                    .to_string()
            }
            (None, true) => {
                "SELECT id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json, deleted
                 FROM records ORDER BY occurred_utc DESC"
                    .to_string()
            }
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| self.row_to_record_result(row))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn soft_delete_record(&self, id: &str, actor: &str) -> Result<bool, LifeError> {
        let conn = self.conn.lock().unwrap();
        let now = now_utc();
        let affected = conn.execute(
            "UPDATE records SET deleted = ?1 WHERE id = ?2 AND deleted IS NULL",
            params![now, id],
        )?;

        if affected > 0 {
            self.log_audit_locked(
                &conn,
                &now,
                actor,
                "soft_delete_record",
                &format!("record:{}", id),
                None,
            )?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- Retention & Hard Deletion ---

    /// Delete records older than cutoff_utc for the given class, and prune related audit entries.
    pub fn purge_retention(
        &self,
        class: DataClass,
        cutoff_utc: &str,
        actor: &str,
    ) -> Result<usize, LifeError> {
        let conn = self.conn.lock().unwrap();
        let now = now_utc();

        let tx = conn.unchecked_transaction()?;

        // Delete audit records pointing to records being purged
        tx.execute(
            "DELETE FROM audit WHERE subject IN (
                SELECT 'record:' || id FROM records WHERE class = ?1 AND occurred_utc < ?2
             )",
            params![class.as_str(), cutoff_utc],
        )?;

        let count = tx.execute(
            "DELETE FROM records WHERE class = ?1 AND occurred_utc < ?2",
            params![class.as_str(), cutoff_utc],
        )?;

        tx.commit()?;

        self.log_audit_locked(
            &conn,
            &now,
            actor,
            "retention_purge",
            &format!("class:{}", class.as_str()),
            Some(&format!("deleted_count={}", count)),
        )?;

        self.checkpoint_and_vacuum_locked(&conn)?;
        Ok(count)
    }

    /// Permanently delete a class (or all classes), including tombstones, audit rows referencing content,
    /// and vacuum the SQLite database so no data remains on disk.
    pub fn hard_delete_class(
        &self,
        class: Option<DataClass>,
        actor: &str,
    ) -> Result<usize, LifeError> {
        let conn = self.conn.lock().unwrap();
        let now = now_utc();

        let tx = conn.unchecked_transaction()?;
        let count = match class {
            Some(c) => {
                // Delete audit entries about records of this class
                tx.execute(
                    "DELETE FROM audit WHERE subject IN (
                        SELECT 'record:' || id FROM records WHERE class = ?1
                     )",
                    params![c.as_str()],
                )?;
                tx.execute(
                    "DELETE FROM assumptions WHERE class = ?1",
                    params![c.as_str()],
                )?;
                tx.execute("DELETE FROM records WHERE class = ?1", params![c.as_str()])?
            }
            None => {
                // Delete all records, assumptions, plans, plan_versions, and record audit rows
                tx.execute("DELETE FROM audit WHERE subject LIKE 'record:%' OR subject LIKE 'assumption:%'", [])?;
                tx.execute("DELETE FROM assumptions", [])?;
                tx.execute("DELETE FROM plan_versions", [])?;
                tx.execute("DELETE FROM plans", [])?;
                tx.execute("DELETE FROM records", [])?
            }
        };
        tx.commit()?;

        let class_name = class
            .map(|c| c.as_str().to_string())
            .unwrap_or_else(|| "all".to_string());
        self.log_audit_locked(
            &conn,
            &now,
            actor,
            "hard_delete",
            &format!("class:{}", class_name),
            Some(&format!("purged_records={}", count)),
        )?;

        self.checkpoint_and_vacuum_locked(&conn)?;
        Ok(count)
    }

    /// Checkpoint WAL and VACUUM the database file.
    pub fn checkpoint_and_vacuum(&self) -> Result<(), LifeError> {
        let conn = self.conn.lock().unwrap();
        self.checkpoint_and_vacuum_locked(&conn)
    }

    fn checkpoint_and_vacuum_locked(&self, conn: &Connection) -> Result<(), LifeError> {
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        conn.execute_batch("VACUUM;")?;
        Ok(())
    }

    // --- Export ---

    pub fn export_class(
        &self,
        class: Option<DataClass>,
        target_dir: &Path,
    ) -> Result<ExportSummary, LifeError> {
        fs::create_dir_all(target_dir)?;

        let records = self.list_records(class, false)?;
        let assumptions = self.list_assumptions(class)?;

        let mut files_written = Vec::new();

        // 1. JSON export of records
        let records_json =
            serde_json::to_string_pretty(&records).map_err(|e| LifeError::Io(e.to_string()))?;
        let records_json_path = target_dir.join("records.json");
        fs::write(&records_json_path, records_json)?;
        files_written.push("records.json".to_string());

        // 2. Markdown export per kind
        let mut kinds_map: HashMap<RecordKind, Vec<&Record>> = HashMap::new();
        for r in &records {
            kinds_map.entry(r.kind).or_default().push(r);
        }

        for (kind, recs) in kinds_map {
            let md_filename = format!("{}.md", kind.as_str());
            let mut md = String::new();
            md.push_str(&format!("# {}\n\n", kind.as_str().to_uppercase()));
            for r in recs {
                md.push_str(&format!(
                    "### [{}] Grade E{} ({})\n- **Date**: {}\n- **Class**: {}\n- **Source**: {}\n\n{}\n\n---\n\n",
                    r.id, r.grade, r.kind, r.occurred_utc, r.class, r.source, r.body
                ));
            }
            fs::write(target_dir.join(&md_filename), md)?;
            files_written.push(md_filename);
        }

        // 3. Assumptions export
        if !assumptions.is_empty() {
            let assumptions_json = serde_json::to_string_pretty(&assumptions)
                .map_err(|e| LifeError::Io(e.to_string()))?;
            fs::write(target_dir.join("assumptions.json"), assumptions_json)?;
            files_written.push("assumptions.json".to_string());

            let mut md = String::from("# ASSUMPTIONS\n\n");
            for a in &assumptions {
                md.push_str(&format!(
                    "### [{}] Status: {}\n- **Class**: {}\n- **Created**: {}\n\n{}\n\n---\n\n",
                    a.id, a.status, a.class, a.created_utc, a.text
                ));
            }
            fs::write(target_dir.join("assumptions.md"), md)?;
            files_written.push("assumptions.md".to_string());
        }

        Ok(ExportSummary {
            records_count: records.len(),
            assumptions_count: assumptions.len(),
            files_written,
        })
    }

    // --- Assumptions ---

    pub fn write_assumption(&self, assumption: Assumption, actor: &str) -> Result<(), LifeError> {
        let conn = self.conn.lock().unwrap();
        let now = now_utc();
        let stored_text = if assumption.class.is_sensitive() {
            self.crypto.encrypt(&assumption.text)?
        } else {
            assumption.text.clone()
        };

        conn.execute(
            "INSERT INTO assumptions (id, text, class, status, evidence_json, created_utc, changed_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET text = excluded.text, status = excluded.status,
                evidence_json = excluded.evidence_json, changed_utc = excluded.changed_utc",
            params![
                assumption.id,
                stored_body_or_text(&stored_text),
                assumption.class.as_str(),
                assumption.status.as_str(),
                assumption.evidence_json,
                assumption.created_utc,
                now,
            ],
        )?;

        self.log_audit_locked(
            &conn,
            &now,
            actor,
            "write_assumption",
            &format!("assumption:{}", assumption.id),
            Some(&format!(
                "class={}, status={}",
                assumption.class, assumption.status
            )),
        )?;
        Ok(())
    }

    pub fn get_assumption(&self, id: &str) -> Result<Option<Assumption>, LifeError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, text, class, status, evidence_json, created_utc, changed_utc
             FROM assumptions WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let class_str: String = row.get(2)?;
            let class = class_str.parse::<DataClass>()?;
            let status_str: String = row.get(3)?;
            let status = status_str.parse::<AssumptionStatus>()?;
            let text_raw: String = row.get(1)?;
            let text = self.crypto.decrypt(&text_raw)?;

            Ok(Some(Assumption {
                id: row.get(0)?,
                text,
                class,
                status,
                evidence_json: row.get(4)?,
                created_utc: row.get(5)?,
                changed_utc: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_assumptions(&self, class: Option<DataClass>) -> Result<Vec<Assumption>, LifeError> {
        let conn = self.conn.lock().unwrap();
        let sql = match class {
            Some(c) => format!(
                "SELECT id, text, class, status, evidence_json, created_utc, changed_utc
                 FROM assumptions WHERE class = '{}' ORDER BY changed_utc DESC",
                c.as_str()
            ),
            None => "SELECT id, text, class, status, evidence_json, created_utc, changed_utc
                 FROM assumptions ORDER BY changed_utc DESC"
                .to_string(),
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let text_raw: String = row.get(1)?;
            let class_str: String = row.get(2)?;
            let status_str: String = row.get(3)?;
            let evidence_json: Option<String> = row.get(4)?;
            let created_utc: String = row.get(5)?;
            let changed_utc: String = row.get(6)?;
            Ok((
                id,
                text_raw,
                class_str,
                status_str,
                evidence_json,
                created_utc,
                changed_utc,
            ))
        })?;

        let mut list = Vec::new();
        for item in rows {
            let (id, text_raw, class_str, status_str, evidence_json, created_utc, changed_utc) =
                item?;
            let class = class_str.parse::<DataClass>()?;
            let status = status_str.parse::<AssumptionStatus>()?;
            let text = self.crypto.decrypt(&text_raw)?;
            list.push(Assumption {
                id,
                text,
                class,
                status,
                evidence_json,
                created_utc,
                changed_utc,
            });
        }
        Ok(list)
    }

    // --- Audit ---

    pub fn list_audit(&self, limit: usize) -> Result<Vec<AuditEntry>, LifeError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, at_utc, actor, action, subject, detail
             FROM audit ORDER BY id DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                at_utc: row.get(1)?,
                actor: row.get(2)?,
                action: row.get(3)?,
                subject: row.get(4)?,
                detail: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }

    fn log_audit_locked(
        &self,
        conn: &Connection,
        at_utc: &str,
        actor: &str,
        action: &str,
        subject: &str,
        detail: Option<&str>,
    ) -> Result<(), LifeError> {
        conn.execute(
            "INSERT INTO audit (at_utc, actor, action, subject, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![at_utc, actor, action, subject, detail],
        )?;
        Ok(())
    }

    // --- Helpers ---

    fn row_to_record(&self, row: &Row<'_>) -> Result<Record, LifeError> {
        let class_str: String = row.get(1)?;
        let kind_str: String = row.get(2)?;
        let body_raw: String = row.get(3)?;
        let source_str: String = row.get(4)?;

        let class = class_str.parse::<DataClass>()?;
        let kind = kind_str.parse::<RecordKind>()?;
        let source = source_str.parse::<RecordSource>()?;
        let body = self.crypto.decrypt(&body_raw)?;

        Ok(Record {
            id: row.get(0)?,
            class,
            kind,
            body,
            source,
            grade: row.get(5)?,
            occurred_utc: row.get(6)?,
            created_utc: row.get(7)?,
            supports_json: row.get(8)?,
            deleted: row.get(9)?,
        })
    }

    fn row_to_record_result(&self, row: &Row<'_>) -> rusqlite::Result<Record> {
        self.row_to_record(row).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
    }
}

fn stored_body_or_text(s: &str) -> &str {
    s
}

fn now_utc() -> String {
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

fn generate_id(prefix: &str) -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = dur.as_nanos();
    format!("{prefix}-{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("rune-life-test-{}", generate_id("d")));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn consent_rejection_and_granting() {
        let store = LifeStore::in_memory_random().unwrap();

        // 1. Consent is off by default for Health
        let res = store.write_record(
            NewRecord {
                id: None,
                class: DataClass::Health,
                kind: RecordKind::Fact,
                body: "Resting heart rate 62 bpm".to_string(),
                source: RecordSource::Tool,
                grade: 2,
                occurred_utc: None,
                supports_json: None,
            },
            "test_user",
        );
        assert_eq!(
            res.unwrap_err(),
            LifeError::ClassConsentDenied(DataClass::Health)
        );

        // 2. Grant consent for Health
        store
            .set_consent(DataClass::Health, true, "test_user")
            .unwrap();
        assert!(store.get_consent(DataClass::Health).unwrap());

        // 3. Now write succeeds
        let rec = store
            .write_record(
                NewRecord {
                    id: None,
                    class: DataClass::Health,
                    kind: RecordKind::Fact,
                    body: "Resting heart rate 62 bpm".to_string(),
                    source: RecordSource::Tool,
                    grade: 2,
                    occurred_utc: None,
                    supports_json: None,
                },
                "test_user",
            )
            .unwrap();

        assert_eq!(rec.body, "Resting heart rate 62 bpm");
        assert_eq!(rec.class, DataClass::Health);

        // 4. Retrieve back
        let fetched = store.get_record(&rec.id).unwrap().unwrap();
        assert_eq!(fetched.body, "Resting heart rate 62 bpm");
    }

    #[test]
    fn encrypted_on_disk_canary_test() {
        let temp_dir = TestDir::new();
        let db_path = temp_dir.path().join("life.sqlite3");
        let crypto = LifeCrypto::in_memory_random();
        let store = LifeStore::open(&db_path, crypto).unwrap();

        store
            .set_consent(DataClass::Finance, true, "test_user")
            .unwrap();

        let canary = "CANARY_SECRET_BANK_BALANCE_987654";
        let rec = store
            .write_record(
                NewRecord {
                    id: Some("canary-rec-1".to_string()),
                    class: DataClass::Finance,
                    kind: RecordKind::Fact,
                    body: canary.to_string(),
                    source: RecordSource::Tool,
                    grade: 2,
                    occurred_utc: None,
                    supports_json: None,
                },
                "test_user",
            )
            .unwrap();

        assert_eq!(rec.body, canary);

        // Checkpoint WAL and close DB
        store.checkpoint_and_vacuum().unwrap();
        drop(store);

        // Read raw disk bytes of SQLite file and WAL
        let db_bytes = fs::read(&db_path).unwrap();
        let wal_path = temp_dir.path().join("life.sqlite3-wal");
        let wal_bytes = if wal_path.exists() {
            fs::read(&wal_path).unwrap()
        } else {
            Vec::new()
        };

        let canary_bytes = canary.as_bytes();
        let found_in_db = db_bytes
            .windows(canary_bytes.len())
            .any(|w| w == canary_bytes);
        let found_in_wal = wal_bytes
            .windows(canary_bytes.len())
            .any(|w| w == canary_bytes);

        assert!(
            !found_in_db,
            "Canary string must NOT be found in SQLite database bytes!"
        );
        assert!(
            !found_in_wal,
            "Canary string must NOT be found in SQLite WAL bytes!"
        );
    }

    #[test]
    fn hard_delete_clears_bytes_canary_test() {
        let temp_dir = TestDir::new();
        let db_path = temp_dir.path().join("life.sqlite3");
        let crypto = LifeCrypto::in_memory_random();
        let store = LifeStore::open(&db_path, crypto).unwrap();

        store
            .set_consent(DataClass::General, true, "test_user")
            .unwrap();

        // Write plaintext general record with canary
        let canary = "GENERAL_PURGED_CANARY_ABCD1234XYZ";
        store
            .write_record(
                NewRecord {
                    id: Some("general-rec-1".to_string()),
                    class: DataClass::General,
                    kind: RecordKind::Note,
                    body: canary.to_string(),
                    source: RecordSource::UserSaid,
                    grade: 0,
                    occurred_utc: None,
                    supports_json: None,
                },
                "test_user",
            )
            .unwrap();

        // Perform hard delete of General class
        let deleted = store
            .hard_delete_class(Some(DataClass::General), "test_user")
            .unwrap();
        assert_eq!(deleted, 1);

        drop(store);

        let db_bytes = fs::read(&db_path).unwrap();
        let wal_path = temp_dir.path().join("life.sqlite3-wal");
        let wal_bytes = if wal_path.exists() {
            fs::read(&wal_path).unwrap()
        } else {
            Vec::new()
        };

        let canary_bytes = canary.as_bytes();
        assert!(
            !db_bytes
                .windows(canary_bytes.len())
                .any(|w| w == canary_bytes),
            "Canary string must be completely gone after hard delete and vacuum!"
        );
        assert!(
            !wal_bytes
                .windows(canary_bytes.len())
                .any(|w| w == canary_bytes),
            "Canary string must be gone from WAL after checkpoint and vacuum!"
        );
    }

    #[test]
    fn export_generates_json_and_markdown() {
        let store = LifeStore::in_memory_random().unwrap();
        store
            .set_consent(DataClass::Mind, true, "test_user")
            .unwrap();

        store
            .write_record(
                NewRecord {
                    id: Some("mind-rec-1".to_string()),
                    class: DataClass::Mind,
                    kind: RecordKind::Note,
                    body: "Focused deeply on coding for 2 hours".to_string(),
                    source: RecordSource::UserSaid,
                    grade: 1,
                    occurred_utc: Some("2026-10-21T10:00:00Z".to_string()),
                    supports_json: None,
                },
                "test_user",
            )
            .unwrap();

        let temp_dir = TestDir::new();
        let summary = store
            .export_class(Some(DataClass::Mind), temp_dir.path())
            .unwrap();

        assert_eq!(summary.records_count, 1);
        assert!(temp_dir.path().join("records.json").exists());
        assert!(temp_dir.path().join("note.md").exists());

        let md_content = fs::read_to_string(temp_dir.path().join("note.md")).unwrap();
        assert!(md_content.contains("Focused deeply on coding for 2 hours"));
        assert!(md_content.contains("Grade E1"));
    }

    #[test]
    fn benchmark_10000_records_write_and_read() {
        let temp_dir = TestDir::new();
        let db_path = temp_dir.path().join("bench_life.sqlite3");
        let crypto = LifeCrypto::in_memory_random();
        let store = LifeStore::open(&db_path, crypto).unwrap();

        store.set_consent(DataClass::Health, true, "bench").unwrap();

        let count = 10_000;
        let start_write = std::time::Instant::now();

        {
            let conn = store.conn.lock().unwrap();
            let tx = conn.unchecked_transaction().unwrap();
            for i in 0..count {
                let body = format!("Daily step count benchmark entry #{i} with timestamp data");
                let enc_body = store.crypto.encrypt(&body).unwrap();
                tx.execute(
                    "INSERT INTO records (id, class, kind, body, source, grade, occurred_utc, created_utc, supports_json)
                     VALUES (?1, 'health', 'fact', ?2, 'tool', 2, '2026-10-21T00:00:00Z', '2026-10-21T00:00:00Z', NULL)",
                    params![format!("rec-{i}"), enc_body],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        let write_duration = start_write.elapsed();

        let start_read = std::time::Instant::now();
        let records = store.list_records(Some(DataClass::Health), false).unwrap();
        let read_duration = start_read.elapsed();

        assert_eq!(records.len(), count);
        println!(
            "Spike benchmark 10,000 records AES-256-GCM + SQLite: Write: {:?}, Read (decrypted): {:?}",
            write_duration, read_duration
        );
    }

    #[test]
    fn backup_file_without_key_is_useless() {
        let temp_dir = TestDir::new();
        let db_path = temp_dir.path().join("original_life.sqlite3");
        let crypto1 = LifeCrypto::in_memory_random();
        let store = LifeStore::open(&db_path, crypto1).unwrap();

        store.set_consent(DataClass::Health, true, "test").unwrap();
        store
            .write_record(
                NewRecord {
                    id: Some("health-1".into()),
                    class: DataClass::Health,
                    kind: RecordKind::Fact,
                    body: "confidential patient diagnosis".into(),
                    source: RecordSource::Tool,
                    grade: 2,
                    occurred_utc: None,
                    supports_json: None,
                },
                "test",
            )
            .unwrap();

        store.checkpoint_and_vacuum().unwrap();
        drop(store);

        // Copy database to backup location
        let backup_path = temp_dir.path().join("backup_life.sqlite3");
        fs::copy(&db_path, &backup_path).unwrap();

        // Attempt to open backup with DIFFERENT crypto key (simulating thief or missing keyring)
        let crypto_other = LifeCrypto::in_memory_random();
        let backup_store = LifeStore::open(&backup_path, crypto_other).unwrap();

        // Attempting to list/read records fails to decrypt
        let result = backup_store.list_records(Some(DataClass::Health), false);
        assert!(
            result.is_err(),
            "Decryption must fail without the original key!"
        );
    }

    #[test]
    fn key_loss_and_start_fresh_recovery() {
        let temp_dir = TestDir::new();
        let db_path = temp_dir.path().join("recovery_life.sqlite3");

        LifeCrypto::delete_keyring_key().unwrap();
        let crypto = LifeCrypto::from_keyring_or_create().unwrap();
        let ciphertext = crypto.encrypt("old private record").unwrap();
        let store = LifeStore::open(&db_path, crypto).unwrap();
        store.set_consent(DataClass::Health, true, "test").unwrap();
        drop(store);

        // Refusing confirmation preserves the key and existing store.
        let fail = LifeStore::reset_start_fresh(&db_path, "no");
        assert!(fail.is_err());
        assert_eq!(
            LifeCrypto::from_keyring_existing()
                .unwrap()
                .decrypt(&ciphertext)
                .unwrap(),
            "old private record"
        );
        assert!(db_path.exists());

        // Simulate actual key loss in the in-memory test keyring.
        LifeCrypto::delete_keyring_key().unwrap();
        assert!(matches!(
            LifeCrypto::from_keyring_existing(),
            Err(super::super::crypto::CryptoError::KeyUnavailable(_))
        ));

        // With explicit confirmation, succeeds
        let fresh_store = LifeStore::reset_start_fresh(&db_path, "START FRESH").unwrap();
        assert_eq!(fresh_store.list_records(None, false).unwrap().len(), 0);
        assert!(!fresh_store.get_consent(DataClass::Health).unwrap());
        assert!(fresh_store.crypto().decrypt(&ciphertext).is_err());
        LifeCrypto::delete_keyring_key().unwrap();
    }
}
