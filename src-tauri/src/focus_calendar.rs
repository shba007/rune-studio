use std::{collections::HashSet, time::Duration};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::exec::CancelToken;
use crate::focus::{self, Focus};

pub const TOOL: &str = "get_upcoming_events";
const HOURS_AHEAD: u64 = 48;
const EVERY: Duration = Duration::from_secs(15 * 60);
const FIRST_AFTER: Duration = Duration::from_secs(20);
const MAX_EVENTS: usize = 50;
const TITLE_MAX: usize = 120;
const DAY_MS: u64 = 24 * 60 * 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub state: &'static str,
    pub events: usize,
    pub message: String,
}

pub fn parse(result: &Value, now: u64) -> Result<Vec<Event>, String> {
    let from_text: Option<Value> = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(|i| i.get("text")?.as_str()))
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|v| v.get("events").cloned());
    let list = result
        .get("structuredContent")
        .and_then(|v| v.get("events"))
        .or(from_text.as_ref())
        .ok_or("The calendar tool sent no list of events.")?
        .as_array()
        .ok_or("The calendar tool's events were not a list.")?;
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    for item in list {
        let id = item.get("id").and_then(Value::as_str).unwrap_or("").trim();
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let (Some(start), Some(end)) = (
            item.get("startMs").and_then(Value::as_u64),
            item.get("endMs").and_then(Value::as_u64),
        ) else {
            continue;
        };
        let all_day = item.get("allDay").and_then(Value::as_bool).unwrap_or(false);
        if id.is_empty()
            || title.is_empty()
            || all_day
            || end <= start
            || end <= now
            || start > now + 2 * DAY_MS
            || !seen.insert(id.to_string())
        {
            continue;
        }
        events.push(Event {
            id: id.chars().take(200).collect(),
            title: title.chars().take(TITLE_MAX).collect(),
            start_ms: start,
            end_ms: end,
        });
    }
    events.sort_by(|a, b| a.start_ms.cmp(&b.start_ms).then(a.id.cmp(&b.id)));
    events.truncate(MAX_EVENTS);
    Ok(events)
}

fn unavailable(message: impl Into<String>) -> Report {
    Report {
        state: "unavailable",
        events: 0,
        message: message.into(),
    }
}

pub async fn sync(app: &AppHandle) -> Report {
    let config = crate::kit::client::load(app);
    if !config.enabled || !config.selected.iter().any(|name| name == TOOL) {
        return unavailable("No calendar tool is selected. Add a calendar in Tools settings.");
    }
    let args = json!({ "hours": HOURS_AHEAD });
    let answer = tauri::async_runtime::spawn_blocking(move || {
        crate::kit::client::call_value(&config, TOOL, &args, &CancelToken::default())
    })
    .await;
    let now = focus::now_ms();
    let events = match answer {
        Ok(Ok(result)) => match parse(&result, now) {
            Ok(events) => events,
            Err(error) => return unavailable(error),
        },
        Ok(Err(error)) => return unavailable(format!("The calendar could not be read: {error}")),
        Err(error) => return unavailable(error.to_string()),
    };
    let Some(focus) = app.try_state::<Focus>() else {
        return unavailable("Focus is not ready yet.");
    };
    match focus.with(|store| {
        let count = store.sync_calendar(&events)?;
        Ok((count, store.snapshot()?))
    }) {
        Ok((count, snapshot)) => {
            let _ = app.emit(focus::STATE_CHANGED, &snapshot);
            Report {
                state: "synced",
                events: count,
                message: match count {
                    0 => "Calendar read: nothing in the next 48 hours.".into(),
                    1 => "Calendar read: 1 event in the next 48 hours.".into(),
                    n => format!("Calendar read: {n} events in the next 48 hours."),
                },
            }
        }
        Err(error) => unavailable(error),
    }
}

#[tauri::command]
pub async fn focus_calendar_sync(
    app: AppHandle,
    _focus: State<'_, Focus>,
) -> Result<Report, String> {
    Ok(sync(&app).await)
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_AFTER).await;
        loop {
            let report = sync(&app).await;
            log::debug!("focus calendar sync: {} ({})", report.state, report.message);
            tokio::time::sleep(EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn event(id: &str, title: &str, start_min: u64, end_min: u64) -> Value {
        json!({"id": id, "title": title, "startMs": start_min * MIN, "endMs": end_min * MIN})
    }
    fn structured(events: Vec<Value>) -> Value {
        json!({"structuredContent": {"events": events}, "content": []})
    }

    #[test]
    fn a_days_meetings_become_events_in_start_order() {
        let result = structured(vec![
            event("b", "Design review", 300, 360),
            event("a", "Standup", 60, 75),
        ]);
        let events = parse(&result, 0).unwrap();
        let shape: Vec<_> = events
            .iter()
            .map(|e| (e.id.as_str(), e.title.as_str()))
            .collect();
        assert_eq!(shape, [("a", "Standup"), ("b", "Design review")]);
        assert_eq!(events[0].start_ms, 60 * MIN);
    }

    #[test]
    fn the_same_list_in_the_text_of_a_tool_result_is_read_too() {
        let text = json!({"events": [event("a", "Standup", 60, 75)]}).to_string();
        let result = json!({"content": [{"type": "text", "text": text}]});
        assert_eq!(parse(&result, 0).unwrap().len(), 1);
    }

    #[test]
    fn unusable_entries_are_dropped_without_failing_the_rest() {
        let result = structured(vec![
            event("ok", "Keep", 60, 90),
            event("", "No id", 60, 90),
            event("t", "  ", 60, 90),
            event("rev", "Ends before it starts", 90, 60),
            event("old", "Already over", 1, 5),
            event("far", "Next week", 4000, 4060),
            event("ok", "Same id again", 70, 80),
            json!({"id": "x", "title": "No times"}),
            json!({"id": "day", "title": "Holiday", "startMs": 60 * MIN, "endMs": 1500 * MIN, "allDay": true}),
        ]);
        let events = parse(&result, 10 * MIN).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Keep");
    }

    #[test]
    fn an_answer_without_a_list_is_an_error_not_an_empty_calendar() {
        assert!(parse(&json!({"content": []}), 0).is_err());
        assert!(parse(&json!({"structuredContent": {"events": "soon"}}), 0).is_err());
        assert_eq!(parse(&structured(vec![]), 0).unwrap(), []);
    }

    #[test]
    fn long_titles_are_cut_and_the_list_is_capped() {
        let long = "x".repeat(300);
        let many: Vec<Value> = (0..80)
            .map(|i| event(&format!("e{i}"), &long, 60 + i, 61 + i))
            .collect();
        let events = parse(&structured(many), 0).unwrap();
        assert_eq!(events.len(), MAX_EVENTS);
        assert_eq!(events[0].title.chars().count(), TITLE_MAX);
    }
}
