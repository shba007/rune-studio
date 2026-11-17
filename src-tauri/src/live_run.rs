use crate::{
    events::{StepFinished, StepStarted},
    sync::LockExt,
    tasks::{Phase, PlannedRequest, Task},
};
use serde::Serialize;
use std::sync::{LazyLock, Mutex};
const MAX_STEPS: usize = 512;
const MAX_RESULTS: usize = 2 * 1024 * 1024;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub start: StepStarted,
    pub finish: Option<StepFinished>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64,
    pub request: Option<PlannedRequest>,
    pub tasks: Vec<Task>,
    pub steps: Vec<Step>,
    pub dropped_results: usize,
    pub dropped_steps: usize,
    pub question: Option<crate::safety::gate::ConfirmRequest>,
    pub choices: Vec<ChoiceRecord>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceRecord {
    pub id: String,
    pub task_id: Option<u64>,
    pub step: Option<u32>,
    pub label: String,
    pub at: u64,
}
#[derive(Default)]
struct Store {
    snapshot: Snapshot,
    question_deadline: Option<u64>,
}
impl Store {
    fn sync(&mut self, board: &crate::tasks::Snapshot) {
        let request = board
            .requests
            .iter()
            .find(|r| matches!(r.phase, Phase::Running | Phase::Planning))
            .or_else(|| board.requests.iter().find(|r| r.phase == Phase::Queued))
            .or_else(|| board.requests.last())
            .cloned();
        if self.snapshot.request.as_ref().map(|r| r.id) != request.as_ref().map(|r| r.id) {
            let revision = self.snapshot.revision;
            self.snapshot = Snapshot {
                revision,
                ..Snapshot::default()
            };
        }
        self.snapshot.tasks = board
            .tasks
            .iter()
            .filter(|t| Some(t.request_id) == request.as_ref().map(|r| r.id))
            .cloned()
            .map(|mut task| {
                task.result = None;
                for step in &mut task.steps {
                    step.result = None;
                }
                task
            })
            .collect();
        self.snapshot.request = request;
        self.snapshot.revision += 1;
    }
    fn start(&mut self, event: &StepStarted, now: u64) {
        if !self
            .snapshot
            .tasks
            .iter()
            .any(|t| Some(t.id) == event.task_id)
            && !self.snapshot.request.as_ref().is_some_and(|r| {
                r.goals
                    .iter()
                    .any(|g| g.tasks.iter().any(|id| Some(*id) == event.task_id))
            })
        {
            return;
        }
        if self
            .snapshot
            .steps
            .iter()
            .any(|s| s.start.task_id == event.task_id && s.start.step == event.step)
        {
            return;
        }
        self.snapshot.steps.push(Step {
            start: event.clone(),
            finish: None,
            started_at: now,
            finished_at: None,
        });
        self.bound();
        self.snapshot.revision += 1;
    }
    fn finish(&mut self, event: &StepFinished, now: u64) {
        let Some(step) = self
            .snapshot
            .steps
            .iter_mut()
            .find(|s| s.start.task_id == event.task_id && s.start.step == event.step)
        else {
            return;
        };
        step.finish = Some(event.clone());
        step.finished_at = Some(now);
        self.bound();
        self.snapshot.revision += 1;
    }
    fn clear_task(&mut self, id: Option<u64>) {
        self.snapshot.steps.retain(|s| s.start.task_id != id);
        self.snapshot.revision += 1;
    }
    fn bound(&mut self) {
        let mut bytes: usize = self
            .snapshot
            .steps
            .iter()
            .filter_map(|s| s.finish.as_ref()?.result.as_ref())
            .map(|r| serde_json::to_vec(r).map_or(0, |v| v.len()))
            .sum();
        for step in &mut self.snapshot.steps {
            if bytes <= MAX_RESULTS {
                break;
            }
            if let Some(result) = step.finish.as_mut().and_then(|f| f.result.take()) {
                bytes = bytes.saturating_sub(serde_json::to_vec(&result).map_or(0, |v| v.len()));
                self.snapshot.dropped_results += 1;
            }
        }
        while self.snapshot.steps.len() > MAX_STEPS {
            let old = self.snapshot.steps.remove(0);
            if old.finish.and_then(|f| f.result).is_some() {
                self.snapshot.dropped_results += 1;
            }
            self.snapshot.dropped_steps += 1;
        }
    }
}
static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| Mutex::new(Store::default()));
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn sync_tasks(board: &crate::tasks::Snapshot) {
    STORE.locked().sync(board);
}
pub fn started(event: &StepStarted) {
    STORE.locked().start(event, now());
}
pub fn finished(event: &StepFinished) {
    STORE.locked().finish(event, now());
}
pub fn clear_task(id: Option<u64>) {
    STORE.locked().clear_task(id);
}
#[tauri::command]
pub fn live_run_snapshot() -> Snapshot {
    let store = STORE.locked();
    let mut snapshot = store.snapshot.clone();
    if let (Some(question), Some(deadline)) = (&mut snapshot.question, store.question_deadline) {
        question.timeout_secs = deadline.saturating_sub(now()).div_ceil(1000);
    }
    snapshot
}

pub fn context_step(task_id: Option<u64>) -> Option<u32> {
    STORE
        .locked()
        .snapshot
        .steps
        .iter()
        .rev()
        .find(|step| step.start.task_id == task_id)
        .map(|step| step.start.step)
}
pub fn question_opened(request: &crate::safety::gate::ConfirmRequest) {
    let mut store = STORE.locked();
    store.question_deadline = Some(now().saturating_add(request.timeout_secs.saturating_mul(1000)));
    store.snapshot.question = Some(request.clone());
    store.snapshot.revision += 1;
}
pub fn question_resolved(id: &str, label: Option<&str>) {
    let mut store = STORE.locked();
    if store
        .snapshot
        .question
        .as_ref()
        .is_some_and(|question| question.id == id)
    {
        let question = store.snapshot.question.take().unwrap();
        store.question_deadline = None;
        if let Some(label) = label {
            store.snapshot.choices.push(ChoiceRecord {
                id: id.into(),
                task_id: question.task_id,
                step: question.step,
                label: label.into(),
                at: now(),
            });
            if store.snapshot.choices.len() > MAX_STEPS {
                store.snapshot.choices.remove(0);
            }
        }
        store.snapshot.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn board() -> crate::tasks::Snapshot {
        crate::tasks::Snapshot {
            revision: 1,
            tasks: vec![],
            requests: vec![PlannedRequest {
                id: 1,
                history_id: "test".into(),
                text: "Get the time".into(),
                phase: Phase::Running,
                started_at: Some(10),
                finished_at: None,
                depends_on: vec![],
                error: None,
                goals: vec![crate::tasks::Goal {
                    id: "goal".into(),
                    title: "Time".into(),
                    depends_on: vec![],
                    tasks: vec![1],
                }],
            }],
        }
    }
    fn start(step: u32) -> StepStarted {
        StepStarted {
            task_id: Some(1),
            step,
            kind: "tool",
            model: Some("model".into()),
            tool: "kit_tool",
            text: "Time".into(),
            detail: "time__now".into(),
            reason: String::new(),
            point: None,
            double: false,
            button: "left",
            duration_ms: 0,
        }
    }
    fn finish(step: u32, text: String) -> StepFinished {
        StepFinished {
            task_id: Some(1),
            step,
            ok: true,
            error: None,
            result: Some(crate::kit::client::StepToolResult {
                output: crate::kit::client::ToolCallResult {
                    text,
                    structured_content: Some(json!({"count":1})),
                    structured_content_omitted: false,
                },
                output_schema: Some(json!({"type":"object"})),
            }),
        }
    }
    #[test]
    fn snapshot_keeps_events_timing_results_and_dismissal_clears_it() {
        let mut store = Store::default();
        store.sync(&board());
        store.start(&start(1), 20);
        store.finish(&finish(1, "output".into()), 30);
        let snapshot = serde_json::to_value(&store.snapshot).unwrap();
        assert_eq!(snapshot["steps"][0]["start"]["tool"], "kit_tool");
        assert_eq!(
            snapshot["steps"][0]["finish"]["result"]["structuredContent"]["count"],
            1
        );
        assert_eq!(snapshot["steps"][0]["startedAt"], 20);
        assert_eq!(snapshot["steps"][0]["finishedAt"], 30);
        let mut dismissed = board();
        dismissed.requests.clear();
        store.sync(&dismissed);
        assert!(store.snapshot.request.is_none());
        assert!(store.snapshot.steps.is_empty());
    }
    #[test]
    fn bounds_drop_oldest_results_then_steps_and_flag_both() {
        let mut store = Store::default();
        store.sync(&board());
        for step in 1..=140 {
            store.start(&start(step), u64::from(step));
            store.finish(&finish(step, "x".repeat(16000)), u64::from(step) + 1);
        }
        assert!(store.snapshot.dropped_results > 0);
        assert!(
            store.snapshot.steps[0]
                .finish
                .as_ref()
                .unwrap()
                .result
                .is_none()
        );
        assert!(
            store
                .snapshot
                .steps
                .last()
                .unwrap()
                .finish
                .as_ref()
                .unwrap()
                .result
                .is_some()
        );
        for step in 141..=514 {
            store.start(&start(step), u64::from(step));
        }
        assert_eq!(store.snapshot.steps.len(), MAX_STEPS);
        assert_eq!(store.snapshot.dropped_steps, 2);
        assert_eq!(store.snapshot.steps[0].start.step, 3);
    }
}
