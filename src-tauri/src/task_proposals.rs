//! First-week work proposals are protected drafts, never executable model output.
use crate::{intake_pathways::Pathway, task_calendar, workspace_tasks::ScheduleInput};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedTask {
    pub id: String,
    pub title: String,
    pub instructions: String,
    pub rationale: String,
    pub execution_mode: String,
    pub schedule: Option<ScheduleInput>,
    pub created_task_id: Option<String>,
    pub dismissed: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: String,
    pub goal: Value,
    pub pathway: Pathway,
    pub start_local: String,
    pub end_local: String,
    pub zone: String,
    pub assessment: String,
    pub tasks: Vec<ProposedTask>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub title: String,
    pub instructions: String,
    pub execution_mode: String,
    pub schedule: Option<ScheduleInput>,
}
impl Approval {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty()
            || self.title.chars().count() > 200
            || self.instructions.trim().is_empty()
            || self.instructions.chars().count() > 4000
            || !matches!(
                self.execution_mode.as_str(),
                "manual" | "agent" | "analysis"
            )
        {
            return Err("Provide a title (up to 200 characters), instructions (up to 4,000) and a supported task mode".into());
        }
        Ok(())
    }
}
impl Draft {
    pub fn task_id(&self, proposal_id: &str) -> Result<String, String> {
        if !self.tasks.iter().any(|task| task.id == proposal_id) {
            return Err("Choose a task in this draft".into());
        }
        Ok(format!("worktask-{}-{}", self.id, proposal_id))
    }
    pub fn check_context(&self, goal: &Value, pathway: &Pathway) -> Result<(), String> {
        if self.goal != *goal
            || serde_json::to_value(&self.pathway).ok() != serde_json::to_value(pathway).ok()
        {
            return Err("Conflict: the approved goal or preferred pathway changed. Generate a new draft before creating work.".into());
        }
        Ok(())
    }
    pub fn check_schedule(&self, schedule: Option<&ScheduleInput>) -> Result<(), String> {
        if let Some(s) = schedule {
            task_calendar::parse(&s.local)?;
            if s.zone != self.zone
                || s.local[..10] < self.start_local[..10]
                || s.local[..10] >= self.end_local[..10]
            {
                return Err("Choose the first occurrence within this draft's first week and recorded timezone, or leave it unscheduled".into());
            }
        }
        Ok(())
    }
}
pub fn schema() -> Value {
    let text = json!({"type":"string"});
    json!({"type":"object","additionalProperties":false,"properties":{
        "assessment":text,"tasks":{"type":"array","maxItems":7,"items":{"type":"object","additionalProperties":false,
        "properties":{"title":text,"instructions":text,"rationale":text,
        "executionMode":{"type":"string","enum":["manual","agent","analysis"]},
        "schedule":{"anyOf":[{"type":"null"},{"type":"object","additionalProperties":false,
        "properties":{"day":{"type":"integer","minimum":0,"maximum":6},"time":text,
        "repeat":{"type":"string","enum":["once","daily","weekly","monthly"]}},"required":["day","time","repeat"]}]}},
        "required":["title","instructions","rationale","executionMode","schedule"]}}},"required":["assessment","tasks"]})
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawTask {
    title: String,
    instructions: String,
    rationale: String,
    execution_mode: String,
    schedule: Option<RawSchedule>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSchedule {
    day: u32,
    time: String,
    repeat: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    assessment: String,
    tasks: Vec<RawTask>,
}
pub fn parse(
    value: Value,
    id: String,
    goal: Value,
    pathway: Pathway,
    start_local: String,
    zone: String,
) -> Result<Draft, String> {
    let response: Response = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if response.assessment.trim().is_empty()
        || response.assessment.len() > 4000
        || response.tasks.len() > 7
    {
        return Err("Invalid first-week task proposal".into());
    }
    let start = task_calendar::parse(&start_local)?;
    let end_local = task_calendar::text(task_calendar::advance(start, "weekly", start.d)?);
    let mut tasks = vec![];
    for (index, raw) in response.tasks.into_iter().enumerate() {
        let schedule = raw
            .schedule
            .map(|s| {
                if s.day > 6
                    || !matches!(s.repeat.as_str(), "once" | "daily" | "weekly" | "monthly")
                {
                    return Err("Invalid proposed recurrence".to_string());
                }
                let mut day = start;
                for _ in 0..s.day {
                    day = task_calendar::advance(day, "daily", day.d)?;
                }
                let local = format!("{}T{}", &task_calendar::text(day)[..10], s.time);
                let parsed = task_calendar::parse(&local)?;
                Ok(ScheduleInput {
                    local,
                    zone: zone.clone(),
                    repeat: s.repeat,
                    anchor_day: Some(parsed.d),
                })
            })
            .transpose()?;
        let approval = Approval {
            title: raw.title,
            instructions: raw.instructions,
            execution_mode: raw.execution_mode,
            schedule,
        };
        approval.validate()?;
        if raw.rationale.trim().is_empty() || raw.rationale.len() > 2000 {
            return Err("Explain each proposed task".into());
        }
        tasks.push(ProposedTask {
            id: format!("task-{}", index + 1),
            title: approval.title,
            instructions: approval.instructions,
            execution_mode: approval.execution_mode,
            schedule: approval.schedule,
            rationale: raw.rationale,
            created_task_id: None,
            dismissed: false,
        });
    }
    Ok(Draft {
        id,
        goal,
        pathway,
        start_local,
        end_local,
        zone,
        assessment: response.assessment,
        tasks,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn sample() -> Draft {
        let pathway: Pathway = serde_json::from_value(json!({"id":"route-1","title":"Gradual route","summary":"Use available time","rationale":"Fits constraints","pros":[],"cons":[],"duration":"One month","resources":[],"feasibility":"conditional","missingInformation":[]})).unwrap();
        parse(
            response(),
            "draft-test".into(),
            json!({"outcome":"PROPOSAL_CANARY"}),
            pathway,
            "2028-02-27T12:00".into(),
            "UTC".into(),
        )
        .unwrap()
    }
    fn response() -> Value {
        json!({"assessment":"PROPOSAL_ASSESSMENT_CANARY","tasks":[{"title":"Short walk","instructions":"Walk for the agreed duration","rationale":"Uses stated availability","executionMode":"manual","schedule":{"day":2,"time":"07:00","repeat":"weekly"}}]})
    }
    fn parse_like(value: Value) -> Result<Draft, String> {
        let d = sample();
        parse(value, d.id, d.goal, d.pathway, d.start_local, d.zone)
    }
    #[test]
    fn drafts_restore_calendar_suggestions_without_approving_or_running_work() {
        let d = sample();
        assert_eq!(
            d.tasks[0].schedule.as_ref().unwrap().local,
            "2028-02-29T07:00"
        );
        assert_eq!(d.end_local, "2028-03-05T12:00");
        assert!(d.tasks[0].created_task_id.is_none());
        assert!(!d.tasks[0].dismissed);
        let restored: Draft = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(
            d.task_id("task-1").unwrap(),
            restored.task_id("task-1").unwrap()
        );
        assert!(d.task_id("forged-id").is_err());
        assert!(
            d.check_context(&json!({"outcome":"changed"}), &d.pathway)
                .is_err()
        );
        let mut changed_route = d.pathway.clone();
        changed_route.summary = "new route".into();
        assert!(d.check_context(&d.goal, &changed_route).is_err());
        assert!(d.check_context(&d.goal, &d.pathway).is_ok());
    }
    #[test]
    fn model_cannot_forge_approval_ids_tools_or_out_of_week_schedules() {
        for field in ["id", "createdTaskId", "dismissed", "tools"] {
            let mut value = response();
            value["tasks"][0][field] = json!("forged");
            assert!(parse_like(value).is_err(), "{field}");
        }
        let mut value = response();
        value["tasks"][0]["schedule"]["day"] = json!(7);
        assert!(parse_like(value).is_err());
        let mut value = response();
        value["tasks"][0]["schedule"]["time"] = json!("25:00");
        assert!(parse_like(value).is_err());
        let mut value = response();
        value["tasks"][0]["executionMode"] = json!("auto");
        assert!(parse_like(value).is_err());
        let mut value = response();
        value["tasks"] = json!([]);
        assert!(parse_like(value).unwrap().tasks.is_empty());
    }
    #[test]
    fn edited_schedules_must_keep_the_draft_week_and_timezone() {
        let d = sample();
        let mut s = d.tasks[0].schedule.clone().unwrap();
        assert!(d.check_schedule(Some(&s)).is_ok());
        s.local = "2028-03-05T07:00".into();
        assert!(d.check_schedule(Some(&s)).is_err());
        s.local = "2028-02-26T07:00".into();
        assert!(d.check_schedule(Some(&s)).is_err());
        s.local = "2028-02-29T07:00".into();
        s.zone = "other".into();
        assert!(d.check_schedule(Some(&s)).is_err());
        assert!(d.check_schedule(None).is_ok());
        let approval = Approval {
            title: "".into(),
            instructions: "work".into(),
            execution_mode: "manual".into(),
            schedule: None,
        };
        assert!(approval.validate().is_err());
    }
}
