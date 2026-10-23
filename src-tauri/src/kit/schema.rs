use serde_json::Value;
// Reject constructs outside the explicitly checked subset.
pub fn supported(schema: &Value) -> bool {
    let Some(obj) = schema.as_object() else {
        return false;
    };
    obj.keys().all(|k| {
        matches!(
            k.as_str(),
            "$schema"
                | "type"
                | "properties"
                | "required"
                | "additionalProperties"
                | "enum"
                | "description"
                | "title"
                | "default"
                | "minimum"
                | "maximum"
                | "minLength"
                | "maxLength"
                | "x-rune-rules"
                | "x-rune-read-limits"
        )
    }) && schema["type"]
        .as_str()
        .is_some_and(|t| matches!(t, "object" | "string" | "number" | "integer" | "boolean"))
        && schema
            .get("enum")
            .is_none_or(|v| v.as_array().is_some_and(|v| !v.is_empty()))
        && ["minimum", "maximum"]
            .iter()
            .all(|k| schema.get(*k).is_none_or(Value::is_number))
        && ["minLength", "maxLength"]
            .iter()
            .all(|k| schema.get(*k).is_none_or(|v| v.as_u64().is_some()))
        && schema
            .get("additionalProperties")
            .is_none_or(Value::is_boolean)
        && schema
            .get("properties")
            .is_none_or(|p| p.as_object().is_some_and(|p| p.values().all(supported)))
        && schema
            .get("required")
            .is_none_or(|r| r.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
}
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    if !supported(schema) {
        return Err("Unsupported argument schema".into());
    }
    let valid = match schema["type"].as_str().unwrap_or("") {
        "object" => value.is_object(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.is_i64() || value.is_u64(),
        _ => false,
    };
    if !valid {
        return Err("Argument type does not match schema".into());
    }
    if let Some(values) = schema["enum"].as_array()
        && !values.contains(value)
    {
        return Err("Value outside enum".into());
    }
    if let Some(obj) = value.as_object() {
        if let Some(required) = schema["required"].as_array() {
            for name in required {
                if !obj.contains_key(name.as_str().ok_or("Invalid required property")?) {
                    return Err(format!("Missing required property {name}"));
                }
            }
        }
        for (name, val) in obj {
            if let Some(sub) = schema["properties"].get(name) {
                validate(sub, val)?
            } else if schema["additionalProperties"] == false {
                return Err(format!("Unexpected property {name}"));
            }
        }
    }
    if let Some(number) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|n| number < n)
            || schema["maximum"].as_f64().is_some_and(|n| number > n))
    {
        return Err("Number outside bounds".into());
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|n| length < n)
            || schema["maxLength"].as_u64().is_some_and(|n| length > n)
        {
            return Err("String outside length bounds".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn required_types_bounds_and_extra_properties_are_enforced() {
        let schema = json!({"type":"object","properties":{"message":{"type":"string","minLength":3,"maxLength":8},"count":{"type":"integer","minimum":1,"maximum":4}},"required":["message"],"additionalProperties":false});
        for value in [
            json!({}),
            json!({"message":3}),
            json!({"message":"a"}),
            json!({"message":"okay","extra":true}),
            json!({"message":"okay","count":5}),
        ] {
            assert!(validate(&schema, &value).is_err());
        }
        assert!(validate(&schema, &json!({"message":"okay","count":2})).is_ok());
    }
    #[test]
    fn unsupported_or_malformed_constraints_are_unavailable() {
        for schema in [
            json!({"type":"string","pattern":".*"}),
            json!({"type":"string","enum":false}),
            json!({"type":"number","maximum":"many"}),
            json!({"type":"array"}),
        ] {
            assert!(!supported(&schema));
        }
    }
}
