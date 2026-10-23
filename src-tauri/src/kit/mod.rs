//! Cycle 2 Kit integration boundaries. Discovery never grants execution authority.
use serde_json::Value;

/// Exact gateway identities only; names containing an approved suffix are not approved.
pub fn approved_identity(name: &str) -> bool {
    matches!(
        name,
        "time__get_current_time"
            | "rune-time__get_current_time"
            | "time__convert_time"
            | "rune-time__convert_time"
            | "git__validate_commit_message"
            | "rune-git__validate_commit_message"
    )
}

/// Unknown or contradictory metadata is unavailable in the read-only release slice.
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
    #[test]
    fn a_validator_is_not_a_commit_operation() {
        assert!(approved_identity("git__validate_commit_message"));
        assert!(!approved_identity("filesystem__write_file"));
        assert!(!approved_identity("untrusted__time__get_current_time"));
    }
}

pub mod client;
pub mod schema;
