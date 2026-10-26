use std::path::{Component, Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    NotRelative,
    Climbs,
    Outside,
    Missing,
}

impl Refused {
    pub fn why(&self) -> &'static str {
        match self {
            Refused::NotRelative => "the path must be relative to the task folder",
            Refused::Climbs => "the path may not climb out of the task folder",
            Refused::Outside => "the file is outside the task folder",
            Refused::Missing => "there is no such file",
        }
    }
}

pub fn resolve(session: &Path, relative: &str) -> Result<PathBuf, Refused> {
    let path = Path::new(relative);
    if path.is_absolute()
        || relative.starts_with(['/', '\\'])
        || path
            .components()
            .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
    {
        return Err(Refused::NotRelative);
    }
    if relative.split(['/', '\\']).any(|part| part == "..") {
        return Err(Refused::Climbs);
    }
    let root = session.canonicalize().map_err(|_| Refused::Missing)?;
    let joined = root.join(path);
    let real = joined.canonicalize().map_err(|_| Refused::Missing)?;
    if !real.starts_with(&root) {
        return Err(Refused::Outside);
    }
    if !real.is_file() {
        return Err(Refused::Missing);
    }
    Ok(real)
}

pub fn relative_to(session: &Path, stored: &str) -> Option<String> {
    let root = session.canonicalize().ok()?;
    let real = Path::new(stored).canonicalize().ok()?;
    real.strip_prefix(&root)
        .ok()
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rune-flight-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("session/screenshots")).unwrap();
        std::fs::write(dir.join("session/screenshots/a.jpg"), b"x").unwrap();
        std::fs::write(dir.join("secret.txt"), b"nope").unwrap();
        dir
    }

    #[test]
    fn a_file_inside_the_session_folder_is_allowed() {
        let dir = scratch("ok");
        let session = dir.join("session");
        assert!(
            resolve(&session, "screenshots/a.jpg")
                .unwrap()
                .ends_with("a.jpg")
        );
        assert!(
            resolve(&session, "screenshots\\a.jpg").is_ok() || cfg!(unix),
            "Windows separators"
        );
        assert_eq!(
            relative_to(
                &session,
                &session.join("screenshots/a.jpg").display().to_string()
            )
            .as_deref(),
            Some("screenshots/a.jpg")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn absolute_and_climbing_paths_are_refused() {
        let dir = scratch("bad");
        let session = dir.join("session");
        assert_eq!(resolve(&session, "../secret.txt"), Err(Refused::Climbs));
        assert_eq!(
            resolve(&session, "screenshots/../../secret.txt"),
            Err(Refused::Climbs)
        );
        assert_eq!(
            resolve(&session, &dir.join("secret.txt").display().to_string()),
            Err(Refused::NotRelative)
        );
        assert_eq!(resolve(&session, "/etc/passwd"), Err(Refused::NotRelative));
        assert!(matches!(
            resolve(&session, "C:\\Windows\\win.ini"),
            Err(Refused::NotRelative | Refused::Missing)
        ));
        assert_eq!(
            resolve(&session, "screenshots/missing.jpg"),
            Err(Refused::Missing)
        );
        assert_eq!(
            resolve(&session, "screenshots"),
            Err(Refused::Missing),
            "a folder is not a file"
        );
        assert_eq!(
            relative_to(&session, &dir.join("secret.txt").display().to_string()),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leaves_the_folder_is_refused() {
        let dir = scratch("link");
        let session = dir.join("session");
        std::os::unix::fs::symlink(dir.join("secret.txt"), session.join("screenshots/link.jpg"))
            .unwrap();
        assert_eq!(
            resolve(&session, "screenshots/link.jpg"),
            Err(Refused::Outside)
        );
        std::os::unix::fs::symlink(&dir, session.join("up")).unwrap();
        assert_eq!(resolve(&session, "up/secret.txt"), Err(Refused::Outside));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
