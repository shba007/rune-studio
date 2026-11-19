//! Focus blocks: a time-boxed stretch of the user's attention (cycle 1 of the
//! Focus mode plan). A block has a title, an optional "done when" line and a
//! length. It is kept in its own `focus.sqlite3`, separate from the trace, and
//! counts down against the wall clock, so it survives closing the app.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::focus_time::{self, Commitment, EXTEND_MS, MINUTE};
use crate::focus_urgency::{self, Candidate, Ranked};
use crate::sync::LockExt;

pub const STATE_CHANGED: &str = "focus_state_changed";
pub const TIME_UP: &str = "focus_time_up";
pub const WRAP_UP: &str = "focus_wrap_up";

const MAX_MINUTES: u64 = 240;
const MAX_TITLE: usize = 120;
const MAX_DONE_WHEN: usize = 200;
const RECENT: usize = 5;
const SCHEMA_VERSION: i64 = 2;
const MAX_COMMITMENT_MINUTES: u64 = 720;
const MAX_UPCOMING: usize = 20;

pub fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join("focus.sqlite3")
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub id: i64,
    pub title: String,
    pub done_when: String,
    pub planned_ms: u64,
    pub started_ms: u64,
    pub ends_ms: u64,
    pub finished_ms: Option<u64>,
    /// `done` or `stopped`; `None` while the block is running.
    pub outcome: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// `idle` or `focusing`.
    pub state: &'static str,
    pub block: Option<Block>,
    pub recent: Vec<Block>,
    /// Commitments that have not ended yet, soonest first.
    pub commitments: Vec<Commitment>,
}

pub struct Store {
    db: Connection,
}

const COLUMNS: &str = "id, title, done_when, planned_ms, started_ms, ended_ms, outcome, task_id";

fn block_from(row: &Row) -> rusqlite::Result<Block> {
    let planned_ms = row.get::<_, i64>(3)? as u64;
    let started_ms = row.get::<_, i64>(4)? as u64;
    Ok(Block {
        id: row.get(0)?,
        title: row.get(1)?,
        done_when: row.get(2)?,
        planned_ms,
        started_ms,
        ends_ms: started_ms + planned_ms,
        finished_ms: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        outcome: row.get(6)?,
        task_id: row.get(7)?,
    })
}

fn err(error: rusqlite::Error) -> String {
    format!("Focus blocks could not be saved: {error}")
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        Self::init(Connection::open(path).map_err(err)?)
    }

    #[cfg(test)]
    fn memory() -> Self {
        Self::init(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn init(db: Connection) -> Result<Self, String> {
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(err)?;
        if version > SCHEMA_VERSION {
            return Err(format!(
                "focus.sqlite3 was written by a newer Rune Studio (schema {version}); update Rune Studio"
            ));
        }
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS focus_blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                done_when TEXT NOT NULL DEFAULT '',
                planned_ms INTEGER NOT NULL,
                started_ms INTEGER NOT NULL,
                ended_ms INTEGER,
                outcome TEXT CHECK (outcome IN ('done', 'stopped')),
                task_id TEXT
             );
             CREATE UNIQUE INDEX IF NOT EXISTS one_running_block
                ON focus_blocks ((1)) WHERE ended_ms IS NULL;
             CREATE TABLE IF NOT EXISTS commitments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                start_ms INTEGER NOT NULL,
                end_ms INTEGER NOT NULL
             );",
        )
        .map_err(err)?;
        db.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(err)?;
        Ok(Self { db })
    }

    pub fn active(&self) -> Result<Option<Block>, String> {
        self.db
            .query_row(
                &format!("SELECT {COLUMNS} FROM focus_blocks WHERE ended_ms IS NULL"),
                [],
                block_from,
            )
            .optional()
            .map_err(err)
    }

    pub fn start(
        &self,
        title: &str,
        done_when: &str,
        minutes: u64,
        now: u64,
    ) -> Result<Block, String> {
        self.start_for(title, done_when, minutes, now, None)
    }

    /// Starts a block, optionally for a task, so its time can be credited to
    /// that task.
    pub fn start_for(
        &self,
        title: &str,
        done_when: &str,
        minutes: u64,
        now: u64,
        task_id: Option<&str>,
    ) -> Result<Block, String> {
        let title = title.trim();
        let done_when = done_when.trim();
        if title.is_empty() {
            return Err("Give the focus block a title.".into());
        }
        if title.chars().count() > MAX_TITLE {
            return Err(format!("The title can be at most {MAX_TITLE} characters."));
        }
        if done_when.chars().count() > MAX_DONE_WHEN {
            return Err(format!(
                "\"Done when\" can be at most {MAX_DONE_WHEN} characters."
            ));
        }
        if !(1..=MAX_MINUTES).contains(&minutes) {
            return Err(format!("A focus block is 1 to {MAX_MINUTES} minutes long."));
        }
        if self.active()?.is_some() {
            return Err("A focus block is already running. Finish it first.".into());
        }
        self.db
            .execute(
                "INSERT INTO focus_blocks (title, done_when, planned_ms, started_ms, task_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    title,
                    done_when,
                    (minutes * 60_000) as i64,
                    now as i64,
                    task_id
                ],
            )
            .map_err(err)?;
        self.active()?
            .ok_or_else(|| "The focus block was not saved.".into())
    }

    /// Time spent on each task in finished blocks. A block counts for at most
    /// its planned length, so one left running overnight does not credit hours.
    pub fn credited_ms(&self) -> Result<HashMap<String, u64>, String> {
        let mut query = self
            .db
            .prepare(
                "SELECT task_id, SUM(MIN(ended_ms - started_ms, planned_ms)) FROM focus_blocks
                 WHERE task_id IS NOT NULL AND ended_ms IS NOT NULL GROUP BY task_id",
            )
            .map_err(err)?;
        query
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?.max(0) as u64,
                ))
            })
            .map_err(err)?
            .collect::<rusqlite::Result<_>>()
            .map_err(err)
    }

    /// Time spent on one task in blocks started at or after `since_ms`.
    pub fn credited_for(&self, task_id: &str, since_ms: u64) -> Result<u64, String> {
        self.db
            .query_row(
                "SELECT COALESCE(SUM(MIN(ended_ms - started_ms, planned_ms)), 0) FROM focus_blocks
                 WHERE task_id = ?1 AND ended_ms IS NOT NULL AND started_ms >= ?2",
                params![task_id, since_ms as i64],
                |row| row.get::<_, i64>(0),
            )
            .map(|ms| ms.max(0) as u64)
            .map_err(err)
    }

    /// Ends the running block as `done` or `stopped`.
    pub fn finish(&self, outcome: &str, now: u64) -> Result<Block, String> {
        if !matches!(outcome, "done" | "stopped") {
            return Err("A focus block ends as done or stopped.".into());
        }
        let running = self
            .active()?
            .ok_or_else(|| "No focus block is running.".to_string())?;
        self.db
            .execute(
                "UPDATE focus_blocks SET ended_ms = ?1, outcome = ?2 WHERE id = ?3",
                params![now as i64, outcome, running.id],
            )
            .map_err(err)?;
        Ok(Block {
            finished_ms: Some(now),
            outcome: Some(outcome.into()),
            ..running
        })
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<Block>, String> {
        let mut query = self
            .db
            .prepare(&format!(
                "SELECT {COLUMNS} FROM focus_blocks WHERE ended_ms IS NOT NULL
                 ORDER BY ended_ms DESC, id DESC LIMIT ?1"
            ))
            .map_err(err)?;
        query
            .query_map([limit as i64], block_from)
            .map_err(err)?
            .collect::<rusqlite::Result<_>>()
            .map_err(err)
    }

    pub fn snapshot(&self) -> Result<Snapshot, String> {
        self.snapshot_at(now_ms())
    }

    pub fn snapshot_at(&self, now: u64) -> Result<Snapshot, String> {
        let block = self.active()?;
        Ok(Snapshot {
            state: if block.is_some() { "focusing" } else { "idle" },
            block,
            recent: self.recent(RECENT)?,
            commitments: self.upcoming(now)?,
        })
    }

    pub fn add_commitment(
        &self,
        title: &str,
        start_ms: u64,
        minutes: u64,
        now: u64,
    ) -> Result<Commitment, String> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > MAX_TITLE {
            return Err(format!(
                "Give the commitment a title of up to {MAX_TITLE} characters."
            ));
        }
        if !(1..=MAX_COMMITMENT_MINUTES).contains(&minutes) {
            return Err(format!(
                "A commitment is 1 to {MAX_COMMITMENT_MINUTES} minutes long."
            ));
        }
        let end_ms = start_ms + minutes * MINUTE;
        if end_ms <= now {
            return Err("That commitment is already over. Pick a time that has not passed.".into());
        }
        if self.upcoming(now)?.len() >= MAX_UPCOMING {
            return Err(format!(
                "You already have {MAX_UPCOMING} upcoming commitments. Remove one first."
            ));
        }
        self.db
            .execute(
                "INSERT INTO commitments (title, start_ms, end_ms) VALUES (?1, ?2, ?3)",
                params![title, start_ms as i64, end_ms as i64],
            )
            .map_err(err)?;
        Ok(Commitment {
            id: self.db.last_insert_rowid(),
            title: title.to_string(),
            start_ms,
            end_ms,
        })
    }

    pub fn remove_commitment(&self, id: i64) -> Result<(), String> {
        let removed = self
            .db
            .execute("DELETE FROM commitments WHERE id = ?1", [id])
            .map_err(err)?;
        if removed == 0 {
            return Err("That commitment is already gone.".into());
        }
        Ok(())
    }

    pub fn upcoming(&self, now: u64) -> Result<Vec<Commitment>, String> {
        let mut query = self
            .db
            .prepare(
                "SELECT id, title, start_ms, end_ms FROM commitments
                 WHERE end_ms > ?1 ORDER BY start_ms, id LIMIT ?2",
            )
            .map_err(err)?;
        query
            .query_map(params![now as i64, MAX_UPCOMING as i64], |row| {
                Ok(Commitment {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    start_ms: row.get::<_, i64>(2)? as u64,
                    end_ms: row.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(err)?
            .collect::<rusqlite::Result<_>>()
            .map_err(err)
    }

    /// Adds five minutes to the running block, counted from now when it has
    /// already run out. Refused when that would run into the next commitment
    /// or its buffer.
    pub fn extend(&self, now: u64) -> Result<Block, String> {
        let running = self
            .active()?
            .ok_or_else(|| "No focus block is running.".to_string())?;
        let from = running.ends_ms.max(now);
        let new_end = from + EXTEND_MS;
        let commitments = self.upcoming(now)?;
        if let Some(limit) = focus_time::focus_limit(&commitments, now)
            && new_end > limit
            && let Some(next) = focus_time::next(&commitments, now)
        {
            let minutes = next.start_ms.saturating_sub(now).div_ceil(MINUTE);
            return Err(if next.start_ms <= now {
                format!("No room to extend: \"{}\" has started.", next.title)
            } else {
                format!(
                    "No room to extend: \"{}\" starts in {minutes} min.",
                    next.title
                )
            });
        }
        let planned_ms = running.planned_ms + (new_end - running.ends_ms);
        if planned_ms > MAX_MINUTES * MINUTE {
            return Err(format!("A focus block is at most {MAX_MINUTES} minutes."));
        }
        self.db
            .execute(
                "UPDATE focus_blocks SET planned_ms = ?1 WHERE id = ?2",
                params![planned_ms as i64, running.id],
            )
            .map_err(err)?;
        Ok(Block {
            planned_ms,
            ends_ms: new_end,
            ..running
        })
    }
}

/// Managed state. Keeps the reason when the database could not be opened, so
/// the commands can tell the user instead of failing silently.
pub struct Focus {
    store: Result<Mutex<Store>, String>,
}

impl Focus {
    pub fn open(data_dir: &Path) -> Self {
        let store = Store::open(&db_path(data_dir)).map(Mutex::new);
        if let Err(error) = &store {
            log::error!("focus blocks are unavailable: {error}");
        }
        Self { store }
    }

    /// Free time before the next commitment, or `None` when unlimited (or when
    /// the store is unavailable, where nothing can be checked anyway).
    pub fn free_ms(&self, now: u64) -> Option<u64> {
        self.with(|store| store.upcoming(now))
            .ok()
            .and_then(|commitments| focus_time::free_ms(&commitments, now))
    }

    /// Time put into a task since `since_ms`; zero when it cannot be read.
    pub fn credited_for(&self, task_id: &str, since_ms: u64) -> u64 {
        self.with(|store| store.credited_for(task_id, since_ms))
            .unwrap_or(0)
    }

    pub(crate) fn with<T>(
        &self,
        run: impl FnOnce(&Store) -> Result<T, String>,
    ) -> Result<T, String> {
        match &self.store {
            Ok(store) => run(&store.locked()),
            Err(reason) => Err(format!("Focus blocks are unavailable: {reason}")),
        }
    }
}

fn announce(app: &AppHandle, snapshot: &Snapshot) {
    let _ = app.emit(STATE_CHANGED, snapshot);
}

#[tauri::command]
pub fn focus_snapshot(focus: State<Focus>) -> Result<Snapshot, String> {
    focus.with(Store::snapshot)
}

#[tauri::command]
pub fn focus_start(
    app: AppHandle,
    focus: State<Focus>,
    title: String,
    done_when: String,
    minutes: u64,
) -> Result<Snapshot, String> {
    let snapshot = focus.with(|store| {
        store.start(&title, &done_when, minutes, now_ms())?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn focus_finish(
    app: AppHandle,
    focus: State<Focus>,
    outcome: String,
) -> Result<Snapshot, String> {
    let (snapshot, finished) = focus.with(|store| {
        let finished = store.finish(&outcome, now_ms())?;
        Ok((store.snapshot()?, finished))
    })?;
    if let Some(task_id) = &finished.task_id
        && let Err(error) = crate::habits::complete_if_target_met(&app, &focus, task_id)
    {
        log::warn!("could not check in a timed habit: {error}");
    }
    announce(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn focus_commitment_add(
    app: AppHandle,
    focus: State<Focus>,
    title: String,
    start_ms: u64,
    minutes: u64,
) -> Result<Snapshot, String> {
    let snapshot = focus.with(|store| {
        store.add_commitment(&title, start_ms, minutes, now_ms())?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn focus_commitment_remove(
    app: AppHandle,
    focus: State<Focus>,
    id: i64,
) -> Result<Snapshot, String> {
    let snapshot = focus.with(|store| {
        store.remove_commitment(id)?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

#[tauri::command]
pub fn focus_extend(app: AppHandle, focus: State<Focus>) -> Result<Snapshot, String> {
    let snapshot = focus.with(|store| {
        store.extend(now_ms())?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

/// The most urgent waiting tasks that are safe to show a model: only General
/// ones, because a Health or Finance title must not leave the device by way
/// of a suggestion.
pub fn urgent_general_tasks(app: &AppHandle, now: u64, limit: usize) -> Vec<Ranked> {
    let Ok(tasks) = crate::workspace_tasks::waiting_manual_tasks(app) else {
        return Vec::new();
    };
    let credited = app
        .try_state::<Focus>()
        .and_then(|focus| focus.with(Store::credited_ms).ok())
        .unwrap_or_default();
    let candidates: Vec<Candidate> = tasks
        .into_iter()
        .filter(|task| task.class == crate::life::DataClass::General)
        // A habit repeats every day; it is not a deadline to rank.
        .filter(|task| !crate::workspace_tasks::is_daily_habit(task))
        .filter_map(|task| {
            Some(Candidate {
                due_ms: task.occurrences.last()?.effective_due_ms()?,
                credited_ms: credited.get(&task.id).copied().unwrap_or(0),
                estimate_minutes: task.estimate_minutes,
                title: task.title,
                id: task.id,
            })
        })
        .collect();
    focus_urgency::rank(&candidates, now, limit)
}

/// Starts a block for a waiting task: its title, a length that fits both the
/// effort left and the free time, and the task linked for credit.
#[tauri::command]
pub fn focus_start_task(
    app: AppHandle,
    focus: State<Focus>,
    task_id: String,
) -> Result<Snapshot, String> {
    let task = crate::workspace_tasks::waiting_manual_tasks(&app)?
        .into_iter()
        .find(|task| task.id == task_id)
        .ok_or("This task is not waiting on you any more.")?;
    let now = now_ms();
    let snapshot = focus.with(|store| {
        // A daily habit starts again each day, so only today's time counts.
        let since = if crate::workspace_tasks::is_daily_habit(&task) {
            task.occurrences.last().and_then(|o| o.due_ms).unwrap_or(0)
        } else {
            0
        };
        let credited = store.credited_for(&task.id, since)?;
        let left = focus_urgency::minutes_left(&Candidate {
            id: task.id.clone(),
            title: String::new(),
            due_ms: 0,
            estimate_minutes: task.estimate_minutes,
            credited_ms: credited,
        });
        let free = focus_time::free_ms(&store.upcoming(now)?, now);
        let minutes = focus_time::fit_minutes(left.clamp(5, 25), free)
            .ok_or("You have no free time before your next commitment.")?;
        let title: String = task.title.chars().take(MAX_TITLE).collect();
        store.start_for(&title, "", minutes, now, Some(&task.id))?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

/// Opens the store and watches the running block. It emits `focus_time_up` when
/// the block runs out and `focus_wrap_up` when a commitment is about to start,
/// each with a notification, once. The block stays running until the user marks
/// it done or stopped.
pub fn start(app: &AppHandle) {
    let dir = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(error) => {
            log::error!("no app data folder, focus blocks will not be saved: {error}");
            app.manage(Focus {
                store: Err("there is no app data folder".into()),
            });
            return;
        }
    };
    app.manage(Focus::open(&dir));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Each warning is shown once: per block end time (an extension makes a
        // new end) and per block and commitment.
        let mut over_told: Option<(i64, u64)> = None;
        let mut wrap_told: Option<(i64, i64)> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let now = now_ms();
            let Some((block, commitments)) = app
                .state::<Focus>()
                .with(|store| Ok((store.active()?, store.upcoming(now)?)))
                .ok()
                .and_then(|(block, commitments)| Some((block?, commitments)))
            else {
                continue;
            };
            if let Some(next) = focus_time::wrap_up(&commitments, now)
                && wrap_told != Some((block.id, next.id))
            {
                wrap_told = Some((block.id, next.id));
                let _ = app.emit(WRAP_UP, next);
                let _ = app
                    .notification()
                    .builder()
                    .title("Time to wrap up")
                    .body(format!("{} is about to start", next.title))
                    .show();
            }
            if block.ends_ms <= now && over_told != Some((block.id, block.ends_ms)) {
                over_told = Some((block.id, block.ends_ms));
                let _ = app.emit(TIME_UP, &block);
                let _ = app
                    .notification()
                    .builder()
                    .title("Focus block is over")
                    .body(&block.title)
                    .show();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    #[test]
    fn a_new_store_is_idle_with_no_history() {
        let snapshot = Store::memory().snapshot().unwrap();
        assert_eq!(snapshot.state, "idle");
        assert_eq!(snapshot.block, None);
        assert!(snapshot.recent.is_empty());
    }

    #[test]
    fn starting_a_block_runs_it_for_exactly_its_length() {
        let store = Store::memory();
        let block = store
            .start("  Write auth tests  ", "tests pass", 25, 1_000)
            .unwrap();
        assert_eq!(block.title, "Write auth tests");
        assert_eq!(block.done_when, "tests pass");
        assert_eq!(block.started_ms, 1_000);
        assert_eq!(block.planned_ms, 25 * MIN);
        assert_eq!(block.ends_ms, 1_000 + 25 * MIN);
        assert_eq!(block.outcome, None);
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.state, "focusing");
        assert_eq!(snapshot.block, Some(block));
    }

    #[test]
    fn only_one_block_can_run_at_a_time() {
        let store = Store::memory();
        store.start("First", "", 25, 0).unwrap();
        let error = store.start("Second", "", 25, 1).unwrap_err();
        assert!(error.contains("already running"), "{error}");
        assert_eq!(store.active().unwrap().unwrap().title, "First");
    }

    #[test]
    fn done_and_stopped_close_the_block_and_log_it() {
        let store = Store::memory();
        store.start("One", "", 25, 0).unwrap();
        let done = store.finish("done", 10 * MIN).unwrap();
        assert_eq!(done.outcome.as_deref(), Some("done"));
        assert_eq!(done.finished_ms, Some(10 * MIN));
        store.start("Two", "", 10, 11 * MIN).unwrap();
        store.finish("stopped", 12 * MIN).unwrap();

        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.state, "idle");
        let log: Vec<_> = snapshot
            .recent
            .iter()
            .map(|b| (b.title.as_str(), b.outcome.as_deref().unwrap()))
            .collect();
        assert_eq!(log, [("Two", "stopped"), ("One", "done")], "newest first");
        store.start("Three", "", 5, 13 * MIN).unwrap();
    }

    #[test]
    fn finishing_needs_a_running_block_and_a_known_outcome() {
        let store = Store::memory();
        assert!(
            store
                .finish("done", 0)
                .unwrap_err()
                .contains("No focus block")
        );
        store.start("One", "", 25, 0).unwrap();
        assert!(
            store
                .finish("abandoned", 1)
                .unwrap_err()
                .contains("done or stopped")
        );
        assert!(store.active().unwrap().is_some(), "still running");
    }

    #[test]
    fn a_block_is_checked_before_it_is_saved() {
        let store = Store::memory();
        assert!(store.start("   ", "", 25, 0).unwrap_err().contains("title"));
        assert!(store.start("x", "", 0, 0).unwrap_err().contains("1 to 240"));
        assert!(
            store
                .start("x", "", 241, 0)
                .unwrap_err()
                .contains("1 to 240")
        );
        let long = "a".repeat(MAX_TITLE + 1);
        assert!(store.start(&long, "", 25, 0).unwrap_err().contains("120"));
        let long = "a".repeat(MAX_DONE_WHEN + 1);
        assert!(store.start("x", &long, 25, 0).unwrap_err().contains("200"));
        assert_eq!(store.active().unwrap(), None, "nothing was saved");
        store.start("x", "", 240, 0).unwrap();
    }

    #[test]
    fn a_running_block_survives_a_restart_and_keeps_its_end_time() {
        let dir = std::env::temp_dir().join(format!("rune-focus-{}", now_ms()));
        let path = db_path(&dir);
        let started = Store::open(&path)
            .unwrap()
            .start("Deep work", "draft done", 50, 5_000)
            .unwrap();

        let reopened = Store::open(&path).unwrap();
        let after = reopened.snapshot().unwrap();
        assert_eq!(after.state, "focusing");
        assert_eq!(after.block, Some(started));
        reopened.finish("done", 6_000).unwrap();
        drop(reopened);

        let again = Store::open(&path).unwrap().snapshot().unwrap();
        assert_eq!(again.state, "idle");
        assert_eq!(again.recent.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_newer_database_is_refused_and_left_alone() {
        let dir = std::env::temp_dir().join(format!("rune-focus-newer-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = db_path(&dir);
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        let error = Store::open(&path).err().unwrap();
        assert!(error.contains("newer Rune Studio"), "{error}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unavailable_storage_says_why_instead_of_failing_silently() {
        let focus = Focus {
            store: Err("disk is read-only".into()),
        };
        let error = focus.with(Store::snapshot).unwrap_err();
        assert_eq!(error, "Focus blocks are unavailable: disk is read-only");
    }

    #[test]
    fn commitments_are_listed_soonest_first_and_drop_off_when_over() {
        let store = Store::memory();
        store.add_commitment("Review", 100 * MIN, 30, 0).unwrap();
        store.add_commitment("Standup", 10 * MIN, 15, 0).unwrap();
        let titles = |now| {
            store
                .snapshot_at(now)
                .unwrap()
                .commitments
                .iter()
                .map(|c| c.title.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(titles(0), ["Standup", "Review"]);
        assert_eq!(titles(30 * MIN), ["Review"], "standup is over");
        assert!(titles(200 * MIN).is_empty());
    }

    #[test]
    fn a_commitment_is_checked_before_it_is_saved() {
        let store = Store::memory();
        assert!(store.add_commitment("  ", 5 * MIN, 30, 0).is_err());
        assert!(store.add_commitment("x", 5 * MIN, 0, 0).is_err());
        assert!(store.add_commitment("x", 5 * MIN, 721, 0).is_err());
        let past = store.add_commitment("x", 0, 5, 10 * MIN).unwrap_err();
        assert!(past.contains("already over"), "{past}");
        assert!(store.upcoming(0).unwrap().is_empty(), "nothing was saved");
        for n in 0..MAX_UPCOMING {
            store
                .add_commitment(&format!("c{n}"), (n as u64 + 1) * MIN, 5, 0)
                .unwrap();
        }
        let full = store
            .add_commitment("one more", 99 * MIN, 5, 0)
            .unwrap_err();
        assert!(full.contains("20 upcoming"), "{full}");
    }

    #[test]
    fn a_commitment_can_be_removed_once() {
        let store = Store::memory();
        let c = store.add_commitment("Standup", 10 * MIN, 15, 0).unwrap();
        store.remove_commitment(c.id).unwrap();
        assert!(store.upcoming(0).unwrap().is_empty());
        assert!(
            store
                .remove_commitment(c.id)
                .unwrap_err()
                .contains("already gone")
        );
    }

    #[test]
    fn extending_adds_five_minutes_when_there_is_room() {
        let store = Store::memory();
        store.start("Deep work", "", 25, 0).unwrap();
        store.add_commitment("Meeting", 60 * MIN, 30, 0).unwrap();
        let longer = store.extend(20 * MIN).unwrap();
        assert_eq!(longer.ends_ms, 30 * MIN);
        assert_eq!(longer.planned_ms, 30 * MIN);
        assert_eq!(store.active().unwrap().unwrap().ends_ms, 30 * MIN, "saved");
    }

    #[test]
    fn extending_a_block_that_ran_out_counts_from_now() {
        let store = Store::memory();
        store.start("Deep work", "", 25, 0).unwrap();
        let longer = store.extend(40 * MIN).unwrap();
        assert_eq!(
            longer.ends_ms,
            45 * MIN,
            "five minutes from now, not from 25"
        );
        assert_eq!(longer.planned_ms, 45 * MIN);
    }

    #[test]
    fn extending_is_refused_when_it_would_run_into_the_next_commitment() {
        let store = Store::memory();
        store.start("Deep work", "", 25, 0).unwrap();
        store.add_commitment("Meeting", 33 * MIN, 30, 0).unwrap();
        let error = store.extend(24 * MIN).unwrap_err();
        assert_eq!(error, "No room to extend: \"Meeting\" starts in 9 min.");
        assert_eq!(
            store.active().unwrap().unwrap().ends_ms,
            25 * MIN,
            "unchanged"
        );
        let started = store.extend(40 * MIN).unwrap_err();
        assert_eq!(started, "No room to extend: \"Meeting\" has started.");
    }

    #[test]
    fn extending_needs_a_running_block_and_stays_under_the_cap() {
        let store = Store::memory();
        assert!(store.extend(0).unwrap_err().contains("No focus block"));
        store.start("Long", "", 240, 0).unwrap();
        assert!(store.extend(0).unwrap_err().contains("at most 240"));
    }

    #[test]
    fn a_version_one_database_gains_commitments_and_keeps_its_blocks() {
        let dir = std::env::temp_dir().join(format!("rune-focus-v1-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = db_path(&dir);
        {
            let old = Connection::open(&path).unwrap();
            old.execute_batch(
                "CREATE TABLE focus_blocks (id INTEGER PRIMARY KEY AUTOINCREMENT,
                  title TEXT NOT NULL, done_when TEXT NOT NULL DEFAULT '',
                  planned_ms INTEGER NOT NULL, started_ms INTEGER NOT NULL,
                  ended_ms INTEGER, outcome TEXT CHECK (outcome IN ('done','stopped')), task_id TEXT);
                 INSERT INTO focus_blocks (title, planned_ms, started_ms, ended_ms, outcome)
                  VALUES ('Old block', 60000, 0, 60000, 'done');
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let snapshot = store.snapshot_at(0).unwrap();
        assert_eq!(snapshot.recent[0].title, "Old block");
        store.add_commitment("Standup", 5 * MIN, 15, 0).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_block_for_a_task_credits_its_time_up_to_the_planned_length() {
        let store = Store::memory();
        store.start_for("Report", "", 25, 0, Some("t1")).unwrap();
        store.finish("done", 20 * MIN).unwrap();
        store
            .start_for("Report", "", 25, 30 * MIN, Some("t1"))
            .unwrap();
        store.finish("stopped", 30 * MIN + 10 * MIN).unwrap();
        store
            .start_for("Left running", "", 10, 60 * MIN, Some("t2"))
            .unwrap();
        store.finish("done", 600 * MIN).unwrap();
        store.start("No task", "", 25, 700 * MIN).unwrap();
        store.finish("done", 710 * MIN).unwrap();
        store
            .start_for("Still going", "", 25, 800 * MIN, Some("t3"))
            .unwrap();

        let credited = store.credited_ms().unwrap();
        assert_eq!(credited["t1"], 30 * MIN, "20 + 10 minutes");
        assert_eq!(credited["t2"], 10 * MIN, "capped at the planned 10");
        assert!(
            !credited.contains_key("t3"),
            "unfinished blocks do not count yet"
        );
        assert_eq!(credited.len(), 2, "blocks without a task credit nothing");
    }

    #[test]
    fn time_for_a_task_can_be_counted_from_a_moment_on() {
        let store = Store::memory();
        store.start_for("Read", "", 30, 0, Some("h")).unwrap();
        store.finish("done", 30 * MIN).unwrap();
        store
            .start_for("Read", "", 30, 1000 * MIN, Some("h"))
            .unwrap();
        store.finish("done", 1010 * MIN).unwrap();
        assert_eq!(store.credited_for("h", 0).unwrap(), 40 * MIN, "all time");
        assert_eq!(
            store.credited_for("h", 500 * MIN).unwrap(),
            10 * MIN,
            "today only"
        );
        assert_eq!(store.credited_for("other", 0).unwrap(), 0);
    }
}
