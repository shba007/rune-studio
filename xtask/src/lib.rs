//! Minimal release integration: manifest synchronization and one exact note extractor.
//! Conventional-commit parsing, version decisions and rendering belong to pinned changelogen.
use semver::Version;
use std::{fs, path::Path};
use toml_edit::{DocumentMut, value};
pub mod history;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Only CRLF/LF and surrounding blank newline delimiters are transport-normalized.
/// Markdown, contributor links, spaces and wording remain significant.
pub fn canonical_notes(text: &str) -> String {
    text.replace("\r\n", "\n").trim_matches('\n').to_string()
}

fn private_or_placeholder(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words
        .iter()
        .any(|w| matches!(*w, "todo" | "tbd" | "placeholder" | "unreleased"))
        || words.windows(2).any(|pair| {
            matches!(
                pair[0],
                "m" | "k" | "l" | "r" | "inf" | "kf" | "wp" | "cycle" | "milestone"
            ) && pair[1].chars().all(|c| c.is_ascii_digit())
        })
        || words.iter().any(|w| {
            let (prefix, number) =
                w.split_at(w.find(|c: char| c.is_ascii_digit()).unwrap_or(w.len()));
            !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit())
                && matches!(prefix, "m" | "k" | "l" | "r" | "inf" | "kf" | "wp")
        })
        || lower.contains("ci(release):")
}

fn sections(changelog: &str) -> Vec<(Version, usize, usize)> {
    let mut offset = 0;
    let mut result = Vec::new();
    for line in changelog.split_inclusive('\n') {
        if let Some(title) = line.trim_end().strip_prefix("## ")
            && let Ok(version) = Version::parse(title.strip_prefix('v').unwrap_or(title))
        {
            result.push((version, offset, offset + line.len()));
        }
        offset += line.len();
    }
    result
}

pub fn release_notes(changelog: &str, version: &Version) -> Result<String> {
    section_notes(changelog, version, true)
}

/// One-time historical formatting retains reviewed wording, including old IDs.
/// Normal preparation/publication continues to use the strict release_notes API.
pub fn historical_release_notes(changelog: &str, version: &Version) -> Result<String> {
    section_notes(changelog, version, false)
}

fn section_notes(
    changelog: &str,
    version: &Version,
    enforce_public_policy: bool,
) -> Result<String> {
    let parts = sections(changelog);
    if parts.first().map(|p| &p.0) != Some(version)
        || parts.iter().filter(|p| &p.0 == version).count() != 1
    {
        return Err(
            format!("First CHANGELOG version section must uniquely match {version}").into(),
        );
    }
    let notes =
        canonical_notes(&changelog[parts[0].2..parts.get(1).map_or(changelog.len(), |p| p.1)]);
    let substantive = notes
        .lines()
        .take_while(|line| !line.contains("Contributors"))
        .any(|l| l.trim_start().starts_with("- ") && l.trim().len() > 2);
    if !substantive || enforce_public_policy && private_or_placeholder(&notes) {
        return Err(format!(
            "CHANGELOG {version} has empty, placeholder or internal planning notes"
        )
        .into());
    }
    Ok(format!("{notes}\n"))
}

pub fn version(root: &Path) -> Result<Version> {
    let json: serde_json::Value = serde_json::from_slice(&fs::read(root.join("package.json"))?)?;
    Ok(Version::parse(
        json["version"]
            .as_str()
            .ok_or("Missing package.json version")?,
    )?)
}

pub fn sync_manifests(root: &Path, next: &Version) -> Result<()> {
    let text = fs::read_to_string(root.join("package.json"))?;
    let mut json: serde_json::Value = serde_json::from_str(&text)?;
    json["version"] = serde_json::Value::String(next.to_string());
    // Preserve source order and all unrelated JSON formatting.
    let old = version(root)?.to_string();
    let package = text.replacen(
        &format!("\"version\": \"{old}\""),
        &format!("\"version\": \"{}\"", next),
        1,
    );
    if serde_json::from_str::<serde_json::Value>(&package)? != json {
        return Err("Unsupported package.json version formatting".into());
    }
    let mut cargo =
        fs::read_to_string(root.join("src-tauri/Cargo.toml"))?.parse::<DocumentMut>()?;
    cargo["package"]["version"] = value(next.to_string());
    let mut lock = fs::read_to_string(root.join("src-tauri/Cargo.lock"))?.parse::<DocumentMut>()?;
    let packages = lock["package"]
        .as_array_of_tables_mut()
        .ok_or("Cargo.lock package list missing")?;
    let package_lock = packages
        .iter_mut()
        .find(|p| p["name"].as_str() == Some("rune-studio"))
        .ok_or("Cargo.lock Studio package missing")?;
    package_lock["version"] = value(next.to_string());
    fs::write(root.join("package.json"), package)?;
    fs::write(root.join("src-tauri/Cargo.toml"), cargo.to_string())?;
    fs::write(root.join("src-tauri/Cargo.lock"), lock.to_string())?;
    Ok(())
}

pub fn validate_versions(root: &Path) -> Result<()> {
    let version = version(root)?.to_string();
    let cargo = fs::read_to_string(root.join("src-tauri/Cargo.toml"))?.parse::<DocumentMut>()?;
    let lock = fs::read_to_string(root.join("src-tauri/Cargo.lock"))?.parse::<DocumentMut>()?;
    if cargo["package"]["version"].as_str() != Some(&version)
        || !lock["package"]
            .as_array_of_tables()
            .ok_or("Missing lock packages")?
            .iter()
            .any(|p| {
                p["name"].as_str() == Some("rune-studio") && p["version"].as_str() == Some(&version)
            })
    {
        return Err("package.json, Cargo.toml and Cargo.lock versions must agree".into());
    }
    let tauri: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("src-tauri/tauri.conf.json"))?)?;
    if tauri["version"] != "../package.json" {
        return Err("Tauri must use package.json version".into());
    }
    release_notes(
        &fs::read_to_string(root.join("CHANGELOG.md"))?,
        &Version::parse(&version)?,
    )?;
    Ok(())
}

pub fn validate_event(root: &Path, event: &serde_json::Value) -> Result<()> {
    validate_versions(root)?;
    let version = version(root)?;
    let notes = release_notes(&fs::read_to_string(root.join("CHANGELOG.md"))?, &version)?;
    if event["release"]["tag_name"] != format!("v{version}")
        || event["release"]["body"].as_str().map(canonical_notes) != Some(canonical_notes(&notes))
    {
        return Err("Published version/body does not match the prepared CHANGELOG section".into());
    }
    Ok(())
}
