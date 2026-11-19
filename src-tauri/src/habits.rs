//! Daily habits (cycle 5 of the Focus mode plan). A habit is a manual task
//! that repeats daily; nothing new is stored for it. A habit with a target
//! length is timed: focus blocks started for it add up toward the target, and
//! reaching it checks the habit in. Any other habit is a plain check-in.

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::focus::Focus;
use crate::focus_time::MINUTE;
use crate::workspace_tasks::{self, WorkspaceTask};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Habit {
    pub id: String,
    pub title: String,
    /// Minutes to put in each day. `None` is a plain check-in.
    pub target_minutes: Option<u32>,
    /// State of the current occurrence: `pending`, `in_progress`, `completed`
    /// or `skipped`.
    pub state: String,
    pub due_ms: Option<u64>,
    pub completed_ms: Option<u64>,
    /// Time put in during the current occurrence.
    pub credited_ms: u64,
}

fn view(task: &WorkspaceTask, credited_ms: u64) -> Option<Habit> {
    let current = task.occurrences.last()?;
    Some(Habit {
        id: task.id.clone(),
        title: task.title.clone(),
        target_minutes: task.estimate_minutes,
        state: current.state.clone(),
        due_ms: current.due_ms,
        completed_ms: current.completed_ms,
        credited_ms,
    })
}

/// Whether the time put in so far reaches a timed habit's target.
pub fn target_met(target_minutes: Option<u32>, credited_ms: u64) -> bool {
    target_minutes.is_some_and(|target| credited_ms >= u64::from(target) * MINUTE)
}

/// The user's habits with how today stands. The app decides which of them
/// count as today's, since "today" is the user's local day.
#[tauri::command]
pub fn habits_state(app: AppHandle, focus: State<Focus>) -> Result<Vec<Habit>, String> {
    let mut habits = Vec::new();
    for task in workspace_tasks::daily_habits(&app)? {
        let since = task.occurrences.last().and_then(|o| o.due_ms).unwrap_or(0);
        let credited = focus.credited_for(&task.id, since);
        habits.extend(view(&task, credited));
    }
    Ok(habits)
}

/// Checks a plain habit in for today.
#[tauri::command]
pub fn habit_check_in(app: AppHandle, id: String) -> Result<(), String> {
    let is_habit = workspace_tasks::daily_habits(&app)?
        .iter()
        .any(|task| task.id == id);
    if !is_habit {
        return Err("This habit is not running any more.".into());
    }
    workspace_tasks::complete_current(&app, &id)
}

/// Called when a focus block for a task ends: a timed habit whose time today
/// has reached its target is checked in.
pub fn complete_if_target_met(app: &AppHandle, focus: &Focus, task_id: &str) -> Result<(), String> {
    let Some(task) = workspace_tasks::daily_habits(app)?
        .into_iter()
        .find(|task| task.id == task_id)
    else {
        return Ok(());
    };
    let since = task.occurrences.last().and_then(|o| o.due_ms).unwrap_or(0);
    if target_met(task.estimate_minutes, focus.credited_for(task_id, since)) {
        workspace_tasks::complete_current(app, task_id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn habit(estimate: Option<u32>, state: &str) -> WorkspaceTask {
        serde_json::from_value(serde_json::json!({
            "id": "worktask-h", "revision": 1, "title": "Read", "parentGoalId": null,
            "class": "general", "createdMs": 0, "paused": false, "error": null,
            "executionMode": "manual", "estimateMinutes": estimate,
            "schedule": {"local": "2026-11-19T07:00", "zone": "UTC", "repeat": "daily",
                "nextLocal": "2026-11-20T07:00", "nextMs": 9_000_000, "anchorDay": 19},
            "occurrences": [{"id": "o1", "dueMs": 1000, "state": state, "runId": null,
                "reported": false, "completedMs": null}]
        }))
        .unwrap()
    }

    #[test]
    fn a_habit_view_carries_todays_state_and_progress() {
        let view = view(&habit(Some(30), "pending"), 10 * MINUTE).unwrap();
        assert_eq!(view.target_minutes, Some(30));
        assert_eq!(view.state, "pending");
        assert_eq!(view.credited_ms, 10 * MINUTE);
        assert_eq!(view.due_ms, Some(1000));
    }

    #[test]
    fn a_habit_with_no_occurrence_yet_has_no_view() {
        let mut task = habit(None, "pending");
        task.occurrences.clear();
        assert_eq!(view(&task, 0), None);
    }

    #[test]
    fn a_timed_habit_is_met_at_its_target_and_a_plain_one_never_is() {
        assert!(!target_met(Some(30), 29 * MINUTE));
        assert!(target_met(Some(30), 30 * MINUTE));
        assert!(target_met(Some(30), 45 * MINUTE));
        assert!(
            !target_met(None, 500 * MINUTE),
            "plain check-ins are checked by hand"
        );
    }

    #[test]
    fn only_manual_daily_tasks_are_habits() {
        assert!(workspace_tasks::is_daily_habit(&habit(None, "pending")));
        let mut weekly = habit(None, "pending");
        weekly.schedule.as_mut().unwrap().repeat = "weekly".into();
        assert!(!workspace_tasks::is_daily_habit(&weekly));
        let mut agent = habit(None, "pending");
        agent.execution_mode = "agent".into();
        assert!(!workspace_tasks::is_daily_habit(&agent));
    }
}
