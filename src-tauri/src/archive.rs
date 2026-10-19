//! The record of what the agent saw and heard: every screenshot it looks at, every recording of
//! the user's voice, and (when screen recording exists) every video, kept in the user's Documents
//! folder under `Rune Studio`.
//!
//! **One folder per session.** A session is one task run: a task the built-in agent carries out, or
//! one outside agent's turn at the wheel (`rune_begin` to `rune_done`). Everything captured while it
//! runs goes inside it, split by type, with a small manifest, so one task's evidence (and what is
//! learned from it later) is one folder:
//!
//! ```text
//! Documents/Rune Studio/sessions/2026-10-19/<hhmmss>-<n>-<task-slug>/session.json
//! Documents/Rune Studio/sessions/2026-10-19/<hhmmss>-<n>-<task-slug>/screenshots/<time>-<n>-screen.jpg
//! Documents/Rune Studio/sessions/2026-10-19/<hhmmss>-<n>-<task-slug>/voice/<time>-<n>-utterance.wav
//! Documents/Rune Studio/sessions/2026-10-19/<hhmmss>-<n>-<task-slug>/recordings/...
//! ```
//!
//! Anything saved while no session is open (a debug capture, an outside agent looking before it
//! has begun, speech that is not a command, a recording from before a spoken task started and was
//! not adopted) keeps the older layout, by type and then by day:
//!
//! ```text
//! Documents/Rune Studio/screenshots/2026-10-19/<time>-<n>-screen.jpg
//! Documents/Rune Studio/voice/2026-10-19/<time>-<n>-utterance.wav
//! Documents/Rune Studio/recordings/2026-10-19/<time>-<n>-<label>.<ext>
//! ```
//!
//! Documents is wherever the OS keeps it (the Windows Known Folder, `~/Documents` or the XDG
//! documents directory elsewhere). There is deliberately no setting, tool or command to turn this
//! off or to point it elsewhere, and it fails closed: the capture and voice code store first and
//! only hand the data on when the save worked, so if the folder cannot be written the agent gets
//! no screenshot and no utterance is acted on. Dates and times in names are UTC. Only one session
//! is open at a time; the manifest is a convenience and failing to write it is only logged.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex, MutexGuard, OnceLock, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

/// The folder inside Documents that holds everything.
pub const FOLDER: &str = "Rune Studio";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Screenshots,
    Voice,
    Recordings,
}

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Kind::Screenshots => "screenshots",
            Kind::Voice => "voice",
            Kind::Recordings => "recordings",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("the record folder is not available: {0}")]
    NotReady(String),
    #[error("could not write to the record folder: {0}")]
    Write(String),
}

static ROOT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// The folder (inside the record) that holds one folder per session.
const SESSIONS_FOLDER: &str = "sessions";
/// The manifest inside a session's folder.
const MANIFEST: &str = "session.json";
/// Longest task slug in a session's folder name.
const SLUG_MAX: usize = 32;

/// Finds (and creates) the record folders. Call once at startup. A failure is remembered, not
/// fatal to the app: every later save then fails, which is what keeps the agent blind.
pub fn init(app: &AppHandle) -> Result<PathBuf, String> {
    let result = app
        .path()
        .document_dir()
        .map_err(|err| err.to_string())
        .map(|dir| dir.join(FOLDER))
        .and_then(|root| {
            for kind in [Kind::Screenshots, Kind::Voice, Kind::Recordings] {
                fs::create_dir_all(root.join(kind.folder())).map_err(|err| format!("{}: {err}", root.display()))?;
            }
            fs::create_dir_all(root.join(SESSIONS_FOLDER)).map_err(|err| format!("{}: {err}", root.display()))?;
            Ok(root)
        });
    match &result {
        Ok(root) => log::info!("record folder: {}", root.display()),
        Err(err) => log::error!("record folder unavailable, the agent cannot see or hear until this is fixed: {err}"),
    }
    let _ = ROOT.set(result.clone());
    result
}

fn root() -> Result<&'static Path, ArchiveError> {
    match ROOT.get() {
        Some(Ok(root)) => Ok(root),
        Some(Err(err)) => Err(ArchiveError::NotReady(err.clone())),
        None => Err(ArchiveError::NotReady("not set up yet".into())),
    }
}

/// (year, month, day) for a count of days since 1970-01-01 (Howard Hinnant's civil_from_days).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `("2026-10-19", "075912")` for a Unix time in seconds.
fn stamp(seconds: u64) -> (String, String) {
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let in_day = seconds % 86_400;
    (format!("{year:04}-{month:02}-{day:02}"), format!("{:02}{:02}{:02}", in_day / 3_600, in_day % 3_600 / 60, in_day % 60))
}

/// One open session: a task run's folder.
#[derive(Debug, Clone)]
struct Session {
    /// The folder's name: `<hhmmss>-<n>-<task-slug>`.
    id: String,
    dir: PathBuf,
    task: String,
    source: String,
    started: u64,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0)
}

fn current() -> MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(PoisonError::into_inner)
}

fn write_error(err: impl std::fmt::Display) -> ArchiveError {
    ArchiveError::Write(err.to_string())
}

/// `2026-10-19T07:59:12Z` for a Unix time in seconds.
fn iso(seconds: u64) -> String {
    let (date, time) = stamp(seconds);
    format!("{date}T{}:{}:{}Z", &time[0..2], &time[2..4], &time[4..6])
}

/// A short, file-name-safe form of the task, for the session's folder name.
fn slug(task: &str) -> String {
    let mut out = String::new();
    for c in task.chars() {
        let c = if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' };
        if c != '-' || !(out.is_empty() || out.ends_with('-')) {
            out.push(c);
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(SLUG_MAX).collect();
    let out = out.trim_end_matches('-');
    if out.is_empty() { "task".to_string() } else { out.to_string() }
}

/// What the manifest says about a session; `ended` is `(outcome, summary, time)` once it is over.
fn manifest(session: &Session, ended: Option<(&str, &str, u64)>, files: Option<[usize; 3]>) -> Value {
    let mut value = json!({
        "id": session.id,
        "task": session.task,
        "source": session.source,
        "started_utc": iso(session.started),
        "ended_utc": null,
        "outcome": null,
        "summary": null,
    });
    if let Some((outcome, summary, at)) = ended {
        value["ended_utc"] = json!(iso(at));
        value["outcome"] = json!(outcome);
        value["summary"] = json!(summary.chars().take(400).collect::<String>());
    }
    if let Some([screenshots, voice, recordings]) = files {
        value["files"] = json!({ "screenshots": screenshots, "voice": voice, "recordings": recordings });
    }
    value
}

fn write_manifest(session: &Session, ended: Option<(&str, &str, u64)>, files: Option<[usize; 3]>) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(&manifest(session, ended, files)).map_err(std::io::Error::other)?;
    fs::write(session.dir.join(MANIFEST), text)
}

fn count_files(dir: &Path) -> usize {
    fs::read_dir(dir).map(|entries| entries.filter_map(Result::ok).filter(|entry| entry.path().is_file()).count()).unwrap_or(0)
}

/// Makes a new session folder under `base` (never reusing one) and writes its first manifest.
fn begin_in(base: &Path, task: &str, source: &str, seconds: u64) -> Result<Session, ArchiveError> {
    let (date, time) = stamp(seconds);
    let parent = base.join(SESSIONS_FOLDER).join(date);
    fs::create_dir_all(&parent).map_err(write_error)?;
    loop {
        let number = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!("{time}-{number:03}-{}", slug(task));
        let dir = parent.join(&id);
        match fs::create_dir(&dir) {
            Ok(()) => {
                let session = Session { id, dir, task: task.trim().chars().take(200).collect(), source: source.chars().take(40).collect(), started: seconds };
                if let Err(err) = write_manifest(&session, None, None) {
                    log::warn!("could not write the session manifest: {err}");
                }
                return Ok(session);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(write_error(err)),
        }
    }
}

/// Writes the final manifest of a session (outcome, summary, how many files of each kind).
fn end_in(session: &Session, outcome: &str, summary: &str, seconds: u64) -> std::io::Result<()> {
    let files = [Kind::Screenshots, Kind::Voice, Kind::Recordings].map(|kind| count_files(&session.dir.join(kind.folder())));
    write_manifest(session, Some((outcome, summary, seconds)), Some(files))
}

/// Opens a session for a task run: everything saved from now on goes in its folder, until it ends.
/// `source` is who runs the task ("built-in", or an outside agent's name). Returns the session's id,
/// to pass to [`end_session`]. A session still open is ended as "replaced". If the folder cannot be
/// made, saving fails anyway (the archive fails closed), so callers may just log the error.
pub fn begin_session(task: &str, source: &str) -> Result<String, ArchiveError> {
    let base = match root() {
        Ok(root) => root,
        // Unit tests have no app; production builds never take this branch.
        Err(_) if cfg!(test) => return Ok(String::new()),
        Err(err) => return Err(err),
    };
    let at = now();
    let session = begin_in(base, task, source, at)?;
    let id = session.id.clone();
    if let Some(old) = current().replace(session)
        && let Err(err) = end_in(&old, "replaced", "", at)
    {
        log::warn!("could not close session {}: {err}", old.id);
    }
    Ok(id)
}

/// Ends the open session. With `Some(id)` it only ends that session, so a task that finishes late
/// cannot close the next one; with `None` it ends whichever is open (Stop). Does nothing when none
/// is open.
pub fn end_session(id: Option<&str>, outcome: &str, summary: &str) {
    let taken = {
        let mut open = current();
        let matches = open.as_ref().is_some_and(|session| id.is_none_or(|wanted| session.id == wanted));
        if matches { open.take() } else { None }
    };
    if let Some(session) = taken
        && let Err(err) = end_in(&session, outcome, summary, now())
    {
        log::warn!("could not close session {}: {err}", session.id);
    }
}

/// Moves a voice file saved just before a task started (the words that started it) into the open
/// session's `voice/` folder. Returns the new path. If no session is open or the move fails, the
/// file stays where it is.
pub fn adopt(path: &Path) -> Option<PathBuf> {
    if path.as_os_str().is_empty() {
        return None;
    }
    let dir = current().as_ref()?.dir.join(Kind::Voice.folder());
    let target = dir.join(path.file_name()?);
    let moved = fs::create_dir_all(&dir).and_then(|()| fs::rename(path, &target));
    match moved {
        Ok(()) => Some(target),
        Err(err) => {
            log::warn!("could not move {} into the session: {err}", path.display());
            None
        }
    }
}

/// The folder a new file goes in: the open session's folder for its kind, else the day's folder
/// for that kind.
fn dir_in(base: &Path, session: Option<&Session>, kind: Kind, seconds: u64) -> PathBuf {
    match session {
        Some(session) => session.dir.join(kind.folder()),
        None => base.join(kind.folder()).join(stamp(seconds).0),
    }
}

fn path_in(base: &Path, session: Option<&Session>, kind: Kind, label: &str, extension: &str, seconds: u64) -> Result<PathBuf, ArchiveError> {
    let dir = dir_in(base, session, kind, seconds);
    fs::create_dir_all(&dir).map_err(write_error)?;
    let (_, time) = stamp(seconds);
    let number = COUNTER.fetch_add(1, Ordering::Relaxed);
    let label: String = label.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    Ok(dir.join(format!("{time}-{number:05}-{label}.{extension}")))
}

/// Where a new file of this kind goes (inside the open session, else the day's folder; the folder
/// is created). Names are unique, so nothing is ever overwritten. A recorder that streams to disk
/// uses this directly.
pub fn path_for(kind: Kind, label: &str, extension: &str) -> Result<PathBuf, ArchiveError> {
    let base = match root() {
        Ok(root) => root,
        // Unit tests have no app; production builds never take this branch.
        Err(_) if cfg!(test) => return Ok(PathBuf::new()),
        Err(err) => return Err(err),
    };
    let session = current().clone();
    path_in(base, session.as_ref(), kind, label, extension, now())
}

/// Stores one file. `Err` means it was NOT stored, and the caller must not use the data.
pub fn save(kind: Kind, label: &str, extension: &str, bytes: &[u8]) -> Result<PathBuf, ArchiveError> {
    let path = path_for(kind, label, extension)?;
    if path.as_os_str().is_empty() {
        return Ok(path);
    }
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|err| ArchiveError::Write(err.to_string()))?;
    file.write_all(bytes).and_then(|()| file.sync_data()).map_err(|err| ArchiveError::Write(err.to_string()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_right_across_leap_days_and_the_epoch() {
        assert_eq!(stamp(0), ("1970-01-01".into(), "000000".into()));
        assert_eq!(stamp(951_782_400), ("2000-02-29".into(), "000000".into())); // a leap day
        assert_eq!(stamp(1_792_396_752), ("2026-10-19".into(), "075912".into()));
    }

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("rune-archive-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_task_becomes_a_short_file_name_safe_slug() {
        assert_eq!(slug("Open Notepad & write hello!"), "open-notepad-write-hello");
        assert_eq!(slug("  ../..\\ "), "task");
        assert_eq!(slug(""), "task");
        assert!(slug(&"a ".repeat(100)).chars().count() <= SLUG_MAX);
        assert!(!slug("ends with a dash ----").ends_with('-'));
    }

    #[test]
    fn times_are_iso_utc() {
        assert_eq!(iso(1_792_396_752), "2026-10-19T07:59:12Z");
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_session_gets_its_own_folder_and_manifest() {
        let base = scratch("begin");
        let first = begin_in(&base, "Open Notepad", "built-in", 1_792_396_752).unwrap();
        let second = begin_in(&base, "Open Notepad", "built-in", 1_792_396_752).unwrap();
        assert_ne!(first.id, second.id, "two sessions in the same second never share a folder");
        assert!(first.dir.starts_with(base.join("sessions").join("2026-10-19")));
        assert!(first.id.starts_with("075912-") && first.id.ends_with("-open-notepad"));
        let text = fs::read_to_string(first.dir.join(MANIFEST)).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["task"], "Open Notepad");
        assert_eq!(value["source"], "built-in");
        assert_eq!(value["started_utc"], "2026-10-19T07:59:12Z");
        assert!(value["outcome"].is_null());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn files_go_inside_the_session_by_type_or_by_day_when_there_is_none() {
        let base = scratch("paths");
        let session = begin_in(&base, "task", "built-in", 1_792_396_752).unwrap();
        let inside = path_in(&base, Some(&session), Kind::Screenshots, "screen", "jpg", 1_792_396_760).unwrap();
        assert_eq!(inside.parent().unwrap(), session.dir.join("screenshots"));
        assert!(inside.file_name().unwrap().to_string_lossy().starts_with("075920-"));
        let voice = path_in(&base, Some(&session), Kind::Voice, "utterance", "wav", 1_792_396_760).unwrap();
        assert_eq!(voice.parent().unwrap(), session.dir.join("voice"));
        let outside = path_in(&base, None, Kind::Screenshots, "screen", "jpg", 1_792_396_760).unwrap();
        assert_eq!(outside.parent().unwrap(), base.join("screenshots").join("2026-10-19"));
        assert_ne!(inside, outside);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn ending_a_session_records_the_outcome_and_the_file_counts() {
        let base = scratch("end");
        let session = begin_in(&base, "task", "Claude", 1_792_396_752).unwrap();
        for _ in 0..3 {
            let path = path_in(&base, Some(&session), Kind::Screenshots, "screen", "jpg", 1_792_396_753).unwrap();
            fs::write(path, b"x").unwrap();
        }
        let path = path_in(&base, Some(&session), Kind::Voice, "utterance", "wav", 1_792_396_753).unwrap();
        fs::write(path, b"x").unwrap();
        end_in(&session, "succeeded", "Typed hello", 1_792_396_800).unwrap();
        let value: Value = serde_json::from_str(&fs::read_to_string(session.dir.join(MANIFEST)).unwrap()).unwrap();
        assert_eq!(value["outcome"], "succeeded");
        assert_eq!(value["summary"], "Typed hello");
        assert_eq!(value["ended_utc"], "2026-10-19T08:00:00Z");
        assert_eq!(value["files"], json!({ "screenshots": 3, "voice": 1, "recordings": 0 }));
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_file_saved_before_the_session_can_be_moved_into_it() {
        let base = scratch("adopt");
        let before = path_in(&base, None, Kind::Voice, "utterance", "wav", 1_792_396_751).unwrap();
        fs::write(&before, b"voice").unwrap();
        let session = begin_in(&base, "task", "built-in", 1_792_396_752).unwrap();
        let target = session.dir.join("voice").join(before.file_name().unwrap());
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::rename(&before, &target).unwrap();
        assert!(target.is_file() && !before.exists());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn folders_are_the_three_kinds() {
        assert_eq!([Kind::Screenshots, Kind::Voice, Kind::Recordings].map(Kind::folder), ["screenshots", "voice", "recordings"]);
    }
}
