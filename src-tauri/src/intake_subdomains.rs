//! Bounded, user-selected Manual context gathering. Models never select or skip for the user.
use crate::{
    goal_domains::{Domain, SubDomainSelection},
    life::DataClass,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_QUESTIONS: usize = 3;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Progress {
    pub selection_done: bool,
    pub current: Option<String>,
    pub completed: Vec<String>,
    pub finished: bool,
}

impl Progress {
    pub fn new(preselected: bool) -> Self {
        Self {
            selection_done: preselected,
            ..Self::default()
        }
    }

    pub fn next(&self, selections: &[SubDomainSelection]) -> Option<String> {
        selections
            .iter()
            .find(|s| {
                !s.skipped && s.answers.len() < MAX_QUESTIONS && !self.completed.contains(&s.id)
            })
            .map(|s| s.id.clone())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub explanation: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub class: DataClass,
    pub ready: bool,
    pub question: String,
    pub explanation: String,
    pub options: Vec<Choice>,
}

pub fn context(domain: &Domain) -> Value {
    json!({"id":domain.id,"name":domain.name,"zone":domain.zone,
        "subDomains":domain.sub_domains.iter().map(|s| json!({"id":s.id,"name":s.name})).collect::<Vec<_>>()})
}

pub fn schema(domain: &Domain, selection: bool) -> Value {
    let id = if selection {
        json!({"type":"string","enum":domain.sub_domains.iter().map(|s| &s.id).collect::<Vec<_>>()})
    } else {
        json!({"type":"string"})
    };
    let ready = if selection {
        json!({"type":"boolean","enum":[false]})
    } else {
        json!({"type":"boolean"})
    };
    json!({"type":"object","additionalProperties":false,"properties":{
        "class":{"type":"string","enum":["general","health","mind","finance"]},
        "ready":ready,"question":{"type":"string"},"explanation":{"type":"string"},
        "options":{"type":"array","minItems":if selection { 3 } else { 0 },"maxItems":3,"items":{
            "type":"object","additionalProperties":false,"properties":{
                "id":id,"label":{"type":"string"},"explanation":{"type":"string"}},
            "required":["id","label","explanation"]}}
    },"required":["class","ready","question","explanation","options"]})
}

pub fn parse(
    value: Value,
    domain: &Domain,
    selection: bool,
    answered: usize,
) -> Result<Response, String> {
    let error = || "Invalid sub-domain question response".to_string();
    let mut response: Response = serde_json::from_value(value).map_err(|_| error())?;
    if response.ready {
        if selection || answered == 0 {
            return Err(error());
        }
        return Ok(response);
    }
    if !valid_question(&response.question, &response.explanation, &response.options) {
        return Err(error());
    }
    for choice in &mut response.options {
        if selection {
            let sub = domain
                .sub_domains
                .iter()
                .find(|s| s.id == choice.id)
                .ok_or_else(error)?;
            choice.label = sub.name.clone();
        }
    }
    Ok(response)
}

pub fn valid_question(question: &str, explanation: &str, options: &[Choice]) -> bool {
    let mut ids = std::collections::HashSet::new();
    !question.trim().is_empty()
        && question.chars().count() <= 600
        && explanation.chars().count() <= 1200
        && options.len() == 3
        && options.iter().all(|o| {
            !o.id.is_empty()
                && o.id.chars().count() <= 80
                && !matches!(o.id.as_str(), "cancel" | "settings" | "skip" | "skip_all")
                && ids.insert(&o.id)
                && !o.label.trim().is_empty()
                && o.label.chars().count() <= 200
                && !o.explanation.trim().is_empty()
                && o.explanation.chars().count() <= 1000
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain() -> &'static Domain {
        crate::goal_domains::find("environment").unwrap()
    }
    fn selection_response() -> Value {
        json!({"class":"general","ready":false,"question":"Which parts matter most?","explanation":"Choose or skip.","options":[
            {"id":"workspace-ergonomics","label":"An outdated label","explanation":"Your stated workspace may matter."},
            {"id":"digital-hygiene","label":"Digital Hygiene","explanation":"Digital distractions may matter."},
            {"id":"geographic-fit","label":"An outdated label","explanation":"Your sensory context may matter."}]})
    }

    #[test]
    fn selection_uses_canonical_names_and_rejects_foreign_or_duplicate_ids() {
        let response = parse(selection_response(), domain(), true, 0).unwrap();
        assert_eq!(response.options[0].label, "Workspace Friction");
        assert_eq!(response.options[2].label, "Sensory Fit");
        assert!(!response.ready);
        let mut invalid = selection_response();
        invalid["options"][0]["id"] = json!("cardiorespiratory-capacity");
        assert!(parse(invalid, domain(), true, 0).is_err());
        let mut invalid = selection_response();
        invalid["options"][1]["id"] = json!("workspace-ergonomics");
        assert!(parse(invalid, domain(), true, 0).is_err());
        assert_eq!(
            schema(domain(), true)["properties"]["options"]["items"]["properties"]["id"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
    }

    #[test]
    fn early_completion_requires_an_answer_and_user_selection() {
        let ready =
            json!({"class":"mind","ready":true,"question":"","explanation":"","options":[]});
        assert!(parse(ready.clone(), domain(), true, 1).is_err());
        assert!(parse(ready.clone(), domain(), false, 0).is_err());
        assert!(parse(ready, domain(), false, 1).unwrap().ready);
    }

    #[test]
    fn questions_are_bounded_and_cannot_forge_skip_or_approval_actions() {
        for reserved in ["cancel", "settings", "skip", "skip_all"] {
            let mut invalid = selection_response();
            invalid["options"][0]["id"] = json!(reserved);
            assert!(parse(invalid, domain(), false, 0).is_err());
        }
        let mut invalid = selection_response();
        invalid["options"] = json!([]);
        assert!(parse(invalid, domain(), false, 0).is_err());
        let mut invalid = selection_response();
        invalid["question"] = json!("SENSITIVE_CANARY".repeat(100));
        assert_eq!(
            parse(invalid, domain(), false, 0).err().unwrap(),
            "Invalid sub-domain question response"
        );
        let mut invalid = selection_response();
        invalid["approved"] = json!(true);
        assert!(parse(invalid, domain(), false, 0).is_err());
    }

    #[test]
    fn progress_obeys_the_three_answer_cap_and_preserves_skipped_and_completed_parts() {
        let selections = vec![
            SubDomainSelection {
                id: "a".into(),
                answers: vec![json!("answer"); MAX_QUESTIONS],
                skipped: false,
            },
            SubDomainSelection {
                id: "b".into(),
                answers: vec![],
                skipped: true,
            },
            SubDomainSelection {
                id: "c".into(),
                answers: vec![json!("enough context")],
                skipped: false,
            },
            SubDomainSelection {
                id: "d".into(),
                answers: vec![],
                skipped: false,
            },
        ];
        let mut progress = Progress::new(true);
        progress.completed.push("c".into());
        assert_eq!(progress.next(&selections).as_deref(), Some("d"));
        progress.completed.push("d".into());
        assert!(progress.next(&selections).is_none());
        let restored: Progress =
            serde_json::from_value(serde_json::to_value(&progress).unwrap()).unwrap();
        assert_eq!(restored.completed, vec!["c", "d"]);
    }
}
