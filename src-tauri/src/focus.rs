use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_notification::NotificationExt;

use crate::focus_time::{self, Commitment, MINUTE, PRESETS, Preset};
use crate::focus_urgency::{self, Candidate, Ranked};
use crate::sync::LockExt;

pub const STATE_CHANGED: &str = "focus_state_changed";
pub const TIME_UP: &str = "focus_time_up";
pub const WRAP_UP: &str = "focus_wrap_up";
pub const HOTKEY_PRESSED: &str = "focus_hotkey";
pub const DICTATED: &str = "focus_dictated";
pub const HOTKEY: &str = "Alt+Shift+F";

const MAX_MINUTES: u64 = 240;
const MAX_TITLE: usize = 120;
const MAX_DONE_WHEN: usize = 200;
const RECENT: usize = 5;
const SCHEMA_VERSION: i64 = 4;
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
    pub outcome: Option<String>,
    pub task_id: Option<String>,
    pub handed_off_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub state: &'static str,
    pub block: Option<Block>,
    pub recent: Vec<Block>,
    pub commitments: Vec<Commitment>,
    pub preset: Preset,
    pub presets: Vec<Preset>,
}

pub struct Store {
    db: Connection,
}

const COLUMNS: &str =
    "id, title, done_when, planned_ms, started_ms, ended_ms, outcome, task_id, handed_off_ms";

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
        handed_off_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
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
                end_ms INTEGER NOT NULL,
                source TEXT NOT NULL DEFAULT 'manual',
                external_id TEXT
             );
             CREATE TABLE IF NOT EXISTS focus_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );",
        )
        .map_err(err)?;
        let has_source = db
            .prepare("SELECT 1 FROM pragma_table_info('commitments') WHERE name = 'source'")
            .and_then(|mut q| q.exists([]))
            .map_err(err)?;
        if !has_source {
            db.execute_batch(
                "ALTER TABLE commitments ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
                 ALTER TABLE commitments ADD COLUMN external_id TEXT;",
            )
            .map_err(err)?;
        }
        let has_handoff = db
            .prepare("SELECT 1 FROM pragma_table_info('focus_blocks') WHERE name = 'handed_off_ms'")
            .and_then(|mut q| q.exists([]))
            .map_err(err)?;
        if !has_handoff {
            db.execute_batch("ALTER TABLE focus_blocks ADD COLUMN handed_off_ms INTEGER;")
                .map_err(err)?;
        }
        db.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS one_commitment_per_event
                ON commitments (external_id) WHERE external_id IS NOT NULL;",
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
            preset: self.preset()?,
            presets: PRESETS.to_vec(),
        })
    }

    pub fn preset(&self) -> Result<Preset, String> {
        let id: Option<String> = self
            .db
            .query_row(
                "SELECT value FROM focus_settings WHERE key = 'preset'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(err)?;
        Ok(id.and_then(|id| Preset::by_id(&id)).unwrap_or_default())
    }

    pub fn set_preset(&self, id: &str) -> Result<Preset, String> {
        let preset = Preset::by_id(id).ok_or_else(|| format!("Unknown focus preset \"{id}\"."))?;
        self.db
            .execute(
                "INSERT INTO focus_settings (key, value) VALUES ('preset', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [preset.id],
            )
            .map_err(err)?;
        Ok(preset)
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
        let typed: i64 = self
            .db
            .query_row(
                "SELECT COUNT(*) FROM commitments WHERE source = 'manual' AND end_ms > ?1",
                [now as i64],
                |row| row.get(0),
            )
            .map_err(err)?;
        if typed as usize >= MAX_UPCOMING {
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
            source: "manual".into(),
        })
    }

    pub fn remove_commitment(&self, id: i64) -> Result<(), String> {
        let source: Option<String> = self
            .db
            .query_row("SELECT source FROM commitments WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)?;
        match source.as_deref() {
            None => return Err("That commitment is already gone.".into()),
            Some("calendar") => {
                return Err("This comes from your calendar. Change it there.".into());
            }
            Some(_) => {}
        }
        self.db
            .execute("DELETE FROM commitments WHERE id = ?1", [id])
            .map_err(err)?;
        Ok(())
    }

    pub fn sync_calendar(&self, events: &[crate::focus_calendar::Event]) -> Result<usize, String> {
        let tx = self.db.unchecked_transaction().map_err(err)?;
        for event in events {
            tx.execute(
                "INSERT INTO commitments (title, start_ms, end_ms, source, external_id)
                 VALUES (?1, ?2, ?3, 'calendar', ?4)
                 ON CONFLICT(external_id) WHERE external_id IS NOT NULL DO UPDATE SET
                    title = excluded.title,
                    start_ms = excluded.start_ms,
                    end_ms = excluded.end_ms",
                params![
                    event.title,
                    event.start_ms as i64,
                    event.end_ms as i64,
                    event.id
                ],
            )
            .map_err(err)?;
        }
        let keep: Vec<&str> = events.iter().map(|e| e.id.as_str()).collect();
        let mut stale = Vec::new();
        {
            let mut query = tx
                .prepare("SELECT id, external_id FROM commitments WHERE source = 'calendar'")
                .map_err(err)?;
            let rows = query
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
                })
                .map_err(err)?;
            for row in rows {
                let (id, external) = row.map_err(err)?;
                if !external.as_deref().is_some_and(|e| keep.contains(&e)) {
                    stale.push(id);
                }
            }
        }
        for id in stale {
            tx.execute("DELETE FROM commitments WHERE id = ?1", [id])
                .map_err(err)?;
        }
        tx.commit().map_err(err)?;
        Ok(events.len())
    }

    pub fn upcoming(&self, now: u64) -> Result<Vec<Commitment>, String> {
        let mut query = self
            .db
            .prepare(
                "SELECT id, title, start_ms, end_ms, source FROM commitments
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
                    source: row.get(4)?,
                })
            })
            .map_err(err)?
            .collect::<rusqlite::Result<_>>()
            .map_err(err)
    }

    pub fn mark_handed_off(&self, now: u64) -> Result<Block, String> {
        let running = self
            .active()?
            .ok_or_else(|| "No focus block is running.".to_string())?;
        if running.handed_off_ms.is_some() {
            return Err("This block is already with the agent.".into());
        }
        self.db
            .execute(
                "UPDATE focus_blocks SET handed_off_ms = ?1 WHERE id = ?2",
                params![now as i64, running.id],
            )
            .map_err(err)?;
        Ok(Block {
            handed_off_ms: Some(now),
            ..running
        })
    }

    pub fn extend(&self, now: u64) -> Result<Block, String> {
        let running = self
            .active()?
            .ok_or_else(|| "No focus block is running.".to_string())?;
        let preset = self.preset()?;
        let from = running.ends_ms.max(now);
        let new_end = from + preset.extend_ms();
        let commitments = self.upcoming(now)?;
        if let Some(limit) = focus_time::focus_limit(&preset, &commitments, now)
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

    pub fn free_ms(&self, now: u64) -> Option<u64> {
        self.with(|store| Ok((store.preset()?, store.upcoming(now)?)))
            .ok()
            .and_then(|(preset, commitments)| focus_time::free_ms(&preset, &commitments, now))
    }

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
pub fn focus_preset_set(
    app: AppHandle,
    focus: State<Focus>,
    id: String,
) -> Result<Snapshot, String> {
    let snapshot = focus.with(|store| {
        store.set_preset(&id)?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

static DICTATING: AtomicBool = AtomicBool::new(false);
static OPENED_MIC: AtomicBool = AtomicBool::new(false);

pub fn dictating() -> bool {
    DICTATING.load(Ordering::SeqCst)
}

#[cfg(desktop)]
pub fn hotkey() -> Shortcut {
    use std::str::FromStr;
    Shortcut::from_str(HOTKEY).expect("the focus hotkey must parse")
}

#[cfg(desktop)]
fn register_hotkey(app: &AppHandle) {
    if let Err(error) = app.global_shortcut().register(hotkey()) {
        log::warn!("could not register the focus hotkey {HOTKEY}: {error}");
    }
}

#[cfg_attr(mobile, allow(dead_code))]
pub fn on_hotkey(app: &AppHandle) {
    crate::window::summon(app);
    let _ = app.emit(HOTKEY_PRESSED, ());
}

pub fn take_dictation(app: &AppHandle, text: &str) -> bool {
    if !dictating() {
        return false;
    }
    let _ = app.emit(DICTATED, text);
    true
}

#[tauri::command]
pub async fn focus_dictation_set(
    app: AppHandle,
    state: State<'_, crate::events::AppState>,
    on: bool,
) -> Result<(), String> {
    if on {
        let mic_was_open = state.mic.locked().is_some();
        if !mic_was_open {
            crate::commands::toggle_listening(app.clone(), state)
                .await
                .map_err(|error| error.to_string())?;
            OPENED_MIC.store(true, Ordering::SeqCst);
        }
        DICTATING.store(true, Ordering::SeqCst);
    } else {
        DICTATING.store(false, Ordering::SeqCst);
        if OPENED_MIC.swap(false, Ordering::SeqCst) {
            crate::commands::close_microphone(&app);
        }
    }
    Ok(())
}

pub fn handoff_request(block: &Block) -> String {
    let minutes = block.planned_ms / MINUTE;
    let check = if block.done_when.is_empty() {
        "No done-when was given: say what you finished and what is left.".to_string()
    } else {
        format!(
            "Done when: {}. Tell me whether that is true when you stop, and why if it is not.",
            block.done_when
        )
    };
    format!(
        "{}\n\n{check}\n\nThis is a {minutes} minute focus block I am handing to you. \
         Ask me before anything you are unsure about or cannot undo.",
        block.title
    )
}

#[tauri::command]
pub fn focus_hand_off(app: AppHandle, focus: State<Focus>) -> Result<Snapshot, String> {
    let block = focus
        .with(Store::active)?
        .ok_or("No focus block is running.")?;
    if block.handed_off_ms.is_some() {
        return Err("This block is already with the agent.".into());
    }
    crate::intake::submit(&app, &handoff_request(&block), false)?;
    let snapshot = focus.with(|store| {
        if let Err(error) = store.mark_handed_off(now_ms()) {
            log::warn!("could not note the hand-off: {error}");
        }
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
        let preset = store.preset()?;
        let free = focus_time::free_ms(&preset, &store.upcoming(now)?, now);
        let minutes = focus_time::fit_minutes(left.clamp(5, preset.default_minutes), free)
            .ok_or("You have no free time before your next commitment.")?;
        let title: String = task.title.chars().take(MAX_TITLE).collect();
        store.start_for(&title, "", minutes, now, Some(&task.id))?;
        store.snapshot()
    })?;
    announce(&app, &snapshot);
    Ok(snapshot)
}

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
    crate::focus_calendar::start(app);
    #[cfg(desktop)]
    register_hotkey(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut over_told: Option<(i64, u64)> = None;
        let mut wrap_told: Option<(i64, i64)> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let now = now_ms();
            let Some((block, commitments, preset)) = app
                .state::<Focus>()
                .with(|store| Ok((store.active()?, store.upcoming(now)?, store.preset()?)))
                .ok()
                .and_then(|(block, commitments, preset)| Some((block?, commitments, preset)))
            else {
                continue;
            };
            if let Some(next) = focus_time::wrap_up(&preset, &commitments, now)
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

    #[test]
    fn the_default_preset_is_used_until_one_is_chosen_and_the_choice_is_kept() {
        let store = Store::memory();
        assert_eq!(store.preset().unwrap().id, "engineer");
        assert_eq!(store.set_preset("student").unwrap().id, "student");
        assert_eq!(store.preset().unwrap().id, "student");
        assert_eq!(store.snapshot_at(0).unwrap().preset.id, "student");
        assert_eq!(store.snapshot_at(0).unwrap().presets.len(), 3);
        store.set_preset("founder").unwrap();
        assert_eq!(store.preset().unwrap().id, "founder", "replaced, not added");
    }

    #[test]
    fn an_unknown_preset_is_refused_and_changes_nothing() {
        let store = Store::memory();
        store.set_preset("student").unwrap();
        assert!(store.set_preset("wizard").unwrap_err().contains("wizard"));
        assert_eq!(store.preset().unwrap().id, "student");
    }

    #[test]
    fn the_preset_decides_how_much_room_a_commitment_needs_around_it() {
        let store = Store::memory();
        store.add_commitment("Class", 40 * MIN, 30, 0).unwrap();
        let free = |store: &Store| {
            focus_time::free_ms(&store.preset().unwrap(), &store.upcoming(0).unwrap(), 0)
        };
        assert_eq!(free(&store), Some(35 * MIN));
        store.set_preset("student").unwrap();
        assert_eq!(free(&store), Some(30 * MIN), "ten minutes to get to class");
    }

    #[test]
    fn extending_adds_the_presets_step() {
        let store = Store::memory();
        store.set_preset("student").unwrap();
        store.start("Read", "", 25, 0).unwrap();
        let block = store.extend(0).unwrap();
        assert_eq!(block.planned_ms, 30 * MIN);
    }

    fn ev(id: &str, title: &str, start_min: u64, end_min: u64) -> crate::focus_calendar::Event {
        crate::focus_calendar::Event {
            id: id.into(),
            title: title.into(),
            start_ms: start_min * MIN,
            end_ms: end_min * MIN,
        }
    }

    #[test]
    fn synced_events_become_calendar_commitments_next_to_typed_ones() {
        let store = Store::memory();
        store.add_commitment("Dentist", 300 * MIN, 30, 0).unwrap();
        assert_eq!(
            store.sync_calendar(&[ev("a", "Standup", 60, 75)]).unwrap(),
            1
        );
        let list = store.upcoming(0).unwrap();
        let shape: Vec<_> = list
            .iter()
            .map(|c| (c.title.as_str(), c.source.as_str()))
            .collect();
        assert_eq!(shape, [("Standup", "calendar"), ("Dentist", "manual")]);
    }

    #[test]
    fn a_sync_updates_in_place_adds_and_removes_but_never_touches_typed_commitments() {
        let store = Store::memory();
        store.add_commitment("Dentist", 300 * MIN, 30, 0).unwrap();
        store
            .sync_calendar(&[ev("a", "Standup", 60, 75), ev("b", "Review", 120, 150)])
            .unwrap();
        let before = store.upcoming(0).unwrap();
        let standup_id = before[0].id;
        store
            .sync_calendar(&[
                ev("a", "Standup (moved)", 90, 105),
                ev("c", "Lunch", 200, 230),
            ])
            .unwrap();
        let after = store.upcoming(0).unwrap();
        let titles: Vec<_> = after.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Standup (moved)", "Lunch", "Dentist"],
            "Review is gone"
        );
        assert_eq!(
            after[0].id, standup_id,
            "the same commitment, so no second wrap-up warning"
        );
        assert_eq!(after[0].start_ms, 90 * MIN);
    }

    #[test]
    fn an_empty_calendar_clears_only_the_synced_commitments() {
        let store = Store::memory();
        store.add_commitment("Dentist", 300 * MIN, 30, 0).unwrap();
        store.sync_calendar(&[ev("a", "Standup", 60, 75)]).unwrap();
        store.sync_calendar(&[]).unwrap();
        let list = store.upcoming(0).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Dentist");
    }

    #[test]
    fn a_calendar_commitment_cannot_be_removed_here() {
        let store = Store::memory();
        store.sync_calendar(&[ev("a", "Standup", 60, 75)]).unwrap();
        let id = store.upcoming(0).unwrap()[0].id;
        assert!(
            store
                .remove_commitment(id)
                .unwrap_err()
                .contains("calendar")
        );
        assert_eq!(store.upcoming(0).unwrap().len(), 1);
    }

    #[test]
    fn calendar_events_do_not_use_up_the_room_for_typed_commitments() {
        let store = Store::memory();
        let events: Vec<_> = (0..MAX_UPCOMING as u64)
            .map(|i| ev(&format!("e{i}"), "Meeting", 60 + i * 10, 65 + i * 10))
            .collect();
        store.sync_calendar(&events).unwrap();
        assert!(store.add_commitment("Dentist", 900 * MIN, 30, 0).is_ok());
    }

    #[test]
    fn free_time_counts_a_calendar_meeting_like_a_typed_one() {
        let store = Store::memory();
        store.sync_calendar(&[ev("a", "Standup", 40, 70)]).unwrap();
        let preset = store.preset().unwrap();
        assert_eq!(
            focus_time::free_ms(&preset, &store.upcoming(0).unwrap(), 0),
            Some(35 * MIN)
        );
    }

    #[test]
    fn a_version_2_database_gains_the_calendar_columns_and_keeps_its_commitments() {
        let path = std::env::temp_dir().join(format!("focus-v2-{}.sqlite3", now_ms()));
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                "CREATE TABLE commitments (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    start_ms INTEGER NOT NULL,
                    end_ms INTEGER NOT NULL
                 );
                 INSERT INTO commitments (title, start_ms, end_ms) VALUES ('Old', 100000, 200000);
                 PRAGMA user_version = 2;",
            )
            .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let list = store.upcoming(0).unwrap();
        assert_eq!(
            (list[0].title.as_str(), list[0].source.as_str()),
            ("Old", "manual")
        );
        assert_eq!(
            store.sync_calendar(&[ev("a", "Standup", 60, 75)]).unwrap(),
            1
        );
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(desktop)]
    #[test]
    fn the_focus_hotkey_parses_and_does_not_collide_with_the_other_hotkeys() {
        use std::str::FromStr;
        let focus = hotkey();
        assert_ne!(
            focus,
            Shortcut::from_str(crate::hotkeys::DEFAULT_TOGGLE).unwrap()
        );
        assert_ne!(
            focus,
            Shortcut::from_str(crate::hotkeys::DEFAULT_STOP).unwrap()
        );
    }

    #[test]
    fn dictation_is_off_until_the_card_asks_for_it() {
        assert!(!dictating());
        DICTATING.store(true, Ordering::SeqCst);
        assert!(dictating());
        DICTATING.store(false, Ordering::SeqCst);
        assert!(!dictating());
    }

    fn block_with(done_when: &str) -> Block {
        Block {
            id: 1,
            title: "Write the release notes".into(),
            done_when: done_when.into(),
            planned_ms: 25 * MIN,
            started_ms: 0,
            ends_ms: 25 * MIN,
            finished_ms: None,
            outcome: None,
            task_id: None,
            handed_off_ms: None,
        }
    }

    #[test]
    fn the_request_to_the_agent_carries_the_title_the_check_and_a_duty_to_ask() {
        let text = handoff_request(&block_with("every change has a line"));
        assert!(text.starts_with("Write the release notes"));
        assert!(text.contains("Done when: every change has a line."));
        assert!(text.contains("25 minute focus block"));
        assert!(text.contains("Ask me before anything"));
        assert!(text.chars().count() <= 4000, "intake refuses more");
    }

    #[test]
    fn without_a_done_when_the_agent_is_asked_to_say_what_it_finished() {
        let text = handoff_request(&block_with(""));
        assert!(!text.contains("Done when:"));
        assert!(text.contains("say what you finished"));
    }

    #[test]
    fn the_longest_block_still_fits_what_intake_accepts() {
        let mut block = block_with(&"x".repeat(MAX_DONE_WHEN));
        block.title = "t".repeat(MAX_TITLE);
        assert!(handoff_request(&block).chars().count() <= 4000);
    }

    #[test]
    fn a_block_is_handed_over_once() {
        let store = Store::memory();
        assert!(
            store
                .mark_handed_off(1)
                .unwrap_err()
                .contains("No focus block")
        );
        store.start("Draft", "", 25, 0).unwrap();
        assert_eq!(store.active().unwrap().unwrap().handed_off_ms, None);
        let marked = store.mark_handed_off(5).unwrap();
        assert_eq!(marked.handed_off_ms, Some(5));
        assert_eq!(
            store.snapshot_at(6).unwrap().block.unwrap().handed_off_ms,
            Some(5)
        );
        assert!(store.mark_handed_off(9).unwrap_err().contains("already"));
    }

    #[test]
    fn the_hand_off_is_kept_in_the_log_after_the_block_ends() {
        let store = Store::memory();
        store.start("Draft", "", 25, 0).unwrap();
        store.mark_handed_off(5).unwrap();
        store.finish("done", 10).unwrap();
        assert_eq!(store.recent(1).unwrap()[0].handed_off_ms, Some(5));
    }

    #[test]
    fn a_version_3_database_gains_the_hand_off_column_and_keeps_its_blocks() {
        let path = std::env::temp_dir().join(format!("focus-v3-{}.sqlite3", now_ms()));
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch(
                "CREATE TABLE focus_blocks (id INTEGER PRIMARY KEY AUTOINCREMENT,
                  title TEXT NOT NULL, done_when TEXT NOT NULL DEFAULT '',
                  planned_ms INTEGER NOT NULL, started_ms INTEGER NOT NULL,
                  ended_ms INTEGER, outcome TEXT CHECK (outcome IN ('done','stopped')), task_id TEXT);
                 INSERT INTO focus_blocks (title, planned_ms, started_ms, ended_ms, outcome)
                  VALUES ('Old block', 60000, 0, 60000, 'done');
                 PRAGMA user_version = 3;",
            )
            .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let recent = store.recent(5).unwrap();
        assert_eq!(recent[0].title, "Old block");
        assert_eq!(recent[0].handed_off_ms, None);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
