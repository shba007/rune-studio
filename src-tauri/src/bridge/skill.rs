//! Packages `docs/AGENT_GUIDE.md` as a Claude Code skill, so one command gives an agent both the
//! tools (the MCP server) and the know-how (the skill).
//!
//! The guide stays the single source of truth. A skill loads in layers: a short `SKILL.md` is
//! read first and the detail files only when needed, so the guide is split by its numbered
//! sections. A test fails if a section is not placed in exactly one file.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const GUIDE: &str = include_str!("../../../docs/AGENT_GUIDE.md");

const NAME: &str = "rune-studio";
/// What makes an agent reach for the skill. Keep it about the user's request, not the product.
const DESCRIPTION: &str = "Use when the user asks you to do something on their Windows PC with the screen, mouse or keyboard: open apps, click, type, press keys, or read what is on screen. Works through the rune_ tools from Rune Studio.";

/// Which guide sections go in which file (by their number in the guide).
const SKILL_SECTIONS: [u32; 4] = [1, 2, 3, 9];
const TOOLS_SECTIONS: [u32; 5] = [4, 5, 6, 7, 8];
const RECIPES_SECTIONS: [u32; 2] = [11, 12];
const ERRORS_SECTIONS: [u32; 2] = [10, 13];

struct Section {
    number: u32,
    /// The heading without its number, e.g. "Tools".
    title: String,
    body: String,
}

/// The text before the first numbered section, and the numbered sections.
fn split(guide: &str) -> (String, Vec<Section>) {
    let mut intro = String::new();
    let mut sections: Vec<Section> = Vec::new();
    for line in guide.lines() {
        if let Some(heading) = line.strip_prefix("## ")
            && let Some((number, title)) = heading.split_once(". ")
            && let Ok(number) = number.parse()
        {
            sections.push(Section { number, title: title.to_string(), body: String::new() });
        } else if let Some(section) = sections.last_mut() {
            section.body.push_str(line);
            section.body.push('\n');
        } else {
            intro.push_str(line);
            intro.push('\n');
        }
    }
    (intro, sections)
}

fn render(sections: &[Section], numbers: &[u32]) -> String {
    let mut out = String::new();
    for number in numbers {
        if let Some(section) = sections.iter().find(|section| section.number == *number) {
            out.push_str(&format!("## {}\n{}\n", section.title, section.body.trim_end()));
            out.push('\n');
        }
    }
    out
}

/// The skill's files as (file name, content).
pub fn files(guide: &str) -> Vec<(&'static str, String)> {
    let (intro, sections) = split(guide);
    let intro = intro.replacen("# Rune Studio: guide for agents", "# Rune Studio", 1);
    let skill = format!(
        "---\nname: {NAME}\ndescription: {DESCRIPTION}\n---\n\n{}\n{}## More detail\n\nRead these when you need them:\n\n- `tools.md`: every tool, coordinates, typing and keys, opening apps, waiting.\n- `recipes.md`: worked examples and a checklist of good habits.\n- `errors.md`: every error code and what to do, and what you cannot do yet.\n",
        intro.trim_end(),
        {
            let mut body = String::from("\n");
            body.push_str(&render(&sections, &SKILL_SECTIONS));
            body
        }
    );
    let page = |title: &str, numbers: &[u32]| format!("# {title}\n\n{}", render(&sections, numbers));
    vec![
        ("SKILL.md", skill),
        ("tools.md", page("Rune Studio: tools", &TOOLS_SECTIONS)),
        ("recipes.md", page("Rune Studio: recipes", &RECIPES_SECTIONS)),
        ("errors.md", page("Rune Studio: errors and limits", &ERRORS_SECTIONS)),
    ]
}

/// `~/.claude/skills/rune-studio`, where Claude Code looks for a user's own skills.
pub fn default_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?).join(".claude").join("skills").join(NAME))
}

/// Writes the skill into `dir`, replacing an older copy. Returns the files written.
pub fn install(dir: &Path, guide: &str) -> io::Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut written = Vec::new();
    for (name, content) in files(guide) {
        let path = dir.join(name);
        fs::write(&path, content)?;
        written.push(path);
    }
    Ok(written)
}

/// Removes the skill. Missing is fine.
pub fn remove(dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(dir) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guide_section_lands_in_exactly_one_file() {
        let (_, sections) = split(GUIDE);
        let placed: Vec<u32> = [SKILL_SECTIONS.as_slice(), &TOOLS_SECTIONS, &RECIPES_SECTIONS, &ERRORS_SECTIONS].concat();
        for section in &sections {
            let count = placed.iter().filter(|number| **number == section.number).count();
            assert_eq!(count, 1, "guide section {} ({}) is placed {count} times", section.number, section.title);
        }
        for number in &placed {
            assert!(sections.iter().any(|section| section.number == *number), "no guide section {number}");
        }
    }

    #[test]
    fn the_skill_has_frontmatter_and_points_at_its_detail_files() {
        let files = files(GUIDE);
        let skill = &files.iter().find(|(name, _)| *name == "SKILL.md").unwrap().1;
        assert!(skill.starts_with("---\nname: rune-studio\ndescription: Use when"));
        assert!(skill.contains("## The loop"));
        assert!(skill.contains("## Safety"));
        for detail in ["tools.md", "recipes.md", "errors.md"] {
            assert!(skill.contains(detail));
            assert!(files.iter().any(|(name, _)| *name == detail));
        }
    }

    #[test]
    fn nothing_from_the_guide_is_lost_in_the_split() {
        let all: String = files(GUIDE).iter().map(|(_, content)| content.as_str()).collect();
        for tool in ["rune_begin", "rune_observe", "rune_click", "rune_type_text", "rune_press_keys", "rune_open_app", "rune_wait", "rune_status", "rune_done"] {
            assert!(all.contains(tool), "{tool} is missing from the skill files");
        }
        for code in ["stopped_by_user", "not_enabled", "out_of_range"] {
            assert!(all.contains(code), "{code} is missing from the skill files");
        }
    }

    #[test]
    fn the_skill_does_not_talk_about_section_numbers() {
        for (name, content) in files(GUIDE) {
            assert!(!content.contains("section "), "{name} still refers to a numbered section");
            assert!(!content.lines().any(|line| line.starts_with("## ") && line[3..].starts_with(|c: char| c.is_ascii_digit())), "{name} has a numbered heading");
        }
    }

    #[test]
    fn install_writes_all_files_and_remove_deletes_them() {
        let dir = std::env::temp_dir().join(format!("rune-skill-test-{}", std::process::id()));
        let written = install(&dir, GUIDE).unwrap();
        assert_eq!(written.len(), 4);
        assert!(dir.join("SKILL.md").exists());
        // Installing again replaces the files.
        install(&dir, GUIDE).unwrap();
        remove(&dir).unwrap();
        assert!(!dir.exists());
        remove(&dir).unwrap(); // already gone is fine
    }
}
