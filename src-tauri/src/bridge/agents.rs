//! Where other agents keep their MCP settings, and how to add or remove Rune Studio there.
//!
//! Claude Code has its own command (`claude mcp add`); Antigravity reads a JSON file. The JSON
//! edits are pure functions on text, so they are tested without touching a real config, and they
//! keep everything else in the file as it was.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value, json};

const SERVER: &str = "rune-studio";

/// Antigravity's global settings and skills (the App and IDE read these).
pub struct Antigravity {
    /// `~/.gemini/config/mcp_config.json`
    pub config: PathBuf,
    /// `~/.gemini/config/skills/rune-studio`
    pub skill: PathBuf,
    /// The command-line version keeps skills elsewhere; present only if it is installed.
    pub cli_skill: Option<PathBuf>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
}

pub fn antigravity() -> Option<Antigravity> {
    let gemini = home()?.join(".gemini");
    let cli = gemini.join("antigravity-cli");
    Some(Antigravity {
        config: gemini.join("config").join("mcp_config.json"),
        skill: gemini.join("config").join("skills").join(SERVER),
        cli_skill: cli.is_dir().then(|| cli.join("skills").join(SERVER)),
    })
}

/// Adds (or replaces) Rune Studio in an MCP settings file's text. Everything else is kept.
/// An empty file counts as `{}`. A file that is not a JSON object is refused, never overwritten.
pub fn add_server(existing: &str, exe: &str) -> Result<String, String> {
    let mut root = parse_root(existing)?;
    let servers = root.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(servers) = servers else {
        return Err("\"mcpServers\" in your settings is not an object".into());
    };
    servers.insert(SERVER.into(), json!({ "command": exe, "args": ["mcp"] }));
    to_text(root)
}

/// Removes Rune Studio from an MCP settings file's text. Returns `None` if it was not there.
pub fn remove_server(existing: &str) -> Result<Option<String>, String> {
    let mut root = parse_root(existing)?;
    let removed = match root.get_mut("mcpServers") {
        Some(Value::Object(servers)) => servers.remove(SERVER).is_some(),
        _ => false,
    };
    if removed { to_text(root).map(Some) } else { Ok(None) }
}

fn parse_root(text: &str) -> Result<Map<String, Value>, String> {
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("the settings file is not a JSON object".into()),
        Err(err) => Err(format!("the settings file is not valid JSON ({err})")),
    }
}

fn to_text(root: Map<String, Value>) -> Result<String, String> {
    serde_json::to_string_pretty(&Value::Object(root)).map(|text| text + "\n").map_err(|err| err.to_string())
}

/// Reads a settings file, applies `edit`, and writes the result, keeping the old copy next to it
/// as `<name>.bak`. A missing file is treated as empty. `edit` returning `None` means no change.
pub fn edit_file(path: &Path, edit: impl FnOnce(&str) -> Result<Option<String>, String>) -> Result<bool, String> {
    let current = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(format!("cannot read {}: {err}", path.display())),
    };
    let Some(updated) = edit(&current)? else {
        return Ok(false);
    };
    if updated == current {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    }
    if !current.trim().is_empty() {
        let mut backup = path.as_os_str().to_owned();
        backup.push(".bak");
        fs::write(&backup, &current).map_err(|err| format!("cannot write a backup of {}: {err}", path.display()))?;
    }
    fs::write(path, updated).map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Rune Studio\rune-studio.exe";

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn adds_to_an_empty_or_missing_file() {
        for start in ["", "  \n", "{}", r#"{"mcpServers": {}}"#] {
            let result = parsed(&add_server(start, EXE).unwrap());
            assert_eq!(result["mcpServers"]["rune-studio"]["command"], EXE);
            assert_eq!(result["mcpServers"]["rune-studio"]["args"], json!(["mcp"]));
        }
    }

    #[test]
    fn keeps_other_servers_and_settings() {
        let start = r#"{"theme": "dark", "mcpServers": {"other": {"command": "node", "args": ["x.js"], "env": {"K": "v"}}}}"#;
        let result = parsed(&add_server(start, EXE).unwrap());
        assert_eq!(result["theme"], "dark");
        assert_eq!(result["mcpServers"]["other"]["env"]["K"], "v");
        assert_eq!(result["mcpServers"]["rune-studio"]["command"], EXE);
    }

    #[test]
    fn running_it_twice_changes_nothing_and_replaces_an_old_path() {
        let once = add_server("", EXE).unwrap();
        assert_eq!(add_server(&once, EXE).unwrap(), once);
        let moved = parsed(&add_server(&once, r"D:\New\rune-studio.exe").unwrap());
        assert_eq!(moved["mcpServers"]["rune-studio"]["command"], r"D:\New\rune-studio.exe");
        assert_eq!(moved["mcpServers"].as_object().unwrap().len(), 1);
    }

    #[test]
    fn refuses_files_it_cannot_understand() {
        assert!(add_server("not json", EXE).is_err());
        assert!(add_server("[1, 2]", EXE).is_err());
        assert!(add_server(r#"{"mcpServers": []}"#, EXE).is_err());
    }

    #[test]
    fn removal_takes_out_only_rune_studio() {
        let start = add_server(r#"{"mcpServers": {"other": {"command": "node"}}}"#, EXE).unwrap();
        let after = parsed(&remove_server(&start).unwrap().unwrap());
        assert!(after["mcpServers"].get("rune-studio").is_none());
        assert_eq!(after["mcpServers"]["other"]["command"], "node");
        assert_eq!(remove_server(&start.replace("rune-studio", "gone")).unwrap(), None);
        assert_eq!(remove_server("").unwrap(), None);
    }

    #[test]
    fn edit_file_writes_a_backup_and_skips_no_ops() {
        let dir = std::env::temp_dir().join(format!("rune-agents-test-{}", std::process::id()));
        let path = dir.join("nested").join("mcp_config.json");
        // Missing file and folder: created, no backup (nothing to keep).
        assert!(edit_file(&path, |text| add_server(text, EXE).map(Some)).unwrap());
        assert!(!dir.join("nested").join("mcp_config.json.bak").exists());
        // Same edit again: nothing to write.
        assert!(!edit_file(&path, |text| add_server(text, EXE).map(Some)).unwrap());
        // A real change keeps the old content as .bak.
        assert!(edit_file(&path, |text| add_server(text, r"E:\other.exe").map(Some)).unwrap());
        let backup = fs::read_to_string(dir.join("nested").join("mcp_config.json.bak")).unwrap();
        assert!(backup.contains(r"C:\\Rune Studio\\rune-studio.exe"));
        // Broken JSON is refused and left alone.
        fs::write(&path, "oops").unwrap();
        assert!(edit_file(&path, |text| add_server(text, EXE).map(Some)).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "oops");
        let _ = fs::remove_dir_all(&dir);
    }
}
