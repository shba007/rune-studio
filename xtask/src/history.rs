//! Lossless structural import of historical notes. Changelogen renders Markdown.
use crate::{Result, canonical_notes};
use semver::Version;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalRelease {
    pub version: String,
    pub compare_from: Option<String>,
    pub entries: Vec<HistoricalEntry>,
    pub contributors: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct HistoricalEntry {
    pub group: String,
    pub text: String,
}

pub fn import_history(markdown: &str) -> Result<Vec<HistoricalRelease>> {
    let normalized = markdown.replace("\r\n", "\n");
    let mut blocks: Vec<(String, String)> = Vec::new();
    for line in normalized.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let version = heading.strip_prefix('v').unwrap_or(heading);
            Version::parse(version)?;
            blocks.push((version.into(), String::new()));
        } else if let Some((_, body)) = blocks.last_mut() {
            body.push_str(line);
            body.push('\n');
        } else if !line.is_empty() && line != "# Changelog" {
            return Err("Unexpected changelog preamble; refusing to discard text".into());
        }
    }
    let mut seen = std::collections::BTreeMap::new();
    let mut releases = Vec::new();
    for (version, body) in blocks {
        let canonical = canonical_notes(&body);
        if let Some(previous) = seen.get(&version) {
            if previous != &canonical {
                return Err(format!("Conflicting historical sections for {version}").into());
            }
            continue;
        }
        seen.insert(version.clone(), canonical);
        let mut release = HistoricalRelease {
            version,
            compare_from: None,
            entries: Vec::new(),
            contributors: Vec::new(),
        };
        let mut group = "Maintenance".to_string();
        for line in body.lines().filter(|line| !line.is_empty()) {
            if let Some(heading) = line.strip_prefix("### ") {
                group = heading.to_string();
            } else if let Some(text) = line.strip_prefix("- ") {
                if group.contains("Contributors") {
                    release.contributors.push(text.into());
                } else {
                    release.entries.push(HistoricalEntry {
                        group: group.clone(),
                        text: text.into(),
                    });
                }
            } else if line.to_ascii_lowercase().starts_with("[compare changes](") {
                let range = line
                    .split_once("/compare/")
                    .ok_or("Invalid historical compare link")?
                    .1;
                let from = range
                    .split_once("...")
                    .ok_or("Invalid historical compare range")?
                    .0;
                Version::parse(from.strip_prefix('v').unwrap_or(from))?;
                release.compare_from = Some(from.into());
            } else {
                return Err(format!(
                    "Unrecognized historical Markdown in {}: {line}",
                    release.version
                )
                .into());
            }
        }
        if release.entries.is_empty() {
            return Err(format!("Empty historical release {}", release.version).into());
        }
        releases.push(release);
    }
    if releases.is_empty() {
        return Err("No historical release sections".into());
    }
    Ok(releases)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_metadata_and_collapses_only_identical_duplicate_sections() {
        let section = "## v0.0.1\r\n\r\n### Features\r\n- Historical Cycle 2 wording.\r\n\r\n### Contributors\r\n- Author\r\n";
        let imported =
            import_history(&format!("# Changelog\r\n\r\n{section}\r\n{section}")).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].entries[0].text, "Historical Cycle 2 wording.");
        assert_eq!(imported[0].contributors, ["Author"]);
        assert!(
            import_history("## v0.0.1\n- First note.\n## v0.0.1\n- Different note.\n").is_err()
        );
    }
    #[test]
    fn refuses_to_drop_unrecognized_text_or_invent_comparison_baselines() {
        assert!(
            import_history("## v0.0.1\nParagraph with public compatibility guidance.\n").is_err()
        );
        assert!(import_history("## v0.0.1\n").is_err());
        let imported = import_history("## v0.0.2\n\n[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.1...v0.0.2)\n\n### Fixes\n- Preserve data.\n## v0.0.1\n- Initial behavior.\n").unwrap();
        assert_eq!(imported[0].compare_from.as_deref(), Some("v0.0.1"));
        assert_eq!(imported[1].compare_from, None);
    }
    #[test]
    fn historical_extraction_retains_old_ids_without_weakening_new_note_policy() {
        let notes = "## v0.0.1\n\n### Features\n\n- Historical M0 shell behavior.\n";
        let version = Version::parse("0.0.1").unwrap();
        assert!(crate::release_notes(notes, &version).is_err());
        assert!(
            crate::historical_release_notes(notes, &version)
                .unwrap()
                .contains("M0")
        );
        assert!(crate::historical_release_notes("## v0.0.1\n", &version).is_err());
        assert!(crate::historical_release_notes(&format!("{notes}\n{notes}"), &version).is_err());
    }
}
