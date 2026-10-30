//! Text-free Auto discovery: models propose questions and drafts, never approvals.
use crate::{
    goal_domains::SubDomainSelection,
    intake_subdomains::{Choice, MAX_QUESTIONS, valid_question},
    life::DataClass,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Questioning,
    Drafting,
    Reviewing,
    Clarifying,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Progress {
    pub phase: Phase,
    pub current: Option<String>,
    pub require_question: bool,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            phase: Phase::Questioning,
            current: None,
            require_question: true,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub outcome: String,
    pub current: String,
    pub success: String,
    pub horizon: String,
    pub resources: String,
    pub constraints: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Response {
    pub class: DataClass,
    pub ready: bool,
    pub sub_domain: Option<String>,
    pub question: String,
    pub explanation: String,
    pub options: Vec<Choice>,
    pub goal: Option<Goal>,
}

pub fn available(selections: &[SubDomainSelection]) -> Vec<String> {
    selections
        .iter()
        .filter(|s| !s.skipped && s.answers.len() < MAX_QUESTIONS)
        .map(|s| s.id.clone())
        .collect()
}

pub fn answered(selections: &[SubDomainSelection], history: &[Value]) -> usize {
    let is_answer = |a: &&Value| {
        a["kind"] == "discovery"
            && a["source"] == "user"
            && a["answer"].as_str().is_some_and(|s| !s.trim().is_empty())
    };
    selections
        .iter()
        .flat_map(|s| &s.answers)
        .filter(is_answer)
        .count()
        + history
            .iter()
            .filter(|a| a["subDomain"].is_null())
            .filter(is_answer)
            .count()
}

pub fn schema(available: &[String], require_question: bool, draft_only: bool) -> Value {
    let text = json!({"type":"string"});
    let topic = if available.is_empty() {
        json!({"type":"null"})
    } else {
        json!({"anyOf":[{"type":"null"},{"type":"string","enum":available}]})
    };
    let ready = if draft_only {
        json!({"type":"boolean","enum":[true]})
    } else if require_question {
        json!({"type":"boolean","enum":[false]})
    } else {
        json!({"type":"boolean"})
    };
    json!({"type":"object","additionalProperties":false,"properties":{
        "class":{"type":"string","enum":["general","health","mind","finance"]},
        "ready":ready,"subDomain":topic,"question":text,"explanation":text,
        "options":{"type":"array","maxItems":3,"items":{"type":"object","additionalProperties":false,
            "properties":{"id":text,"label":text,"explanation":text},"required":["id","label","explanation"]}},
        "goal":{"anyOf":[{"type":"null"},{"type":"object","additionalProperties":false,
            "properties":{"outcome":text,"current":text,"success":text,"horizon":text,"resources":text,"constraints":text},
            "required":["outcome","current","success","horizon","resources","constraints"]}]}
    },"required":["class","ready","subDomain","question","explanation","options","goal"]})
}

pub fn parse(
    value: Value,
    available: &[String],
    answered: usize,
    require_question: bool,
    draft_only: bool,
) -> Result<Response, String> {
    let error = || "Invalid Auto discovery response".to_string();
    let response: Response = serde_json::from_value(value).map_err(|_| error())?;
    if response.ready {
        if answered == 0
            || require_question
            || response.sub_domain.is_some()
            || !response.question.is_empty()
            || !response.options.is_empty()
            || response.explanation.chars().count() > 1200
        {
            return Err(error());
        }
        let goal = response.goal.as_ref().ok_or_else(error)?;
        if [
            &goal.outcome,
            &goal.current,
            &goal.success,
            &goal.horizon,
            &goal.resources,
            &goal.constraints,
        ]
        .iter()
        .any(|s| s.trim().is_empty() || s.chars().count() > 4000)
        {
            return Err(error());
        }
    } else if draft_only
        || response.goal.is_some()
        || !response
            .sub_domain
            .as_ref()
            .is_some_and(|id| available.contains(id))
        || !valid_question(&response.question, &response.explanation, &response.options)
    {
        return Err(error());
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question() -> Value {
        json!({"class":"health","ready":false,"subDomain":"sleep-and-circadian-architecture",
            "question":"How rested do you usually feel after waking?","explanation":"Start with current recovery context.",
            "options":[{"id":"rested","label":"Usually rested","explanation":"This reflects a steady baseline."},
                {"id":"mixed","label":"It varies","explanation":"The pattern may need more context."},
                {"id":"unknown","label":"I am not sure","explanation":"No baseline is assumed."}],"goal":null})
    }

    fn draft() -> Value {
        json!({"class":"health","ready":true,"subDomain":null,"question":"","explanation":"A provisional draft from the reported inconsistency.","options":[],
            "goal":{"outcome":"Explore a more consistent wind-down routine","current":"Sleep feels inconsistent",
                "success":"Unknown","horizon":"Unknown","resources":"Unknown","constraints":"Unknown"}})
    }

    #[test]
    fn first_turn_requires_a_question_and_never_accepts_a_goal_without_an_answer() {
        let topics = vec!["sleep-and-circadian-architecture".into()];
        let first = parse(question(), &topics, 0, true, false).unwrap();
        assert!(!first.ready);
        assert!(first.goal.is_none());
        assert!(parse(draft(), &topics, 0, false, true).is_err());
        assert!(parse(draft(), &topics, 1, true, false).is_err());
        assert_eq!(
            schema(&topics, true, false)["properties"]["ready"]["enum"],
            json!([false])
        );
        let proposed = parse(draft(), &topics, 1, false, true).unwrap();
        assert_eq!(proposed.goal.unwrap().resources, "Unknown");
    }

    #[test]
    fn adaptive_topics_cannot_escape_the_available_parts_or_replace_draft_mode_with_questions() {
        let topics = vec!["sleep-and-circadian-architecture".into()];
        let mut invalid = question();
        invalid["subDomain"] = json!("digital-hygiene");
        assert!(parse(invalid, &topics, 1, false, false).is_err());
        assert!(parse(question(), &[], 1, false, false).is_err());
        assert!(parse(question(), &topics, 1, false, true).is_err());
        let mut invalid = question();
        invalid["goal"] = draft()["goal"].clone();
        assert!(parse(invalid, &topics, 1, false, false).is_err());
    }

    #[test]
    fn response_validation_rejects_forged_actions_unbounded_drafts_and_private_error_bodies() {
        let topics = vec!["sleep-and-circadian-architecture".into()];
        let mut invalid = question();
        invalid["options"][0]["id"] = json!("skip_all");
        assert!(parse(invalid, &topics, 0, true, false).is_err());
        let mut invalid = question();
        invalid["options"][1]["id"] = json!("rested");
        assert!(parse(invalid, &topics, 0, true, false).is_err());
        let mut invalid = draft();
        invalid["approvedGoal"] = json!({"outcome":"Execute immediately"});
        assert!(parse(invalid, &topics, 1, false, true).is_err());
        let mut invalid = draft();
        invalid["domain"] = json!("financial");
        assert!(parse(invalid, &topics, 1, false, true).is_err());
        let mut invalid = draft();
        invalid["goal"]["outcome"] = json!("PRIVATE_CANARY".repeat(400));
        assert_eq!(
            parse(invalid, &topics, 1, false, true).err().unwrap(),
            "Invalid Auto discovery response"
        );
        let mut invalid = draft();
        invalid["goal"]["current"] = json!("");
        assert!(parse(invalid, &topics, 1, false, true).is_err());
    }

    #[test]
    fn counts_only_real_user_answers_and_bounds_each_subdomain() {
        let selections = vec![
            SubDomainSelection {
                id: "a".into(),
                skipped: true,
                answers: vec![
                    json!({"kind":"discovery","source":"user","answer":"A real answer"}),
                    json!({"kind":"discovery","source":"model","answer":"An inferred fact"}),
                    json!({"kind":"discovery","source":"user","answer":" "}),
                ],
            },
            SubDomainSelection {
                id: "b".into(),
                skipped: false,
                answers: vec![],
            },
        ];
        let history = vec![
            json!({"kind":"discovery_skip","source":"user"}),
            json!({"kind":"discovery","subDomain":"a","source":"user"}),
            json!({"kind":"discovery","subDomain":null,"source":"user","answer":"Domain-level context"}),
        ];
        assert_eq!(answered(&selections, &history), 2);
        assert_eq!(available(&selections), vec!["b"]);
        let mut bounded = selections;
        bounded[1].answers =
            vec![json!({"kind":"discovery","source":"user","answer":"Context"}); MAX_QUESTIONS];
        assert!(available(&bounded).is_empty());
    }
}
