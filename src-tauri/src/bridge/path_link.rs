//! Puts `rune-studio-cli` on the user's PATH on macOS and Linux.
//!
//! The Windows installer does this itself. A macOS app bundle and an AppImage have no install
//! step, so the app links the CLI that sits next to it into `~/.local/bin` the first time it runs
//! (and again if the app is moved). A deb or rpm install puts the CLI in `/usr/bin` and needs
//! nothing. An existing file of that name that is not our link is never touched.

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

const NAME: &str = "rune-studio-cli";

/// Where the link goes, given the user's home directory.
fn link_path(home: &Path) -> PathBuf {
    home.join(".local").join("bin").join(NAME)
}

/// Makes `link` point at `target`. Returns whether it changed anything.
fn point_link_at(link: &Path, target: &Path) -> std::io::Result<bool> {
    if let Ok(existing) = fs::read_link(link) {
        if existing == target {
            return Ok(false);
        }
        fs::remove_file(link)?; // our own link, left over from where the app used to be
    } else if link.exists() {
        return Ok(false); // somebody else's file: leave it alone
    }
    if let Some(dir) = link.parent() {
        fs::create_dir_all(dir)?;
    }
    symlink(target, link)?;
    Ok(true)
}

pub fn ensure() {
    // An AppImage runs from a temporary mount that changes every launch; a link would go stale.
    if std::env::var_os("APPIMAGE").is_some() {
        return;
    }
    let Ok(app) = std::env::current_exe().and_then(fs::canonicalize) else {
        return;
    };
    let cli = app.with_file_name(NAME);
    // A package install already put it in /usr/bin.
    if !cli.exists() || cli.starts_with("/usr/") {
        return;
    }
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    match point_link_at(&link_path(Path::new(&home)), &cli) {
        Ok(true) => log::info!("{NAME} linked into ~/.local/bin"),
        Ok(false) => {}
        Err(err) => log::warn!("could not link {NAME} into ~/.local/bin: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rune-path-link-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_new_link_is_made_and_a_moved_app_repoints_it() {
        let dir = scratch("repoint");
        let (first, second) = (dir.join("a"), dir.join("b"));
        let link = link_path(&dir);
        assert!(point_link_at(&link, &first).unwrap());
        assert_eq!(fs::read_link(&link).unwrap(), first);
        assert!(!point_link_at(&link, &first).unwrap());
        assert!(point_link_at(&link, &second).unwrap());
        assert_eq!(fs::read_link(&link).unwrap(), second);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn someone_elses_file_is_left_alone() {
        let dir = scratch("keep");
        let link = link_path(&dir);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        fs::write(&link, "mine").unwrap();
        assert!(!point_link_at(&link, &dir.join("a")).unwrap());
        assert_eq!(fs::read_to_string(&link).unwrap(), "mine");
        let _ = fs::remove_dir_all(dir);
    }
}
