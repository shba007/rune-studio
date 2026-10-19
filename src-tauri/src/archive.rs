//! The record of what the agent saw and heard: every screenshot it looks at, every recording of
//! the user's voice, and (when screen recording exists) every video, kept in the user's Documents
//! folder under `Rune Studio`:
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
//! no screenshot and no utterance is acted on. Dates and times in names are UTC.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

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

/// Where a new file of this kind goes (the day's folder is created). Names are unique, so nothing
/// is ever overwritten. A recorder that streams to disk uses this directly.
pub fn path_for(kind: Kind, label: &str, extension: &str) -> Result<PathBuf, ArchiveError> {
    let base = match root() {
        Ok(root) => root,
        // Unit tests have no app; production builds never take this branch.
        Err(_) if cfg!(test) => return Ok(PathBuf::new()),
        Err(err) => return Err(err),
    };
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0);
    let (date, time) = stamp(seconds);
    let dir = base.join(kind.folder()).join(date);
    fs::create_dir_all(&dir).map_err(|err| ArchiveError::Write(err.to_string()))?;
    let number = COUNTER.fetch_add(1, Ordering::Relaxed);
    let label: String = label.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    Ok(dir.join(format!("{time}-{number:05}-{label}.{extension}")))
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

    #[test]
    fn folders_are_the_three_kinds() {
        assert_eq!([Kind::Screenshots, Kind::Voice, Kind::Recordings].map(Kind::folder), ["screenshots", "voice", "recordings"]);
    }
}
