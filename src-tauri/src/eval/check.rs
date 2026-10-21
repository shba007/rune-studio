use std::path::Path;

use super::spec::{Check, Expect, Task};

pub trait Machine {
    fn window_titles(&self) -> Result<Vec<String>, String>;
    fn process_names(&self) -> Result<Vec<String>, String>;
}

#[cfg(test)]
pub struct Unsupported;

#[cfg(test)]
impl Machine for Unsupported {
    fn window_titles(&self) -> Result<Vec<String>, String> {
        Err("listing windows is not supported on this system".into())
    }

    fn process_names(&self) -> Result<Vec<String>, String> {
        Err("listing processes is not supported on this system".into())
    }
}

pub fn read_text(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|err| format!("could not read {}: {err}", path.display()))?;
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

pub fn evaluate(check: &Check, machine: &dyn Machine) -> Result<bool, String> {
    Ok(match check {
        Check::FileExists { path } => Path::new(path).is_file(),
        Check::DirExists { path } => Path::new(path).is_dir(),
        Check::FileAbsent { path } => !Path::new(path).exists(),
        Check::FileContains { path, text } => {
            let path = Path::new(path);
            path.is_file() && read_text(path)?.contains(text.as_str())
        }
        Check::AnyWindowTitleContains { text } => machine
            .window_titles()?
            .iter()
            .any(|title| title.contains(text.as_str())),
        Check::ProcessRunning { name } => machine
            .process_names()?
            .iter()
            .any(|running| running.eq_ignore_ascii_case(name)),
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
            return first_problem.map_or(Ok(false), Err);
        }
        Check::Not { check } => !evaluate(check, machine)?,
    })
}

pub fn passed(task: &Task, check_passed: bool, outcome: &str) -> bool {
    match task.expect {
        Expect::Success => check_passed,
        Expect::Failure => {
            check_passed
                && task
                    .accepted_outcomes()
                    .iter()
                    .any(|accepted| accepted == outcome)
        }
    }
}
