use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pathway {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub summary: String,
    pub rationale: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub duration: String,
    pub resources: Vec<String>,
    pub feasibility: String,
    pub missing_information: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathwaySet {
    pub assessment: String,
    pub similar_outcome: String,
    pub options: Vec<Pathway>,
    #[serde(default)]
    pub goal: Value,
    #[serde(default)]
    pub selected: Option<String>,
    #[serde(default)]
    pub approved: Option<Pathway>,
}
pub fn schema() -> Value {
    let text = json!({"type":"string"});
    let list = json!({"type":"array","maxItems":6,"items":{"type":"string"}});
    json!({"type":"object","additionalProperties":false,"properties":{
        "assessment":text,"similarOutcome":text,"options":{"type":"array","maxItems":3,"items":{"type":"object","additionalProperties":false,"properties":{
        "title":text,"summary":text,"rationale":text,"pros":list,"cons":list,"duration":text,"resources":list,
        "feasibility":{"type":"string","enum":["feasible","conditional","infeasible","unknown"]},"missingInformation":list},
        "required":["title","summary","rationale","pros","cons","duration","resources","feasibility","missingInformation"]}}},"required":["assessment","similarOutcome","options"]})
}
pub fn parse(value: Value, goal: &Value) -> Result<PathwaySet, String> {
    // Model output cannot preselect a route or replace its approved goal.
    let object = value.as_object().ok_or("Invalid pathway response")?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "assessment" | "similarOutcome" | "options"))
    {
        return Err("Unexpected pathway response fields".into());
    }
    let mut set: PathwaySet = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if set.assessment.trim().is_empty() || set.options.len() > 3 {
        return Err("Invalid pathway assessment".into());
    }
    for (index, route) in set.options.iter_mut().enumerate() {
        if !route.id.is_empty()
            || [
                route.title.as_str(),
                route.summary.as_str(),
                route.rationale.as_str(),
                route.duration.as_str(),
            ]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 3000)
            || !matches!(
                route.feasibility.as_str(),
                "feasible" | "conditional" | "infeasible" | "unknown"
            )
            || [
                &route.pros,
                &route.cons,
                &route.resources,
                &route.missing_information,
            ]
            .iter()
            .any(|list| {
                list.len() > 6 || list.iter().any(|s| s.trim().is_empty() || s.len() > 2000)
            })
        {
            return Err("Invalid pathway proposal".into());
        }
        route.id = format!("route-{}", index + 1);
    }
    set.goal = goal.clone();
    set.selected = None;
    Ok(set)
}
pub fn select(set: &mut PathwaySet, approved_goal: &Value, id: &str) -> Result<(), String> {
    if &set.goal != approved_goal {
        return Err(
            "Conflict: the goal changed. Compare pathways for its latest approved definition."
                .into(),
        );
    }
    let route = set
        .options
        .iter()
        .find(|p| p.id == id)
        .ok_or("Choose an existing pathway")?;
    if !matches!(route.feasibility.as_str(), "feasible" | "conditional") {
        return Err("Clarify feasibility before choosing this pathway".into());
    }
    set.approved = Some(route.clone());
    set.selected = Some(id.into());
    Ok(())
}
pub fn preserve_preference(proposal: &mut PathwaySet, previous: Option<&PathwaySet>) {
    proposal.approved = previous
        .filter(|old| old.goal == proposal.goal)
        .and_then(|old| old.approved.clone());
}
#[cfg(test)]
mod tests {
    use super::*;
    fn response() -> Value {
        json!({"assessment":"Based on supplied constraints only","similarOutcome":"","options":[{"title":"Gradual route","summary":"A gradual approach","rationale":"Fits the supplied time","pros":["Flexible"],"cons":["Slower"],"duration":"About a month; uncertain","resources":["Time"],"feasibility":"conditional","missingInformation":["Available days"]}]})
    }
    #[test]
    fn selection_is_explicit_and_conflicts_with_changed_goals() {
        let goal = json!({"outcome":"Improve fitness"});
        let mut set = parse(response(), &goal).unwrap();
        assert!(set.selected.is_none());
        assert!(select(&mut set, &json!({"outcome":"Different"}), "route-1").is_err());
        select(&mut set, &goal, "route-1").unwrap();
        assert_eq!(set.selected.as_deref(), Some("route-1"));
        let mut replacement = parse(response(), &goal).unwrap();
        preserve_preference(&mut replacement, Some(&set));
        assert!(replacement.selected.is_none());
        assert_eq!(
            replacement.approved.as_ref().unwrap().title,
            "Gradual route"
        );
        let mut changed = parse(response(), &json!({"outcome":"Different"})).unwrap();
        preserve_preference(&mut changed, Some(&set));
        assert!(changed.approved.is_none());
        set.options[0].feasibility = "infeasible".into();
        assert!(select(&mut set, &goal, "route-1").is_err());
    }
    #[test]
    fn model_cannot_select_routes_or_fake_executable_work() {
        let mut value = response();
        value["selected"] = json!("route-1");
        assert!(parse(value, &json!({})).is_err());
        let mut value = response();
        value["options"][0]["tasks"] = json!(["Execute now"]);
        assert!(parse(value, &json!({})).is_err());
        let mut value = response();
        value["options"] = json!([]);
        assert!(parse(value, &json!({})).unwrap().options.is_empty());
    }
}
