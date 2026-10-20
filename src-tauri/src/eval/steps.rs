//! Carrying out a task's setup and teardown. Every path is checked against the jail again here,
//! on top of the check made before the run starts.

use std::{fs, path::PathBuf, time::Duration};

use super::{
    spec::{Step, jail},
    windows,
};

/// The longest a `wait_ms` step may wait.
const MAX_WAIT_MS: u64 = 30_000;

/// Does one step. `roots` are the folders it may touch.
pub fn apply(step: &Step, roots: &[PathBuf]) -> Result<(), String> {
    match step {
        Step::MakeDir { path } => {
            let path = jail(path, roots)?;
            fs::create_dir_all(&path)
                .map_err(|err| format!("could not make {}: {err}", path.display()))
        }
        Step::WriteFile { path, content } => {
            let path = jail(path, roots)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("could not make {}: {err}", parent.display()))?;
            }
            fs::write(&path, content)
                .map_err(|err| format!("could not write {}: {err}", path.display()))
        }
        Step::DeletePath { path } => {
            let path = jail(path, roots)?;
            // The folders themselves are never removed: a task may only clear what is inside them.
            if roots.iter().any(|root| root == &path) {
                return Err(format!(
                    "{} is a root folder and cannot be deleted",
                    path.display()
                ));
            }
            match fs::symlink_metadata(&path) {
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(format!("could not look at {}: {err}", path.display())),
                Ok(meta) if meta.is_dir() => fs::remove_dir_all(&path)
                    .map_err(|err| format!("could not delete {}: {err}", path.display())),
                Ok(_) => fs::remove_file(&path)
                    .map_err(|err| format!("could not delete {}: {err}", path.display())),
            }
        }
        Step::KillProcess { name } => windows::kill_process(name),
        Step::WaitMs { ms } => {
            std::thread::sleep(Duration::from_millis((*ms).min(MAX_WAIT_MS)));
            Ok(())
        }
    }
}

/// Runs all of them; stops at the first that fails.
pub fn apply_all(steps: &[Step], roots: &[PathBuf]) -> Result<(), String> {
    steps.iter().try_for_each(|step| apply(step, roots))
}
