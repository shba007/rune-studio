use super::Kind;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub depends_on_requests: Vec<u64>,
    pub goals: Vec<Goal>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub depends_on: Vec<String>,
    pub tasks: Vec<Task>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub kind: Kind,
    pub depends_on: Vec<String>,
    pub steps: Vec<String>,
}
impl Plan {
    pub fn parse(value: Value) -> Result<Self, String> {
        let plan: Self =
            serde_json::from_value(value).map_err(|e| format!("Invalid task plan: {e}"))?;
        plan.validate()?;
        Ok(plan)
    }
    pub fn dependencies(&self) -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        for goal in &self.goals {
            for task in &goal.tasks {
                let mut deps = task.depends_on.clone();
                for parent in &goal.depends_on {
                    if let Some(goal) = self.goals.iter().find(|g| &g.id == parent) {
                        deps.extend(goal.tasks.iter().map(|t| t.id.clone()));
                    }
                }
                deps.sort();
                deps.dedup();
                result.insert(task.id.clone(), deps);
            }
        }
        result
    }
    fn validate(&self) -> Result<(), String> {
        fn text(value: &str, max: usize) -> bool {
            !value.trim().is_empty() && value.chars().count() <= max
        }
        fn id(value: &str) -> bool {
            text(value, 40)
                && value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        }
        if self.goals.len() != 1 {
            return Err("A request must have exactly one specific goal".into());
        }
        if !self.goals[0].depends_on.is_empty()
            || self.goals[0].tasks.iter().any(|t| !t.depends_on.is_empty())
        {
            return Err("Tasks must be independent. Combine dependent work into the sequential steps of one finite task.".into());
        }
        let mut goals = HashSet::new();
        let mut tasks = HashSet::new();
        for goal in &self.goals {
            if !id(&goal.id)
                || !goals.insert(goal.id.clone())
                || !text(&goal.title, 1000)
                || goal.tasks.is_empty()
                || goal.tasks.len() > 12
            {
                return Err("Plan has invalid or duplicate goals".into());
            }
            for task in &goal.tasks {
                if !id(&task.id)
                    || !tasks.insert(task.id.clone())
                    || !text(&task.title, 1000)
                    || task.steps.is_empty()
                    || task.steps.len() > 12
                    || task.steps.iter().any(|s| !text(s, 500))
                {
                    return Err("Plan has invalid tasks or sequential steps".into());
                }
            }
        }
        if tasks.len() > 12 {
            return Err("Plan exceeds 12 tasks; split the request into smaller goals".into());
        }
        for goal in &self.goals {
            if goal
                .depends_on
                .iter()
                .any(|dep| dep == &goal.id || !goals.contains(dep))
            {
                return Err("Plan has an unknown or self-dependent goal".into());
            }
            for task in &goal.tasks {
                if task
                    .depends_on
                    .iter()
                    .any(|dep| dep == &task.id || !tasks.contains(dep))
                {
                    return Err("Plan has an unknown or self-dependent task".into());
                }
            }
        }
        let dependencies = self.dependencies();
        let mut completed = HashSet::new();
        loop {
            let previous = completed.len();
            for (task, deps) in &dependencies {
                if deps.iter().all(|d| completed.contains(d)) {
                    completed.insert(task.clone());
                }
            }
            if completed.len() == dependencies.len() {
                return Ok(());
            }
            if previous == completed.len() {
                return Err("Plan contains a dependency cycle between goals or tasks".into());
            }
        }
    }
}
pub fn schema() -> Value {
    let task = json!({"type":"object","additionalProperties":false,"required":["id","title","kind","depends_on","steps"],"properties":{
        "id":{"type":"string"},"title":{"type":"string"},"kind":{"type":"string","enum":["background","desktop"]},"depends_on":{"type":"array","maxItems":0,"items":{"type":"string"}},
        "steps":{"type":"array","minItems":1,"maxItems":12,"items":{"type":"string"}}
    }});
    let goal = json!({"type":"object","additionalProperties":false,"required":["id","title","depends_on","tasks"],"properties":{
        "id":{"type":"string"},"title":{"type":"string"},"depends_on":{"type":"array","maxItems":0,"items":{"type":"string"}},
        "tasks":{"type":"array","minItems":1,"maxItems":12,"items":task}
    }});
    json!({
        "type":"object","additionalProperties":false,"required":["goals","depends_on_requests"],"properties":{"depends_on_requests":{"type":"array","items":{"type":"integer"}},"goals":{"type":"array","minItems":1,"maxItems":1,"items":goal}}
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub fn fixture() -> Value {
        json!({"depends_on_requests":[],"goals":[{"id":"g1","title":"Summarize the supplied texts and open Notepad","depends_on":[],"tasks":[
            {"id":"a","title":"Summarize text A","kind":"background","depends_on":[],"steps":["Read text A","Summarize text A"]},
            {"id":"b","title":"Extract keywords from text B","kind":"background","depends_on":[],"steps":["Extract keywords"]},
            {"id":"c","title":"Open Notepad","kind":"desktop","depends_on":[],"steps":["Open Notepad"]}]}]})
    }
    #[test]
    fn one_specific_goal_has_independent_tasks_with_atomic_sequential_steps() {
        let plan = Plan::parse(fixture()).unwrap();
        let deps = plan.dependencies();
        assert!(deps["a"].is_empty());
        assert!(deps["b"].is_empty());
        assert!(deps["c"].is_empty());
        assert_eq!(
            plan.goals[0].tasks[0].steps,
            ["Read text A", "Summarize text A"]
        );
    }
    #[test]
    fn multiple_goals_or_dependent_tasks_are_rejected() {
        let mut value = fixture();
        let second = value["goals"][0].clone();
        value["goals"].as_array_mut().unwrap().push(second);
        assert!(Plan::parse(value).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][1]["depends_on"] = json!(["a"]);
        assert!(Plan::parse(value).is_err());
    }
    #[test]
    fn rejects_cycles_unknown_dependencies_and_duplicate_ids() {
        let mut value = fixture();
        value["goals"][0]["depends_on"] = json!(["g3"]);
        assert!(Plan::parse(value).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][0]["depends_on"] = json!(["d"]);
        assert!(Plan::parse(value).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][0]["depends_on"] = json!(["missing"]);
        assert!(Plan::parse(value).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][1]["id"] = json!("a");
        assert!(Plan::parse(value).is_err());
    }
    #[test]
    fn absent_dependency_metadata_is_rejected_instead_of_assuming_independence() {
        let mut value = fixture();
        value.as_object_mut().unwrap().remove("depends_on_requests");
        assert!(Plan::parse(value).is_err());
    }
    #[test]
    fn malformed_or_unbounded_plans_never_become_executable_tasks() {
        assert!(Plan::parse(json!({"depends_on_requests":[],"goals":[]})).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][0]["kind"] = json!("parallel");
        assert!(Plan::parse(value).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][0]["steps"] = json!([]);
        assert!(Plan::parse(value).is_err());
        let mut value = fixture();
        value["goals"][0]["tasks"][0]["unknown"] = json!(true);
        assert!(Plan::parse(value).is_err());
    }
}
