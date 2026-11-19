//! "Suggest a block" (cycle 2 of the Focus mode plan): turns a rough goal into
//! one suggested focus block plus a shorter and a longer alternative, through
//! the embedded Infer. With no model the user still gets a plain block, so the
//! timer never depends on a model being up.

use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use crate::events::AppState;
use crate::focus::Focus;
use crate::focus_time::{self, MINUTE};
use crate::focus_urgency::Ranked;

const GOAL_MAX: usize = 500;
const TITLE_MAX: usize = 120;
const DONE_WHEN_MAX: usize = 200;
const MAX_MINUTES: i64 = 240;
const OPTIONS: usize = 3;
pub const DEFAULT_MINUTES: u64 = 25;
const ASK_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub title: String,
    pub done_when: String,
    pub minutes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// `model` when Infer answered, `fallback` for the plain local block.
    pub source: &'static str,
    /// Why the plain block was used, when it was.
    pub note: Option<String>,
    /// The suggested block first, then a shorter and a longer one.
    pub options: Vec<Proposal>,
}

fn schema() -> Value {
    let option = json!({"type":"object","additionalProperties":false,"properties":{
        "title":{"type":"string"},"doneWhen":{"type":"string"},
        "minutes":{"type":"integer","minimum":1,"maximum":MAX_MINUTES}},
        "required":["title","doneWhen","minutes"]});
    json!({"type":"object","additionalProperties":false,"properties":{
        "options":{"type":"array","minItems":1,"maxItems":OPTIONS,"items":option}},
        "required":["options"]})
}

fn cut(text: &str, max: usize) -> String {
    text.trim()
        .chars()
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Checks what the model sent. Entries without a title are dropped and the
/// rest are clamped to what a focus block accepts, so Accept never fails on
/// the model's numbers.
fn parse(value: &Value) -> Result<Vec<Proposal>, String> {
    let list = value
        .get("options")
        .and_then(Value::as_array)
        .ok_or("The model's suggestion had no options")?;
    let mut options: Vec<Proposal> = Vec::new();
    for item in list {
        let title = cut(
            item.get("title").and_then(Value::as_str).unwrap_or(""),
            TITLE_MAX,
        );
        if title.is_empty() {
            continue;
        }
        let minutes = item
            .get("minutes")
            .and_then(|m| m.as_i64().or_else(|| m.as_f64().map(|f| f.round() as i64)))
            .unwrap_or(DEFAULT_MINUTES as i64)
            .clamp(1, MAX_MINUTES) as u64;
        let option = Proposal {
            title,
            done_when: cut(
                item.get("doneWhen").and_then(Value::as_str).unwrap_or(""),
                DONE_WHEN_MAX,
            ),
            minutes,
        };
        if !options
            .iter()
            .any(|seen| seen.title == option.title && seen.minutes == option.minutes)
        {
            options.push(option);
        }
        if options.len() == OPTIONS {
            break;
        }
    }
    if options.is_empty() {
        return Err("The model's suggestion had no usable block".into());
    }
    Ok(options)
}

/// The waiting tasks the model may steer toward, most urgent first. Negative
/// `dueInMinutes` means overdue.
fn tasks_context(urgent: &[Ranked], now: u64) -> Value {
    urgent
        .iter()
        .map(|task| {
            json!({
                "title": cut(&task.title, TITLE_MAX),
                "dueInMinutes": ((task.due_ms as f64 - now as f64) / MINUTE as f64).round() as i64,
                "minutesLeft": task.minutes_left,
            })
        })
        .collect()
}

/// Keeps every option inside the free time and drops the ones that become
/// the same block once shortened.
fn fit(options: Vec<Proposal>, free: Option<u64>) -> Vec<Proposal> {
    let mut fitted: Vec<Proposal> = Vec::new();
    for mut option in options {
        let Some(minutes) = focus_time::fit_minutes(option.minutes, free) else {
            continue;
        };
        option.minutes = minutes;
        if !fitted
            .iter()
            .any(|seen| seen.title == option.title && seen.minutes == option.minutes)
        {
            fitted.push(option);
        }
    }
    fitted
}

fn fallback(goal: &str, why: &str, free: Option<u64>) -> Suggestion {
    let first_line = goal.lines().next().unwrap_or("");
    Suggestion {
        source: "fallback",
        note: Some(why.to_string()),
        options: vec![Proposal {
            title: cut(first_line, TITLE_MAX),
            done_when: String::new(),
            minutes: focus_time::fit_minutes(DEFAULT_MINUTES, free).unwrap_or(1),
        }],
    }
}

async fn ask(
    app: &AppHandle,
    goal: &str,
    free: Option<u64>,
    urgent: &[Ranked],
    now: u64,
) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let (settings, key) = crate::infer_catalog::agent_settings(&state).await?;
    let model = settings.model_for_role(crate::model::role::ModelRole::Agent);
    let profile =
        crate::model::protected::discover(&state.http, &settings.base_url, key.as_deref(), model)
            .await?;
    let client = crate::model::client::ChatClient {
        app: Some(app),
        output_limit: settings.output_limit(),
        http: &state.http,
        base_url: &settings.base_url,
        api_key: key.as_deref(),
        model,
        profile: Some(&profile),
        sink: None,
        no_swap: false,
        on_loading: None,
    };
    let mut user = json!({"goal": goal, "defaultMinutes": DEFAULT_MINUTES});
    if let Some(free) = free {
        user["freeMinutes"] = json!(free / MINUTE);
    }
    if !urgent.is_empty() {
        user["tasks"] = tasks_context(urgent, now);
    }
    let user = user.to_string();
    let request = crate::model::client::Request {
        system: include_str!("../prompts/focus/suggest.md"),
        user_text: &user,
        max_tokens: Some(600),
        ..Default::default()
    };
    client
        .call_json_retrying(&request, &schema())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn focus_suggest(app: AppHandle, goal: String) -> Result<Suggestion, String> {
    let goal = goal.trim().to_string();
    if goal.is_empty() {
        return Err("Type what you want to work on first.".into());
    }
    if goal.chars().count() > GOAL_MAX {
        return Err(format!("Keep it to {GOAL_MAX} characters or fewer."));
    }
    let now = crate::focus::now_ms();
    let free = app
        .try_state::<Focus>()
        .and_then(|focus| focus.free_ms(now));
    if free.is_some_and(|free| free < MINUTE) {
        return Err(
            "You have no free time before your next commitment. Remove it, or start a block yourself."
                .into(),
        );
    }
    let urgent = crate::focus::urgent_general_tasks(&app, now, 3);
    let answer = match tokio::time::timeout(ASK_TIMEOUT, ask(&app, &goal, free, &urgent, now)).await
    {
        Ok(Ok(value)) => parse(&value).map(|options| fit(options, free)),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("the model took too long".into()),
    };
    Ok(match answer {
        Ok(options) => Suggestion {
            source: "model",
            note: None,
            options,
        },
        Err(error) => {
            log::info!("focus suggestion fell back to a plain block: {error}");
            fallback(
                &goal,
                "The model could not suggest a block, so this is a plain block.",
                free,
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(title: &str, done: &str, minutes: Value) -> Value {
        json!({"title": title, "doneWhen": done, "minutes": minutes})
    }

    #[test]
    fn a_messy_goal_becomes_a_suggested_block_with_a_shorter_and_a_longer_one() {
        let value = json!({"options": [
            option("Finish the report draft", "Report draft is complete", json!(45)),
            option("Outline the report", "Every section has a heading", json!(20)),
            option("Finish and review the report", "Report is proofread", json!(75)),
        ]});
        let options = parse(&value).unwrap();
        let shape: Vec<_> = options
            .iter()
            .map(|o| (o.title.as_str(), o.minutes))
            .collect();
        assert_eq!(
            shape,
            [
                ("Finish the report draft", 45),
                ("Outline the report", 20),
                ("Finish and review the report", 75)
            ]
        );
        assert_eq!(options[0].done_when, "Report draft is complete");
    }

    #[test]
    fn the_models_numbers_are_clamped_so_accept_never_fails() {
        let value = json!({"options": [
            option("a", "", json!(0)),
            option("b", "", json!(9999)),
            option("c", "", json!(29.6)),
            option("d", "", json!("soon")),
        ]});
        let minutes: Vec<_> = parse(&value).unwrap().iter().map(|o| o.minutes).collect();
        assert_eq!(minutes, [1, 240, 30], "only three options are kept");
    }

    #[test]
    fn blank_titles_and_duplicates_are_dropped_and_long_text_is_cut() {
        let long = "x".repeat(500);
        let value = json!({"options": [
            option("   ", "no title", json!(25)),
            option(&long, &long, json!(25)),
            option(&long, "same block again", json!(25)),
        ]});
        let options = parse(&value).unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].title.chars().count(), TITLE_MAX);
        assert_eq!(options[0].done_when.chars().count(), DONE_WHEN_MAX);
    }

    #[test]
    fn a_reply_with_nothing_usable_is_an_error() {
        assert!(parse(&json!({})).is_err());
        assert!(parse(&json!({"options": []})).is_err());
        assert!(parse(&json!({"options": [option("", "x", json!(5))]})).is_err());
    }

    #[test]
    fn with_no_model_the_goal_becomes_a_plain_25_minute_block() {
        let plain = fallback(
            "  Write the quarterly update\nand send it",
            "No model.",
            None,
        );
        assert_eq!(plain.source, "fallback");
        assert_eq!(plain.note.as_deref(), Some("No model."));
        assert_eq!(
            plain.options,
            [Proposal {
                title: "Write the quarterly update".into(),
                done_when: String::new(),
                minutes: 25
            }]
        );
    }

    #[test]
    fn the_schema_allows_up_to_three_options_and_nothing_else() {
        let schema = schema();
        assert_eq!(schema["properties"]["options"]["maxItems"], 3);
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["options"]["items"]["required"],
            json!(["title", "doneWhen", "minutes"])
        );
    }

    #[test]
    fn suggestions_never_run_past_free_time() {
        let options = vec![
            Proposal {
                title: "Draft".into(),
                done_when: String::new(),
                minutes: 45,
            },
            Proposal {
                title: "Outline".into(),
                done_when: String::new(),
                minutes: 20,
            },
            Proposal {
                title: "Draft".into(),
                done_when: String::new(),
                minutes: 75,
            },
        ];
        let fitted = fit(options.clone(), Some(35 * MINUTE));
        let shape: Vec<_> = fitted
            .iter()
            .map(|o| (o.title.as_str(), o.minutes))
            .collect();
        assert_eq!(
            shape,
            [("Draft", 35), ("Outline", 20)],
            "both long drafts became one"
        );
        assert_eq!(fit(options.clone(), None).len(), 3);
        assert!(fit(options, Some(30_000)).is_empty());
    }

    #[test]
    fn the_plain_block_fits_free_time_too() {
        let plain = fallback("Write", "No model.", Some(12 * MINUTE + 30_000));
        assert_eq!(plain.options[0].minutes, 12);
    }

    #[test]
    fn urgent_tasks_are_described_to_the_model_with_how_long_until_they_are_due() {
        let ranked = |title: &str, due_min: i64, left: u64| Ranked {
            id: title.into(),
            title: title.into(),
            due_ms: (due_min * MINUTE as i64) as u64,
            minutes_left: left,
            urgency: 1.0,
            overdue: due_min < 10,
        };
        let now = 10 * MINUTE;
        let context = tasks_context(&[ranked("Invoice", 130, 45), ranked("Taxes", 4, 30)], now);
        assert_eq!(
            context,
            json!([
                {"title": "Invoice", "dueInMinutes": 120, "minutesLeft": 45},
                {"title": "Taxes", "dueInMinutes": -6, "minutesLeft": 30},
            ])
        );
    }
}
