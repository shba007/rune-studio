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

const SESSIONS_FOLDER: &str = "sessions";
const MANIFEST: &str = "session.json";
const SLUG_MAX: usize = 32;

pub fn init(app: &AppHandle) -> Result<PathBuf, String> {
    let result = app
        .path()
        .document_dir()
        .map_err(|err| err.to_string())
        .map(|dir| dir.join(FOLDER))
        .and_then(|root| {
            for kind in [Kind::Screenshots, Kind::Voice, Kind::Recordings] {
                fs::create_dir_all(root.join(kind.folder()))
                    .map_err(|err| format!("{}: {err}", root.display()))?;
            }
            fs::create_dir_all(root.join(SESSIONS_FOLDER))
                .map_err(|err| format!("{}: {err}", root.display()))?;
            Ok(root)
        });
    match &result {
        Ok(root) => log::info!("record folder: {}", root.display()),
        Err(err) => log::error!(
            "record folder unavailable, the agent cannot see or hear until this is fixed: {err}"
        ),
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

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn stamp(seconds: u64) -> (String, String) {
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let in_day = seconds % 86_400;
    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!(
            "{:02}{:02}{:02}",
            in_day / 3_600,
            in_day % 3_600 / 60,
            in_day % 60
        ),
    )
}

#[derive(Debug, Clone)]
struct Session {
    id: String,
    dir: PathBuf,
    task: String,
    source: String,
    started: u64,
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn current() -> MutexGuard<'static, Option<Session>> {
    SESSION.lock().unwrap_or_else(PoisonError::into_inner)
}

fn write_error(err: impl std::fmt::Display) -> ArchiveError {
    ArchiveError::Write(err.to_string())
}

pub(crate) fn iso(seconds: u64) -> String {
    let (date, time) = stamp(seconds);
    format!("{date}T{}:{}:{}Z", &time[0..2], &time[2..4], &time[4..6])
}

fn slug(task: &str) -> String {
    let mut out = String::new();
    for c in task.chars() {
        let c = if c.is_ascii_alphanumeric() {
            c.to_ascii_lowercase()
        } else {
            '-'
        };
        if c != '-' || !(out.is_empty() || out.ends_with('-')) {
            out.push(c);
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(SLUG_MAX).collect();
    let out = out.trim_end_matches('-');
    if out.is_empty() {
        "task".to_string()
    } else {
        out.to_string()
    }
}

fn manifest(
    session: &Session,
    ended: Option<(&str, &str, u64)>,
    files: Option<[usize; 3]>,
) -> Value {
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
        value["files"] =
            json!({ "screenshots": screenshots, "voice": voice, "recordings": recordings });
    }
    value
}

fn write_manifest(
    session: &Session,
    ended: Option<(&str, &str, u64)>,
    files: Option<[usize; 3]>,
) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(&manifest(session, ended, files))
        .map_err(std::io::Error::other)?;
    fs::write(session.dir.join(MANIFEST), text)
}

fn count_files(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_file())
                .count()
        })
        .unwrap_or(0)
}

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
                let session = Session {
                    id,
                    dir,
                    task: task.trim().chars().take(200).collect(),
                    source: source.chars().take(40).collect(),
                    started: seconds,
                };
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

fn end_in(session: &Session, outcome: &str, summary: &str, seconds: u64) -> std::io::Result<()> {
    let files = [Kind::Screenshots, Kind::Voice, Kind::Recordings]
        .map(|kind| count_files(&session.dir.join(kind.folder())));
    write_manifest(session, Some((outcome, summary, seconds)), Some(files))
}

pub fn begin_session(task: &str, source: &str) -> Result<String, ArchiveError> {
    let base = match root() {
        Ok(root) => root,
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

pub fn end_session(id: Option<&str>, outcome: &str, summary: &str) {
    let taken = {
        let mut open = current();
        let matches = open
            .as_ref()
            .is_some_and(|session| id.is_none_or(|wanted| session.id == wanted));
        if matches { open.take() } else { None }
    };
    if let Some(session) = taken
        && let Err(err) = end_in(&session, outcome, summary, now())
    {
        log::warn!("could not close session {}: {err}", session.id);
    }
}

pub fn open_session_dir() -> Option<PathBuf> {
    current().as_ref().map(|session| session.dir.clone())
}

pub fn session_dir(id: &str) -> Option<PathBuf> {
    current()
        .as_ref()
        .filter(|session| session.id == id)
        .map(|session| session.dir.clone())
}

// Insight archives one completed capture without becoming the active desktop
// session or enabling the continuous session recorder.
pub fn save_completed_insight(
    id: &str,
    question: &str,
    answer: &str,
    png: &[u8],
) -> Result<PathBuf, ArchiveError> {
    save_insight_in(root()?, id, question, answer, png, now())
}

fn save_insight_in(
    base: &Path,
    id: &str,
    question: &str,
    answer: &str,
    png: &[u8],
    at: u64,
) -> Result<PathBuf, ArchiveError> {
    let mut session = begin_in(base, question, "screen-insight", at)?;
    session.id = id.to_string();
    let result = (|| {
        let folder = session.dir.join(Kind::Screenshots.folder());
        fs::create_dir(&folder).map_err(write_error)?;
        let pending = folder.join("capture.part");
        let path = folder.join("capture.png");
        fs::write(&pending, png).map_err(write_error)?;
        fs::rename(pending, &path).map_err(write_error)?;
        end_in(&session, "succeeded", answer, at).map_err(write_error)?;
        Ok(path)
    })();
    if result.is_err() {
        // begin_in created this unique directory for this write only.
        let _ = fs::remove_dir_all(&session.dir);
    }
    result
}

pub struct SessionDeletion {
    original: PathBuf,
    staged: PathBuf,
    committed: bool,
}
impl SessionDeletion {
    pub fn finish(mut self) -> Result<(), ArchiveError> {
        self.committed = true;
        fs::remove_dir_all(&self.staged)
            .map_err(|error| ArchiveError::Write(format!("{}: {error}", self.staged.display())))
    }
}
impl Drop for SessionDeletion {
    fn drop(&mut self) {
        if !self.committed
            && let Err(error) = fs::rename(&self.staged, &self.original)
        {
            log::error!("Could not restore session folder after failed deletion: {error}");
        }
    }
}

pub fn stage_session_deletion(
    dir: &Path,
    id: &str,
) -> Result<Option<SessionDeletion>, ArchiveError> {
    stage_deletion_in(root()?, dir, id)
}

fn stage_deletion_in(
    base: &Path,
    dir: &Path,
    id: &str,
) -> Result<Option<SessionDeletion>, ArchiveError> {
    let metadata = match fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(write_error(error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(ArchiveError::Write(
            "Refusing to delete a linked task folder".into(),
        ));
    }
    let root = base
        .join(SESSIONS_FOLDER)
        .canonicalize()
        .map_err(write_error)?;
    let original = dir.canonicalize().map_err(write_error)?;
    let relative = original
        .strip_prefix(&root)
        .map_err(|_| ArchiveError::Write("Task folder is outside Studio's archive".into()))?;
    let parts = relative.components().collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].as_os_str().to_str().and_then(day_number).is_none() {
        return Err(ArchiveError::Write(
            "Refusing to delete a directory that is not one task".into(),
        ));
    }
    let manifest: Value =
        serde_json::from_slice(&fs::read(original.join(MANIFEST)).map_err(write_error)?)
            .map_err(write_error)?;
    if manifest["id"].as_str() != Some(id) {
        return Err(ArchiveError::Write(
            "Task folder belongs to a different task".into(),
        ));
    }
    let number = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staged = original.with_file_name(format!(".deleting-{}-{number}", std::process::id()));
    if staged.exists() {
        return Err(ArchiveError::Write(
            "A task deletion is already pending".into(),
        ));
    }
    fs::rename(&original, &staged).map_err(write_error)?;
    Ok(Some(SessionDeletion {
        original,
        staged,
        committed: false,
    }))
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_index = i64::from((month + 9) % 12);
    let day_of_year = (153 * month_index + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn day_number(name: &str) -> Option<i64> {
    let bytes = name.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return None;
    }
    let (year, month, day) = (
        name[0..4].parse::<i64>().ok()?,
        name[5..7].parse::<u32>().ok()?,
        name[8..10].parse::<u32>().ok()?,
    );
    let days = days_from_civil(year, month, day);
    (civil_from_days(days) == (year, month, day)).then_some(days)
}

pub fn expired_dates(names: &[String], today: &str, keep_days: u32) -> Vec<String> {
    let Some(today) = day_number(today).filter(|_| keep_days > 0) else {
        return Vec::new();
    };
    let cutoff = today - i64::from(keep_days);
    names
        .iter()
        .filter(|name| day_number(name).is_some_and(|day| day < cutoff))
        .cloned()
        .collect()
}

fn purge_in(base: &Path, today: &str, keep_days: u32) -> Vec<String> {
    let sessions = base.join(SESSIONS_FOLDER);
    let Ok(entries) = fs::read_dir(&sessions) else {
        return Vec::new();
    };
    let names: Vec<String> = entries
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .symlink_metadata()
                .is_ok_and(|meta| meta.is_dir())
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    let mut removed = Vec::new();
    for date in expired_dates(&names, today, keep_days) {
        let day_dir = sessions.join(&date);
        let sessions_inside: Vec<String> = fs::read_dir(&day_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path().display().to_string())
                    .collect()
            })
            .unwrap_or_default();
        match fs::remove_dir_all(&day_dir) {
            Ok(()) => {
                log::info!(
                    "record retention: removed {} ({} session folder(s)) older than {keep_days} days",
                    day_dir.display(),
                    sessions_inside.len()
                );
                removed.extend(sessions_inside);
            }
            Err(err) => log::warn!(
                "record retention: could not remove {}: {err}",
                day_dir.display()
            ),
        }
    }
    removed
}

pub fn purge_expired(keep_days: u32) -> Vec<String> {
    if keep_days == 0 {
        return Vec::new();
    }
    match root() {
        Ok(base) => purge_in(base, &stamp(now()).0, keep_days),
        Err(_) => Vec::new(),
    }
}

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

fn dir_in(base: &Path, session: Option<&Session>, kind: Kind, seconds: u64) -> PathBuf {
    match session {
        Some(session) => session.dir.join(kind.folder()),
        None => base.join(kind.folder()).join(stamp(seconds).0),
    }
}

fn path_in(
    base: &Path,
    session: Option<&Session>,
    kind: Kind,
    label: &str,
    extension: &str,
    seconds: u64,
) -> Result<PathBuf, ArchiveError> {
    let dir = dir_in(base, session, kind, seconds);
    fs::create_dir_all(&dir).map_err(write_error)?;
    let (_, time) = stamp(seconds);
    let number = COUNTER.fetch_add(1, Ordering::Relaxed);
    let label: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(dir.join(format!("{time}-{number:05}-{label}.{extension}")))
}

pub fn path_for(kind: Kind, label: &str, extension: &str) -> Result<PathBuf, ArchiveError> {
    let base = match root() {
        Ok(root) => root,
        Err(_) if cfg!(test) => return Ok(PathBuf::new()),
        Err(err) => return Err(err),
    };
    let session = current().clone();
    path_in(base, session.as_ref(), kind, label, extension, now())
}

pub fn save(
    kind: Kind,
    label: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<PathBuf, ArchiveError> {
    let path = path_for(kind, label, extension)?;
    save_to(path, bytes)
}

pub fn save_unscoped_voice(bytes: &[u8]) -> Result<PathBuf, ArchiveError> {
    let path = path_in(root()?, None, Kind::Voice, "utterance", "wav", now())?;
    save_to(path, bytes)
}

fn save_to(path: PathBuf, bytes: &[u8]) -> Result<PathBuf, ArchiveError> {
    if path.as_os_str().is_empty() {
        return Ok(path);
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|err| ArchiveError::Write(err.to_string()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_data())
        .map_err(|err| ArchiveError::Write(err.to_string()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_right_across_leap_days_and_the_epoch() {
        assert_eq!(stamp(0), ("1970-01-01".into(), "000000".into()));
        assert_eq!(stamp(951_782_400), ("2000-02-29".into(), "000000".into()));
        assert_eq!(stamp(1_792_396_752), ("2026-10-19".into(), "075912".into()));
    }

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "rune-archive-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_task_becomes_a_short_file_name_safe_slug() {
        assert_eq!(
            slug("Open Notepad & write hello!"),
            "open-notepad-write-hello"
        );
        assert_eq!(slug("  ../..\\ "), "task");
        assert_eq!(slug(""), "task");
        assert!(slug(&"a ".repeat(100)).chars().count() <= SLUG_MAX);
        assert!(!slug("ends with a dash ----").ends_with('-'));
    }

    #[test]
    fn completed_insight_saves_one_capture_without_replacing_the_active_session() {
        let base = scratch("insight");
        let original = current().as_ref().map(|session| session.id.clone());
        let png = b"native-png-bytes";
        let path = save_insight_in(
            &base,
            "insight-test",
            "What is visible?",
            "A document",
            png,
            1_792_396_752,
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), png);
        assert_eq!(path.file_name().unwrap(), "capture.png");
        let session = path.parent().unwrap().parent().unwrap();
        let manifest: Value =
            serde_json::from_slice(&fs::read(session.join("session.json")).unwrap()).unwrap();
        assert_eq!(manifest["id"], "insight-test");
        assert_eq!(manifest["source"], "screen-insight");
        assert_eq!(
            current().as_ref().map(|session| session.id.clone()),
            original
        );
        assert!(!path.with_extension("part").exists());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn session_deletion_checks_ownership_and_can_roll_back_before_commit() {
        let base = scratch("delete");
        let at = 1_792_396_752;
        let session = begin_in(&base, "Delete this", "built-in", at).unwrap();
        let other = begin_in(&base, "Keep this", "built-in", at).unwrap();
        assert!(stage_deletion_in(&base, &session.dir, "wrong-id").is_err());
        assert!(session.dir.exists());
        let root = base.join(SESSIONS_FOLDER);
        assert!(stage_deletion_in(&base, &root, &session.id).is_err());
        let outside = scratch("outside-delete");
        assert!(stage_deletion_in(&base, &outside, &session.id).is_err());
        {
            let staged = stage_deletion_in(&base, &session.dir, &session.id)
                .unwrap()
                .unwrap();
            assert!(!session.dir.exists());
            drop(staged);
        }
        assert!(session.dir.exists());
        stage_deletion_in(&base, &session.dir, &session.id)
            .unwrap()
            .unwrap()
            .finish()
            .unwrap();
        assert!(!session.dir.exists());
        assert!(other.dir.join(MANIFEST).exists());
        let _ = fs::remove_dir_all(base);
        let _ = fs::remove_dir_all(outside);
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
        assert_ne!(
            first.id, second.id,
            "two sessions in the same second never share a folder"
        );
        assert!(
            first
                .dir
                .starts_with(base.join("sessions").join("2026-10-19"))
        );
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
        let inside = path_in(
            &base,
            Some(&session),
            Kind::Screenshots,
            "screen",
            "jpg",
            1_792_396_760,
        )
        .unwrap();
        assert_eq!(inside.parent().unwrap(), session.dir.join("screenshots"));
        assert!(
            inside
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("075920-")
        );
        let voice = path_in(
            &base,
            Some(&session),
            Kind::Voice,
            "utterance",
            "wav",
            1_792_396_760,
        )
        .unwrap();
        assert_eq!(voice.parent().unwrap(), session.dir.join("voice"));
        let outside = path_in(
            &base,
            None,
            Kind::Screenshots,
            "screen",
            "jpg",
            1_792_396_760,
        )
        .unwrap();
        assert_eq!(
            outside.parent().unwrap(),
            base.join("screenshots").join("2026-10-19")
        );
        assert_ne!(inside, outside);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn ending_a_session_records_the_outcome_and_the_file_counts() {
        let base = scratch("end");
        let session = begin_in(&base, "task", "Claude", 1_792_396_752).unwrap();
        for _ in 0..3 {
            let path = path_in(
                &base,
                Some(&session),
                Kind::Screenshots,
                "screen",
                "jpg",
                1_792_396_753,
            )
            .unwrap();
            fs::write(path, b"x").unwrap();
        }
        let path = path_in(
            &base,
            Some(&session),
            Kind::Voice,
            "utterance",
            "wav",
            1_792_396_753,
        )
        .unwrap();
        fs::write(path, b"x").unwrap();
        end_in(&session, "succeeded", "Typed hello", 1_792_396_800).unwrap();
        let value: Value =
            serde_json::from_str(&fs::read_to_string(session.dir.join(MANIFEST)).unwrap()).unwrap();
        assert_eq!(value["outcome"], "succeeded");
        assert_eq!(value["summary"], "Typed hello");
        assert_eq!(value["ended_utc"], "2026-10-19T08:00:00Z");
        assert_eq!(
            value["files"],
            json!({ "screenshots": 3, "voice": 1, "recordings": 0 })
        );
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
        assert_eq!(
            [Kind::Screenshots, Kind::Voice, Kind::Recordings].map(Kind::folder),
            ["screenshots", "voice", "recordings"]
        );
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn zero_days_keeps_everything() {
        assert!(expired_dates(&names(&["2020-01-01", "2026-10-18"]), "2026-10-19", 0).is_empty());
    }

    #[test]
    fn a_folder_exactly_keep_days_old_is_kept_and_one_day_older_goes() {
        let all = names(&["2026-10-11", "2026-10-12", "2026-10-13", "2026-10-19"]);
        assert_eq!(expired_dates(&all, "2026-10-19", 7), ["2026-10-11"]);
        assert_eq!(
            expired_dates(&all, "2026-10-19", 1),
            ["2026-10-11", "2026-10-12", "2026-10-13"]
        );
        assert_eq!(
            expired_dates(
                &names(&["2025-12-30", "2025-12-31", "2026-01-01"]),
                "2026-01-31",
                31
            ),
            ["2025-12-30"]
        );
        assert_eq!(
            expired_dates(
                &names(&["2024-02-28", "2024-02-29", "2024-03-01"]),
                "2024-03-01",
                1
            ),
            ["2024-02-28"]
        );
    }

    #[test]
    fn names_that_are_not_dates_are_ignored() {
        let odd = names(&[
            "notes",
            "2026-1-1",
            "2026-13-01",
            "2026-02-30",
            "20261001",
            "2026-10-1x",
            "2026-10-11-extra",
            "",
        ]);
        assert!(expired_dates(&odd, "2030-01-01", 1).is_empty());
        assert!(
            expired_dates(&names(&["2020-01-01"]), "not a date", 5).is_empty(),
            "an unreadable today deletes nothing"
        );
    }

    #[test]
    fn only_expired_date_folders_below_sessions_are_removed() {
        let base = std::env::temp_dir().join(format!("rune-retention-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        for dir in [
            "sessions/2026-01-01/120000-001-old",
            "sessions/2026-01-01/130000-002-old-too",
            "sessions/2026-10-19/120000-003-new",
            "sessions/not-a-date/x",
            "screenshots/2026-01-01",
            "voice/2026-01-01",
        ] {
            fs::create_dir_all(base.join(dir)).unwrap();
        }
        fs::write(
            base.join("sessions/2026-01-01/120000-001-old/shot.jpg"),
            b"x",
        )
        .unwrap();
        fs::write(base.join("sessions/readme.txt"), b"x").unwrap();

        let mut removed = purge_in(&base, "2026-10-19", 30);
        removed.sort();
        let expected: Vec<String> = ["120000-001-old", "130000-002-old-too"]
            .iter()
            .map(|dir| {
                base.join("sessions")
                    .join("2026-01-01")
                    .join(dir)
                    .display()
                    .to_string()
            })
            .collect();
        assert_eq!(removed, expected);
        assert!(!base.join("sessions/2026-01-01").exists());
        for kept in [
            "sessions/2026-10-19/120000-003-new",
            "sessions/not-a-date/x",
            "screenshots/2026-01-01",
            "voice/2026-01-01",
            "sessions/readme.txt",
        ] {
            assert!(base.join(kept).exists(), "{kept} must survive");
        }
        assert!(
            purge_in(&base, "2026-10-19", 0).is_empty(),
            "0 days removes nothing"
        );
        assert!(base.join("sessions/2026-10-19").exists());
        let _ = fs::remove_dir_all(&base);
    }
}
