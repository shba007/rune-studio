//! Validate domain suggestions as proposals; only a user's answer assigns a domain.
use crate::{goal_domains, life::DataClass};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Alternative {
    pub id: String,
    pub reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suggestion {
    pub class: DataClass,
    pub domain: String,
    pub confidence: f64,
    pub reason: String,
    pub alternatives: Vec<Alternative>,
}

pub fn schema() -> Value {
    let id = json!({"type":"string", "enum":goal_domains::REGISTRY.iter().map(|d| &d.id).collect::<Vec<_>>()});
    json!({"type":"object","additionalProperties":false,"properties":{
        "class":{"type":"string","enum":["general","health","mind","finance"]},
        "domain":id,"confidence":{"type":"number","minimum":0,"maximum":1},
        "reason":{"type":"string"},
        "alternatives":{"type":"array","minItems":2,"maxItems":2,"items":{
            "type":"object","additionalProperties":false,"properties":{
                "id":id,"reason":{"type":"string"}},"required":["id","reason"]}}
    },"required":["class","domain","confidence","reason","alternatives"]})
}

pub fn parse(value: Value) -> Result<Suggestion, String> {
    // Do not expose model response bodies in errors or logs.
    let suggestion: Suggestion = serde_json::from_value(value)
        .map_err(|_| "Invalid domain suggestion response".to_string())?;
    let mut ids = std::collections::HashSet::new();
    let valid = |id: &str, reason: &str| {
        goal_domains::REGISTRY.iter().any(|d| d.id == id)
            && !reason.trim().is_empty()
            && reason.chars().count() <= 1000
    };
    if !suggestion.confidence.is_finite()
        || !(0.0..=1.0).contains(&suggestion.confidence)
        || suggestion.alternatives.len() != 2
        || !valid(&suggestion.domain, &suggestion.reason)
    {
        return Err("Invalid domain suggestion response".into());
    }
    ids.insert(&suggestion.domain);
    for alternative in &suggestion.alternatives {
        if !valid(&alternative.id, &alternative.reason) || !ids.insert(&alternative.id) {
            return Err("Invalid domain suggestion response".into());
        }
    }
    Ok(suggestion)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> Value {
        json!({"class":"general","domain":"career","confidence":0.3,
            "reason":"The intended outcome may concern professional work; context is limited.",
            "alternatives":[{"id":"cognitive","reason":"It may instead concern learning."},
                {"id":"purpose","reason":"It may concern personal direction."}]})
    }

    #[test]
    fn ambiguous_matches_remain_ranked_proposals_from_the_registry() {
        let suggestion = parse(response()).unwrap();
        assert_eq!(suggestion.domain, "career");
        assert_eq!(suggestion.confidence, 0.3);
        assert_eq!(suggestion.alternatives[0].id, "cognitive");
        assert_eq!(
            schema()["properties"]["domain"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            12
        );
    }

    #[test]
    fn rejects_unknown_duplicate_unbounded_and_fabricated_suggestions() {
        let mut invalid = response();
        invalid["domain"] = json!("invented");
        assert!(parse(invalid).is_err());
        let mut invalid = response();
        invalid["alternatives"][0]["id"] = json!("career");
        assert!(parse(invalid).is_err());
        let mut invalid = response();
        invalid["alternatives"] = json!([]);
        assert!(parse(invalid).is_err());
        let mut invalid = response();
        invalid["confidence"] = json!(1.1);
        assert!(parse(invalid).is_err());
        let mut invalid = response();
        invalid["reason"] = json!(" ");
        assert!(parse(invalid).is_err());
        let mut invalid = response();
        invalid["reason"] = json!("SENSITIVE_CANARY".repeat(200));
        assert_eq!(
            parse(invalid).err().unwrap(),
            "Invalid domain suggestion response"
        );
        let mut invalid = response();
        invalid["approved"] = json!(true);
        assert!(parse(invalid).is_err());
    }
}
