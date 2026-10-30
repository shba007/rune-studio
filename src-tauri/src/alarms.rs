//! Alarm delivery for scheduled reminders: Windows toast while the orb is hidden and a tray summary.
use crate::sync::LockExt;
use crate::workspace_tasks::WorkspaceTask;
use std::{collections::HashSet, sync::Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_notification::NotificationExt;

const TRAY_ID: &str = "main";
const BASE_TOOLTIP: &str = "Rune Studio";

static NOTIFIED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
static TOOLTIP: Mutex<String> = Mutex::new(String::new());

#[derive(Debug, PartialEq, Eq)]
pub struct Due {
    pub key: String,
    pub title: String,
    pub at_ms: u64,
}

pub fn due_now(tasks: &[WorkspaceTask], now: u64) -> Vec<Due> {
    let mut due: Vec<Due> = tasks
        .iter()
        .filter(|task| !task.paused)
        .filter_map(|task| {
            let current = task.occurrences.last()?;
            let at_ms = current.effective_due_ms()?;
            (current.state == "pending" && at_ms <= now).then(|| Due {
                key: format!("{}:{}", current.id, at_ms),
                title: task.title.clone(),
                at_ms,
            })
        })
        .collect();
    due.sort_by_key(|d| d.at_ms);
    due
}

pub fn next_upcoming(tasks: &[WorkspaceTask], now: u64) -> Option<(String, u64)> {
    tasks
        .iter()
        .filter(|task| !task.paused)
        .filter_map(|task| {
            let current = task.occurrences.last()?;
            let at_ms = current.effective_due_ms()?;
            (current.state == "pending" && at_ms > now).then(|| (task.title.clone(), at_ms))
        })
        .min_by_key(|(_, at)| *at)
}

pub fn tooltip(due: usize, next: Option<&str>) -> String {
    match (due, next) {
        (0, None) => BASE_TOOLTIP.into(),
        (0, Some(next)) => format!("{BASE_TOOLTIP} · Next: {next}"),
        (count, _) => format!("{BASE_TOOLTIP} · {count} due"),
    }
}

fn fresh(due: &[Due], seen: &mut HashSet<String>) -> Vec<String> {
    seen.retain(|key| due.iter().any(|d| &d.key == key));
    due.iter()
        .filter(|d| seen.insert(d.key.clone()))
        .map(|d| d.title.clone())
        .collect()
}

pub fn refresh(app: &AppHandle, tasks: &[WorkspaceTask], now: u64) {
    let due = due_now(tasks, now);
    let next = next_upcoming(tasks, now);
    let hidden = app
        .get_webview_window("main")
        .is_none_or(|window| !window.is_visible().unwrap_or(true));
    let newly = {
        let mut guard = NOTIFIED.locked();
        fresh(&due, guard.get_or_insert_with(HashSet::new))
    };
    if hidden {
        for title in newly {
            let _ = app
                .notification()
                .builder()
                .title("Rune reminder")
                .body(title)
                .show();
        }
    }
    let text = tooltip(due.len(), next.as_ref().map(|(title, _)| title.as_str()));
    let mut last = TOOLTIP.locked();
    if *last != text {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(text.as_str()));
        }
        *last = text;
    }
}

#[tauri::command]
pub fn alarms_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn alarms_set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    manager.is_enabled().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(
        title: &str,
        state: &str,
        due: Option<u64>,
        snoozed: Option<u64>,
        paused: bool,
    ) -> WorkspaceTask {
        serde_json::from_value(serde_json::json!({
            "id":"worktask-t","revision":1,"title":title,"parentGoalId":null,"class":"general",
            "createdMs":0,"paused":paused,"error":null,"schedule":null,
            "occurrences":[{"id":format!("o-{title}"),"dueMs":due,"state":state,"runId":null,
                "reported":false,"completedMs":null,"snoozedUntilMs":snoozed}]
        }))
        .unwrap()
    }

    #[test]
    fn only_pending_unpaused_reminders_that_have_reached_their_effective_time_are_due() {
        let tasks = [
            task("a", "pending", Some(100), None, false),
            task("b", "pending", Some(100), Some(500), false),
            task("c", "pending", Some(100), None, true),
            task("d", "completed", Some(100), None, false),
            task("e", "pending", None, None, false),
        ];
        let due = due_now(&tasks, 200);
        assert_eq!(
            due.iter().map(|d| d.title.as_str()).collect::<Vec<_>>(),
            ["a"]
        );
        let later = due_now(&tasks, 600);
        assert_eq!(later.len(), 2);
        assert_eq!(later[0].title, "a");
        assert_eq!(later[1].key, "o-b:500");
        assert_eq!(next_upcoming(&tasks, 200), Some(("b".into(), 500)));
    }

    #[test]
    fn a_reminder_is_announced_once_and_again_after_a_snooze() {
        let mut seen = HashSet::new();
        let first = due_now(&[task("a", "pending", Some(100), None, false)], 200);
        assert_eq!(fresh(&first, &mut seen), ["a"]);
        assert!(fresh(&first, &mut seen).is_empty());
        assert!(fresh(&[], &mut seen).is_empty());
        assert_eq!(fresh(&first, &mut seen), ["a"]);
        let snoozed = due_now(&[task("a", "pending", Some(100), Some(300), false)], 400);
        assert_eq!(fresh(&snoozed, &mut seen), ["a"]);
    }

    #[test]
    fn the_tray_summary_prefers_due_count_then_the_next_reminder() {
        assert_eq!(tooltip(0, None), "Rune Studio");
        assert_eq!(tooltip(0, Some("Stretch")), "Rune Studio · Next: Stretch");
        assert_eq!(tooltip(2, Some("Stretch")), "Rune Studio · 2 due");
    }
}
