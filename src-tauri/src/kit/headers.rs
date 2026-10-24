use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::collections::HashSet;
pub fn encode(value: &str) -> String {
    if value.trim() != value
        || !value.bytes().all(|b| b == b'\t' || (32..=126).contains(&b))
        || (value.starts_with("=?base64?") && value.ends_with("?="))
    {
        format!("=?base64?{}?=", STANDARD.encode(value))
    } else {
        value.into()
    }
}
pub fn extract(schema: &Value, args: &Value) -> Result<Vec<(String, String)>, String> {
    fn walk(
        schema: &Value,
        args: &Value,
        reachable: bool,
        names: &mut HashSet<String>,
        out: &mut Vec<(String, String)>,
    ) -> Result<(), String> {
        if let Some(annotation) = schema.get("x-mcp-header") {
            let name = annotation
                .as_str()
                .ok_or("MCP header annotation must be a string")?;
            if !reachable
                || name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
                || !names.insert(name.to_ascii_lowercase())
                || !matches!(
                    schema["type"].as_str(),
                    Some("string" | "boolean" | "integer")
                )
            {
                return Err("Invalid or duplicate MCP header annotation".into());
            }
            if !args.is_null() {
                if schema["type"] == "integer"
                    && !args
                        .as_i64()
                        .is_some_and(|n| n.unsigned_abs() <= 9_007_199_254_740_991)
                {
                    return Err("MCP header integer is outside the safe range".into());
                }
                let value = args
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| args.to_string());
                out.push((format!("Mcp-Param-{name}"), encode(&value)));
            }
        }
        if let Some(properties) = schema["properties"].as_object() {
            for (key, child) in properties {
                walk(child, &args[key], reachable, names, out)?;
            }
        }
        if let Some(object) = schema.as_object() {
            for (key, child) in object {
                if key != "properties" && (child.is_object() || child.is_array()) {
                    fn invalid(v: &Value) -> bool {
                        match v {
                            Value::Object(o) => {
                                o.contains_key("x-mcp-header") || o.values().any(invalid)
                            }
                            Value::Array(a) => a.iter().any(invalid),
                            _ => false,
                        }
                    }
                    if invalid(child) {
                        return Err("MCP header annotation is not statically reachable".into());
                    }
                }
            }
        }
        Ok(())
    }
    if schema.get("x-mcp-header").is_some() {
        return Err("Root header annotation is invalid".into());
    }
    let mut out = vec![];
    walk(schema, args, true, &mut HashSet::new(), &mut out)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn headers_encode_untrusted_values_and_reject_duplicates() {
        assert!(encode("a\nb").starts_with("=?base64?"));
        assert!(encode(" padded ").starts_with("=?base64?"));
        let schema =
            json!({"type":"object","properties":{"x":{"type":"string","x-mcp-header":"Target"}}});
        assert_eq!(
            extract(&schema, &json!({"x":"ok"})).unwrap(),
            vec![("Mcp-Param-Target".into(), "ok".into())]
        );
        assert!(
            extract(
                &json!({"type":"object","properties":{"x":{"type":"number","x-mcp-header":"Bad"}}}),
                &json!({})
            )
            .is_err()
        );
    }
}
