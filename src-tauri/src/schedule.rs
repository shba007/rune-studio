use crate::AppState;
use crate::task_calendar::{Civil, parse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

const MAX_DELAY_SECONDS: u64 = 60 * 24 * 3600;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interpretation {
    pub kind: String,
    pub title: String,
    pub delay_seconds: u64,
    pub local: String,
}

fn weekday(c: Civil) -> &'static str {
    const OFFSET: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if c.m < 3 { c.y - 1 } else { c.y };
    let index = (y + y / 4 - y / 100 + y / 400 + OFFSET[c.m as usize - 1] + c.d as i32) % 7;
    [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ][index as usize]
}

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["kind","title","delaySeconds","local"],"properties":{
        "kind":{"type":"string","enum":["after","at","unclear"]},
        "title":{"type":"string"},
        "delaySeconds":{"type":"integer","minimum":0},
        "local":{"type":"string"}
    }})
}

fn clean_title(title: &str) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err("Say what to remind you about in up to 200 characters.".into());
    }
    Ok(title.to_string())
}

fn validate(value: Value, now_local: &str) -> Result<Interpretation, String> {
    let parsed: Interpretation = serde_json::from_value(value)
        .map_err(|e| format!("Rune could not read the reminder: {e}"))?;
    match parsed.kind.as_str() {
        "unclear" => Ok(Interpretation {
            kind: parsed.kind,
            title: String::new(),
            delay_seconds: 0,
            local: String::new(),
        }),
        "after" => {
            let title = clean_title(&parsed.title)?;
            if parsed.delay_seconds == 0 || parsed.delay_seconds > MAX_DELAY_SECONDS {
                return Err("Choose a reminder time within 60 days.".into());
            }
            Ok(Interpretation {
                kind: parsed.kind,
                title,
                delay_seconds: parsed.delay_seconds,
                local: String::new(),
            })
        }
        "at" => {
            let title = clean_title(&parsed.title)?;
            parse(&parsed.local)
                .map_err(|_| "Rune could not read that date and time.".to_string())?;
            if parsed.local.as_str() <= now_local {
                return Err("That time has already passed.".into());
            }
            Ok(Interpretation {
                kind: parsed.kind,
                title,
                delay_seconds: 0,
                local: parsed.local,
            })
        }
        _ => Err("Rune could not read the reminder.".into()),
    }
}

#[tauri::command]
pub async fn schedule_interpret(app: AppHandle, text: String) -> Result<Interpretation, String> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 500 {
        return Err("Enter a reminder of 1 to 500 characters.".into());
    }
    let zone = crate::task_calendar::current_zone()?;
    let now_local = crate::task_calendar::local_now(&zone)?;
    let day = weekday(parse(&now_local)?);
    let state = app.state::<AppState>();
    let (settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
    let model = settings.model_for_role(crate::model::role::ModelRole::Agent);
    let profile =
        crate::model::protected::discover(&state.http, &settings.base_url, key.as_deref(), model)
            .await?;
    let client = crate::model::client::ChatClient {
        app: Some(&app),
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
    let user = format!("Now (local time, {zone}): {now_local} {day}\nReminder request: {text}");
    let request = crate::model::client::Request {
        system: include_str!("../prompts/schedule/interpret.md"),
        user_text: &user,
        think: false,
        max_tokens: Some(300),
        ..Default::default()
    };
    let value = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        client.call_json_retrying(&request, &schema()),
    )
    .await
    .map_err(|_| "The model took too long to read the reminder.".to_string())?
    .map_err(|e| e.to_string())?;
    validate(value, &now_local)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-10-30T22:46";

    #[test]
    fn weekday_matches_the_calendar() {
        assert_eq!(weekday(parse("2026-10-30T00:00").unwrap()), "Friday");
        assert_eq!(weekday(parse("2024-02-29T00:00").unwrap()), "Thursday");
        assert_eq!(weekday(parse("2026-01-01T00:00").unwrap()), "Thursday");
    }

    #[test]
    fn a_relative_delay_is_accepted_with_a_clean_title() {
        let ok = validate(
            json!({"kind":"after","title":" Drink water again ","delaySeconds":50,"local":""}),
            NOW,
        )
        .unwrap();
        assert_eq!(ok.title, "Drink water again");
        assert_eq!(ok.delay_seconds, 50);
    }

    #[test]
    fn an_absolute_time_must_be_valid_and_in_the_future() {
        let at = |local: &str| {
            validate(
                json!({"kind":"at","title":"Call","delaySeconds":0,"local":local}),
                NOW,
            )
        };
        assert!(at("2026-10-31T09:00").is_ok());
        assert!(at("2026-10-30T22:46").is_err());
        assert!(at("2026-10-29T09:00").is_err());
        assert!(at("tomorrow").is_err());
        assert!(at("2026-02-30T09:00").is_err());
    }

    #[test]
    fn unclear_zero_or_oversized_requests_are_not_scheduled() {
        let unclear = validate(
            json!({"kind":"unclear","title":"x","delaySeconds":9,"local":"y"}),
            NOW,
        )
        .unwrap();
        assert_eq!(unclear.kind, "unclear");
        assert!(unclear.title.is_empty() && unclear.delay_seconds == 0 && unclear.local.is_empty());
        for bad in [
            json!({"kind":"after","title":"x","delaySeconds":0,"local":""}),
            json!({"kind":"after","title":"x","delaySeconds":5_184_001u64,"local":""}),
            json!({"kind":"after","title":" ","delaySeconds":60,"local":""}),
            json!({"kind":"sometime","title":"x","delaySeconds":60,"local":""}),
            json!({"title":"x"}),
        ] {
            assert!(validate(bad, NOW).is_err());
        }
    }
}
