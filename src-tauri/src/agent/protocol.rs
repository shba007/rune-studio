//! The actions the model can take, as tool definitions it is shown and the typed form we parse
//! its answer into. One tool per action; the model must call exactly one each turn.

use serde_json::{Value, json};

use crate::exec::input::MouseButton;

/// Longest pause the model may ask for.
const MAX_WAIT_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A point in model space (0..=1000 on both axes).
    Click { x: f64, y: f64, button: MouseButton, double: bool },
    TypeText { text: String },
    PressKeys { keys: Vec<String> },
    OpenApp { name: String },
    Wait { ms: u64 },
    Done { summary: String, success: bool },
}

impl Action {
    pub fn tool_name(&self) -> &'static str {
        match self {
            Action::Click { .. } => "click",
            Action::TypeText { .. } => "type_text",
            Action::PressKeys { .. } => "press_keys",
            Action::OpenApp { .. } => "open_app",
            Action::Wait { .. } => "wait",
            Action::Done { .. } => "done",
        }
    }

    /// One short line for the history the model sees next turn and for the log.
    pub fn describe(&self) -> String {
        match self {
            Action::Click { x, y, button, double } => {
                let kind = match (button, double) {
                    (MouseButton::Left, false) => "click",
                    (MouseButton::Left, true) => "double-click",
                    (MouseButton::Right, _) => "right-click",
                    (MouseButton::Middle, _) => "middle-click",
                };
                format!("{kind} ({}, {})", x.round(), y.round())
            }
            Action::TypeText { text } => format!("type \"{}\"", text.chars().take(60).collect::<String>()),
            Action::PressKeys { keys } => format!("press {}", keys.join("+")),
            Action::OpenApp { name } => format!("open {name}"),
            Action::Wait { ms } => format!("wait {ms} ms"),
            Action::Done { summary, .. } => format!("done: {summary}"),
        }
    }
}

/// What the model decided: the action plus its one-sentence reason (shown in the pill).
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub action: Action,
    pub reason: String,
    /// The goal (1-based, from the plan) this action is for, when the model said.
    pub goal: Option<u32>,
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

/// Turns a tool call into a typed decision. Arguments come from a language model, so every
/// field is checked and a wrong shape is an error the loop can report back to the model.
pub fn parse(name: &str, arguments: &Value) -> Result<Decision, String> {
    let reason = arguments["reason"].as_str().unwrap_or("").trim().to_string();
    let text = |key: &str| arguments[key].as_str().map(str::to_string).ok_or_else(|| format!("{name}: missing \"{key}\""));

    let action = match name {
        "click" => {
            let x = number(&arguments["x"]).ok_or("click: missing \"x\"")?;
            let y = number(&arguments["y"]).ok_or("click: missing \"y\"")?;
            if !(0.0..=1000.0).contains(&x) || !(0.0..=1000.0).contains(&y) {
                return Err(format!("click: ({x}, {y}) is outside the 0 to 1000 range"));
            }
            let button = match arguments["button"].as_str() {
                Some(button) => button.parse().map_err(|_| format!("click: unknown button \"{button}\""))?,
                None => MouseButton::Left,
            };
            Action::Click { x, y, button, double: arguments["double"].as_bool().unwrap_or(false) }
        }
        "type_text" => Action::TypeText { text: text("text")? },
        "press_keys" => {
            let keys: Vec<String> = arguments["keys"]
                .as_array()
                .map(|list| list.iter().filter_map(|key| key.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            if keys.is_empty() {
                return Err("press_keys: \"keys\" must be a non-empty list like [\"ctrl\", \"s\"]".into());
            }
            Action::PressKeys { keys }
        }
        "open_app" => Action::OpenApp { name: text("name")? },
        "wait" => Action::Wait { ms: number(&arguments["ms"]).map_or(1_000, |ms| ms.max(0.0) as u64).min(MAX_WAIT_MS) },
        "done" => Action::Done { summary: text("summary")?, success: arguments["success"].as_bool().unwrap_or(true) },
        other => return Err(format!("unknown tool \"{other}\"")),
    };
    let goal = number(&arguments["goal"]).filter(|goal| *goal >= 1.0).map(|goal| goal as u32);
    Ok(Decision { action, reason, goal })
}

/// Most goals a plan may have. More than this is a to-do list, not a plan.
pub const MAX_GOALS: usize = 6;

/// Checks the goals the model wrote: trimmed, non-empty, short, and not too many.
pub fn parse_plan(arguments: &Value) -> Result<Vec<String>, String> {
    let goals: Vec<String> = arguments["goals"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(|goal| goal.trim().chars().take(80).collect::<String>())
                .filter(|goal| !goal.is_empty())
                .take(MAX_GOALS)
                .collect()
        })
        .unwrap_or_default();
    if goals.is_empty() {
        return Err("set_plan: \"goals\" must be a list of one to six short goals".into());
    }
    Ok(goals)
}

/// The one tool offered for the planning call.
pub fn plan_tools() -> Value {
    json!([{ "type": "function", "function": {
        "name": "set_plan",
        "description": "Write the goals for this task, in order.",
        "parameters": { "type": "object", "required": ["goals"], "properties": {
            "goals": { "type": "array", "items": { "type": "string" }, "description": "Two to six short goals (a few words each), in the order they will be done. A simple task can have just one or two." },
        }},
    }}])
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    let mut properties = properties;
    properties["reason"] = json!({ "type": "string", "description": "One short sentence: why this action." });
    properties["goal"] = json!({ "type": "integer", "description": "Number of the goal (from the plan) this action is for, starting at 1." });
    json!({ "type": "function", "function": {
        "name": name,
        "description": description,
        "parameters": { "type": "object", "properties": properties, "required": required },
    }})
}

/// The tool list sent with every request.
pub fn tools() -> Value {
    let mut click_properties = json!({
        "x": { "type": "integer", "description": "Horizontal position, 0 to 1000, left to right across the screenshot" },
        "y": { "type": "integer", "description": "Vertical position, 0 to 1000, top to bottom across the screenshot" },
    });
    click_properties["button"] = json!({ "type": "string", "enum": ["left", "right", "middle"] });
    click_properties["double"] = json!({ "type": "boolean", "description": "Double-click, for opening files and folders" });

    json!([
        tool("click", "Click a point on the screen.", click_properties, &["x", "y"]),
        tool(
            "type_text",
            "Type text into the control that currently has keyboard focus. Click the field first if needed.",
            json!({ "text": { "type": "string" } }),
            &["text"],
        ),
        tool(
            "press_keys",
            "Press a key or a key combination, such as [\"enter\"] or [\"ctrl\", \"s\"].",
            json!({ "keys": { "type": "array", "items": { "type": "string" } } }),
            &["keys"],
        ),
        tool(
            "open_app",
            "Launch an application or open a file, folder or web address by name, for example \"notepad\" or \"calc\".",
            json!({ "name": { "type": "string" } }),
            &["name"],
        ),
        tool(
            "wait",
            "Wait for something to load or finish.",
            json!({ "ms": { "type": "integer", "description": "Milliseconds, at most 5000" } }),
            &[],
        ),
        tool(
            "done",
            "Finish the task. Use success=false when the task cannot be done.",
            json!({ "summary": { "type": "string" }, "success": { "type": "boolean" } }),
            &["summary"],
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_parses_numbers_and_numeric_strings() {
        let decision = parse("click", &json!({ "x": 500, "y": "250", "reason": "open it" })).unwrap();
        assert_eq!(decision.action, Action::Click { x: 500.0, y: 250.0, button: MouseButton::Left, double: false });
        assert_eq!(decision.reason, "open it");
    }

    #[test]
    fn click_outside_the_range_is_rejected() {
        assert!(parse("click", &json!({ "x": 1200, "y": 10 })).is_err());
        assert!(parse("click", &json!({ "x": 10 })).is_err());
        assert!(parse("click", &json!({ "x": 1, "y": 1, "button": "wheel" })).is_err());
    }

    #[test]
    fn keys_must_be_a_non_empty_list() {
        assert!(parse("press_keys", &json!({ "keys": [] })).is_err());
        let decision = parse("press_keys", &json!({ "keys": ["ctrl", "s"] })).unwrap();
        assert_eq!(decision.action, Action::PressKeys { keys: vec!["ctrl".into(), "s".into()] });
    }

    #[test]
    fn wait_is_capped() {
        assert_eq!(parse("wait", &json!({ "ms": 999_999 })).unwrap().action, Action::Wait { ms: MAX_WAIT_MS });
        assert_eq!(parse("wait", &json!({})).unwrap().action, Action::Wait { ms: 1_000 });
    }

    #[test]
    fn done_defaults_to_success_and_unknown_tools_fail() {
        assert_eq!(
            parse("done", &json!({ "summary": "ok" })).unwrap().action,
            Action::Done { summary: "ok".into(), success: true }
        );
        assert!(parse("format_disk", &json!({})).is_err());
    }

    #[test]
    fn the_goal_an_action_serves_is_read() {
        assert_eq!(parse("wait", &json!({ "goal": 2 })).unwrap().goal, Some(2));
        assert_eq!(parse("wait", &json!({ "goal": "3" })).unwrap().goal, Some(3));
        assert_eq!(parse("wait", &json!({ "goal": 0 })).unwrap().goal, None);
        assert_eq!(parse("wait", &json!({})).unwrap().goal, None);
    }

    #[test]
    fn a_plan_is_cleaned_and_limited() {
        let goals = parse_plan(&json!({ "goals": ["  Open Notepad ", "", "Type hello", 7] })).unwrap();
        assert_eq!(goals, ["Open Notepad", "Type hello"]);
        let many: Vec<String> = (1..=10).map(|n| format!("goal {n}")).collect();
        assert_eq!(parse_plan(&json!({ "goals": many })).unwrap().len(), MAX_GOALS);
        assert!(parse_plan(&json!({ "goals": [] })).is_err());
        assert!(parse_plan(&json!({})).is_err());
        let long = "x".repeat(200);
        assert_eq!(parse_plan(&json!({ "goals": [long] })).unwrap()[0].chars().count(), 80);
    }

    #[test]
    fn every_action_tool_accepts_a_goal() {
        for tool in tools().as_array().unwrap() {
            assert!(tool["function"]["parameters"]["properties"]["goal"].is_object(), "{} has no goal", tool["function"]["name"]);
        }
        assert_eq!(plan_tools()[0]["function"]["name"], "set_plan");
    }

    #[test]
    fn every_tool_has_a_matching_parser() {
        let names: Vec<String> =
            tools().as_array().unwrap().iter().map(|tool| tool["function"]["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, ["click", "type_text", "press_keys", "open_app", "wait", "done"]);
    }
}
