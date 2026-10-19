//! Judging a run by looking at the machine. Success never comes from what the model says: the
//! model's own claim is recorded next to the result, because the disagreement between the two is
//! the data the verifier will need.

use std::path::Path;

use super::spec::{Check, Expect, Task};

/// What the checkers can see besides files: windows and processes. Windows-only in practice; tests
/// give it a fake.
pub trait Machine {
    /// The titles of the visible top-level windows.
    fn window_titles(&self) -> Result<Vec<String>, String>;
    /// The names of the running programs (`notepad.exe`).
    fn process_names(&self) -> Result<Vec<String>, String>;
}

/// For systems where windows and processes cannot be listed.
pub struct Unsupported;

impl Machine for Unsupported {
    fn window_titles(&self) -> Result<Vec<String>, String> {
        Err("listing windows is not supported on this system".into())
    }

    fn process_names(&self) -> Result<Vec<String>, String> {
        Err("listing processes is not supported on this system".into())
    }
}

/// The text of a file, whatever Notepad saved it as: UTF-8 (with or without a byte order mark) or UTF-16.
pub fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|err| format!("could not read {}: {err}", path.display()))?;
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// Whether the condition holds right now. `Err` means it could not be decided (an unreadable
/// file, windows that cannot be listed); the runner counts that as not passed and says why.
pub fn evaluate(check: &Check, machine: &dyn Machine) -> Result<bool, String> {
    Ok(match check {
        Check::FileExists { path } => Path::new(path).is_file(),
        Check::DirExists { path } => Path::new(path).is_dir(),
        Check::FileAbsent { path } => !Path::new(path).exists(),
        Check::FileContains { path, text } => {
            let path = Path::new(path);
            path.is_file() && read_text(path)?.contains(text.as_str())
        }
        Check::AnyWindowTitleContains { text } => machine.window_titles()?.iter().any(|title| title.contains(text.as_str())),
        Check::ProcessRunning { name } => machine.process_names()?.iter().any(|running| running.eq_ignore_ascii_case(name)),
        Check::All { checks } => {
            for check in checks {
                if !evaluate(check, machine)? {
                    return Ok(false);
                }
            }
            true
        }
        Check::Any { checks } => {
            let mut first_problem = None;
            for check in checks {
                match evaluate(check, machine) {
                    Ok(true) => return Ok(true),
                    Ok(false) => {}
                    Err(problem) => {
                        first_problem.get_or_insert(problem);
                    }
                }
            }
            // One alternative that cannot be told is not "no" when none of the others held.
            return first_problem.map_or(Ok(false), Err);
        }
        Check::Not { check } => !evaluate(check, machine)?,
    })
}

/// Whether the run counts as passed. For an ordinary task the checker alone decides, whatever the
/// model said. For an `expect: failure` task the checker must say nothing bad happened **and** the
/// run must have ended in one of the accepted ways (it gave up, asked, or was stopped), not by
/// claiming success, running out of time, or the like.
pub fn passed(task: &Task, check_passed: bool, outcome: &str) -> bool {
    match task.expect {
        Expect::Success => check_passed,
        Expect::Failure => check_passed && task.accepted_outcomes().iter().any(|accepted| accepted == outcome),
    }
}
