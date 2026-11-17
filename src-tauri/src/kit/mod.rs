use serde_json::Value;

/// A tool's plugin namespace is the part before `__`. Only the built-in Rune plugins (and Kit's
/// own remote servers) may be run from Studio; anything else is listed but stays blocked.
fn namespace(name: &str) -> &str {
    name.split_once("__")
        .map_or(name, |(namespace, _)| namespace)
}

pub fn trusted_namespace(name: &str) -> bool {
    let namespace = namespace(name);
    namespace.starts_with("rune-") || matches!(namespace, "time" | "git" | "filesystem")
}

/// Why a discovered tool is listed but cannot be run from Studio yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    /// Changes files or other state. Only a confirmed Playground run may call it; the agent never can.
    Writes,
    /// Reads only, but the plugin does not yet claim verified read-only confinement.
    Unverified,
    /// Metadata is missing, malformed, unsupported or from an untrusted source.
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub reason: &'static str,
}

const fn block(kind: BlockKind, reason: &'static str) -> Option<Block> {
    Some(Block { kind, reason })
}

/// `None` means the tool may run (when selected). Every discovered tool gets an answer, so
/// the tool list can show all of them and say why each blocked one is blocked.
pub fn classify(name: &str, entry: &Value, remote: bool) -> Option<Block> {
    if !remote && !trusted_namespace(name) {
        return block(BlockKind::Unavailable, "not a built-in Rune plugin");
    }
    if headers::extract(&entry["inputSchema"], &Value::Object(Default::default())).is_err() {
        return block(BlockKind::Unavailable, "invalid MCP header annotations");
    }
    let rules = &entry["inputSchema"]["x-rune-rules"];
    if filesystem_identity(name) && rules["limits"]["root_param"] != "allowed_dir" {
        return block(
            BlockKind::Unavailable,
            "plugin does not declare the required allowed_dir boundary",
        );
    }
    if read_only_facts(entry) {
        return if schema::supported(&entry["inputSchema"]) {
            None
        } else {
            block(
                BlockKind::Unavailable,
                "plugin input schema contains unsupported constraints",
            )
        };
    }
    let Some(effects) = rules["effects"].as_array() else {
        return block(
            BlockKind::Unavailable,
            "plugin metadata is missing, conflicting or not read-only",
        );
    };
    if effects.iter().all(|effect| effect.as_str() == Some("read")) {
        block(
            BlockKind::Unverified,
            "reads files, but its read-only folder confinement is not verified yet",
        )
    } else {
        block(
            BlockKind::Writes,
            "changes files or other state; the Playground runs it only after you confirm, and the agent cannot use it",
        )
    }
}

pub fn read_only_facts(tool: &Value) -> bool {
    let annotations = &tool["annotations"];
    let rules = &tool["inputSchema"]["x-rune-rules"];
    let readonly = annotations
        .get("readOnlyHint")
        .or_else(|| annotations.get("readOnly"))
        .and_then(Value::as_bool);
    if readonly != Some(true) || rules["read_only"].as_bool() != Some(true) {
        return false;
    }
    for key in ["readOnly", "readOnlyHint"] {
        if annotations
            .get(key)
            .is_some_and(|v| v.as_bool() != Some(true))
        {
            return false;
        }
    }
    for key in [
        "destructive",
        "destructiveHint",
        "openWorld",
        "openWorldHint",
    ] {
        if annotations
            .get(key)
            .is_some_and(|v| v.as_bool() != Some(false))
        {
            return false;
        }
    }
    let Some(effects) = rules["effects"].as_array() else {
        return false;
    };
    effects.iter().all(|effect| effect.as_str() == Some("read"))
        && rules["risk"].as_str() == Some("low")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn missing_and_conflicting_facts_do_not_grant_read_only_access() {
        assert!(!read_only_facts(&json!({})));
        let mut tool = json!({"annotations":{"readOnlyHint":true,"destructiveHint":false},"inputSchema":{"x-rune-rules":{"read_only":true,"risk":"low","effects":["read"]}}});
        assert!(read_only_facts(&tool));
        tool["annotations"]["readOnly"] = json!(false);
        assert!(!read_only_facts(&tool));
        tool["annotations"]["readOnly"] = json!(true);
        tool["inputSchema"]["x-rune-rules"]["effects"] = json!(["read", "exec"]);
        assert!(!read_only_facts(&tool));
    }
    fn entry(read_only: bool, effects: serde_json::Value, risk: &str) -> Value {
        json!({"name":"x","annotations":{"readOnlyHint":read_only,"destructiveHint":!read_only},
        "inputSchema":{"type":"object","properties":{"path":{"type":"string"}},
        "x-rune-rules":{"read_only":read_only,"risk":risk,"effects":effects,
        "limits":{"root_param":"allowed_dir"}}}})
    }
    #[test]
    fn built_in_namespaces_are_trusted_and_others_are_not() {
        for name in [
            "rune-time__get_current_time",
            "time__convert_time",
            "rune-filesystem__list_directory",
            "git__validate_commit_message",
        ] {
            assert!(trusted_namespace(name), "{name}");
        }
        for name in [
            "mhb-mconnect__call",
            "untrusted__time__get_current_time",
            "bare",
        ] {
            assert!(!trusted_namespace(name), "{name}");
        }
    }
    #[test]
    fn verified_read_only_tools_may_run_and_everything_else_says_why() {
        let read = entry(true, json!(["read"]), "low");
        assert_eq!(
            classify("rune-filesystem__list_directory", &read, false),
            None
        );
        let unverified = entry(false, json!(["read"]), "low");
        assert_eq!(
            classify("rune-filesystem__directory_tree", &unverified, false)
                .unwrap()
                .kind,
            BlockKind::Unverified
        );
        for effects in [json!(["write"]), json!(["read", "write"]), json!(["exec"])] {
            let write = entry(false, effects, "medium");
            assert_eq!(
                classify("rune-filesystem__write_file", &write, false)
                    .unwrap()
                    .kind,
                BlockKind::Writes
            );
        }
    }
    #[test]
    fn missing_or_untrusted_facts_are_unavailable_not_writes() {
        let no_rules = json!({"name":"x","inputSchema":{"type":"object"}});
        assert_eq!(
            classify("rune-time__get_current_time", &no_rules, false)
                .unwrap()
                .kind,
            BlockKind::Unavailable
        );
        let read = entry(true, json!(["read"]), "low");
        assert_eq!(
            classify("mhb-mconnect__lookup", &read, false).unwrap().kind,
            BlockKind::Unavailable,
            "an untrusted plugin cannot run on its own say-so"
        );
        assert_eq!(classify("remote__lookup", &read, true), None);
        let mut no_boundary = entry(true, json!(["read"]), "low");
        no_boundary["inputSchema"]["x-rune-rules"]["limits"] = json!({});
        assert_eq!(
            classify("rune-filesystem__list_directory", &no_boundary, false)
                .unwrap()
                .kind,
            BlockKind::Unavailable
        );
    }
}

pub mod client;
pub mod schema;

pub fn filesystem_identity(name: &str) -> bool {
    name.starts_with("filesystem__") || name.starts_with("rune-filesystem__")
}

pub mod headers;
