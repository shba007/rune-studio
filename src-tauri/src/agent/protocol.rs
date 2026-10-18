//! The actions the model can take, as tool definitions it is shown and the typed form we parse
//! its answer into. One tool per action; the model must call exactly one each turn.

use serde_json::{Value, json};

use crate::{
    exec::{
        input::{MouseButton, ScrollDirection},
        uia,
    },
    safety::Risk,
};

/// Longest pause the model may ask for.
const MAX_WAIT_MS: u64 = 5_000;
/// Most wheel notches in one scroll.
const MAX_SCROLL: u32 = 10;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A point in model space (0..=1000 on both axes).
    Click { x: f64, y: f64, button: MouseButton, double: bool },
    /// A control from the latest element list (see `exec::uia`), clicked at its middle.
    ClickElement { id: String, button: MouseButton, double: bool },
    /// Scroll the wheel with the pointer at a point in model space.
    Scroll { x: f64, y: f64, direction: ScrollDirection, amount: u32 },
    /// Press at one point in model space, move to another, release.
    Drag { from: (f64, f64), to: (f64, f64) },
    /// Ask the user a yes/no question. The loop handles it; it never touches the desktop.
    AskUser { question: String },
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
            Action::ClickElement { .. } => "click_element",
            Action::Scroll { .. } => "scroll",
            Action::Drag { .. } => "drag",
            Action::AskUser { .. } => "ask_user",
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
            Action::ClickElement { id, button, double } => {
                let kind = match (button, double) {
                    (MouseButton::Left, false) => "click",
                    (MouseButton::Left, true) => "double-click",
                    (MouseButton::Right, _) => "right-click",
                    (MouseButton::Middle, _) => "middle-click",
                };
                match uia::lookup(id) {
                    Some(element) => format!("{kind} {} \"{}\" ({id})", element.role, element.name.chars().take(40).collect::<String>()),
                    None => format!("{kind} element {id}"),
                }
            }
            Action::Scroll { direction, amount, .. } => format!("scroll {} {amount}", direction.name()),
            Action::Drag { from, to } => format!("drag ({}, {}) to ({}, {})", from.0.round(), from.1.round(), to.0.round(), to.1.round()),
            Action::AskUser { question } => format!("ask the user: {}", question.chars().take(80).collect::<String>()),
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
    /// How risky the agent itself says the action is. Our own rules may raise it.
    pub risk: Option<Risk>,
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
        "click_element" => {
            let id = text("id")?;
            if id.trim().is_empty() {
                return Err("click_element: \"id\" must be an element id like e3".into());
            }
            let button = match arguments["button"].as_str() {
                Some(button) => button.parse().map_err(|_| format!("click_element: unknown button \"{button}\""))?,
                None => MouseButton::Left,
            };
            Action::ClickElement { id: id.trim().to_string(), button, double: arguments["double"].as_bool().unwrap_or(false) }
        }
        "scroll" => {
            let x = number(&arguments["x"]).unwrap_or(500.0);
            let y = number(&arguments["y"]).unwrap_or(500.0);
            if !(0.0..=1000.0).contains(&x) || !(0.0..=1000.0).contains(&y) {
                return Err(format!("scroll: ({x}, {y}) is outside the 0 to 1000 range"));
            }
            let direction = arguments["direction"].as_str().ok_or("scroll: missing \"direction\"")?;
            let direction = direction.parse().map_err(|_| format!("scroll: direction must be up, down, left or right, not \"{direction}\""))?;
            let amount = number(&arguments["amount"]).map_or(3, |amount| amount.max(1.0) as u32).min(MAX_SCROLL);
            Action::Scroll { x, y, direction, amount }
        }
        "drag" => {
            let point = |key_x: &str, key_y: &str| -> Result<(f64, f64), String> {
                let x = number(&arguments[key_x]).ok_or(format!("drag: missing \"{key_x}\""))?;
                let y = number(&arguments[key_y]).ok_or(format!("drag: missing \"{key_y}\""))?;
                if !(0.0..=1000.0).contains(&x) || !(0.0..=1000.0).contains(&y) {
                    return Err(format!("drag: ({x}, {y}) is outside the 0 to 1000 range"));
                }
                Ok((x, y))
            };
            Action::Drag { from: point("x1", "y1")?, to: point("x2", "y2")? }
        }
        "ask_user" => {
            let question = text("question")?;
            if question.trim().is_empty() {
                return Err("ask_user: \"question\" must not be empty".into());
            }
            Action::AskUser { question: question.trim().to_string() }
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
    let risk = arguments["risk"].as_str().and_then(Risk::parse);
    Ok(Decision { action, reason, goal, risk })
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
    properties["risk"] = json!({ "type": "string", "enum": ["low", "medium", "high"], "description": "How risky this action is. High means it deletes, sends, pays, installs or cannot be undone: the user is asked first." });
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
            "click_element",
            "Click a control from the element list (lines like: e3 button Save). More exact than click for small buttons, links, tabs and fields. Use only ids from the current list.",
            json!({
                "id": { "type": "string", "description": "An element id such as e3" },
                "button": { "type": "string", "enum": ["left", "right", "middle"] },
                "double": { "type": "boolean" },
            }),
            &["id"],
        ),
        tool(
            "scroll",
            "Scroll with the mouse wheel over a point, to see more of a page, list or document.",
            json!({
                "x": { "type": "integer", "description": "0 to 1000, default 500 (the middle)" },
                "y": { "type": "integer", "description": "0 to 1000, default 500" },
                "direction": { "type": "string", "enum": ["up", "down", "left", "right"] },
                "amount": { "type": "integer", "description": "Wheel notches, 1 to 10, default 3 (about a third of a page)" },
            }),
            &["direction"],
        ),
        tool(
            "drag",
            "Press at one point, move to another and let go: to move a file, a slider or a window, or select a range.",
            json!({
                "x1": { "type": "integer", "description": "Start, 0 to 1000" },
                "y1": { "type": "integer" },
                "x2": { "type": "integer", "description": "End, 0 to 1000" },
                "y2": { "type": "integer" },
            }),
            &["x1", "y1", "x2", "y2"],
        ),
        tool(
            "ask_user",
            "Ask the user a yes/no question when you cannot tell what they want or something unexpected needs their decision. Do not ask what you can see on the screen.",
            json!({ "question": { "type": "string", "description": "A short question that can be answered yes or no" } }),
            &["question"],
        ),
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
    fn the_risk_an_action_declares_is_read() {
        assert_eq!(parse("wait", &json!({ "risk": "high" })).unwrap().risk, Some(Risk::High));
        assert_eq!(parse("wait", &json!({ "risk": "Medium" })).unwrap().risk, Some(Risk::Medium));
        assert_eq!(parse("wait", &json!({ "risk": "scary" })).unwrap().risk, None);
        assert_eq!(parse("wait", &json!({})).unwrap().risk, None);
        for tool in tools().as_array().unwrap() {
            assert!(tool["function"]["parameters"]["properties"]["risk"].is_object());
        }
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
        assert_eq!(names, ["click", "click_element", "scroll", "drag", "ask_user", "type_text", "press_keys", "open_app", "wait", "done"]);
        // Every tool the model is offered must parse (with the arguments it requires).
        let samples = [
            ("click", json!({ "x": 1, "y": 2 })),
            ("click_element", json!({ "id": "e1" })),
            ("scroll", json!({ "direction": "down" })),
            ("drag", json!({ "x1": 1, "y1": 2, "x2": 3, "y2": 4 })),
            ("ask_user", json!({ "question": "Ok?" })),
            ("type_text", json!({ "text": "a" })),
            ("press_keys", json!({ "keys": ["a"] })),
            ("open_app", json!({ "name": "notepad" })),
            ("wait", json!({})),
            ("done", json!({ "summary": "s" })),
        ];
        for (name, arguments) in samples {
            assert!(parse(name, &arguments).is_ok(), "{name} does not parse");
        }
    }

    #[test]
    fn click_element_needs_an_id_and_reads_the_button() {
        let ok = parse("click_element", &json!({ "id": " e7 ", "button": "right", "double": true })).unwrap();
        assert_eq!(ok.action, Action::ClickElement { id: "e7".into(), button: MouseButton::Right, double: true });
        assert!(parse("click_element", &json!({})).is_err());
        assert!(parse("click_element", &json!({ "id": "  " })).is_err());
        assert!(parse("click_element", &json!({ "id": "e1", "button": "side" })).is_err());
    }

    #[test]
    fn scroll_defaults_to_the_middle_and_three_notches_and_is_capped() {
        let plain = parse("scroll", &json!({ "direction": "Down" })).unwrap();
        assert_eq!(plain.action, Action::Scroll { x: 500.0, y: 500.0, direction: ScrollDirection::Down, amount: 3 });
        let big = parse("scroll", &json!({ "direction": "up", "amount": 99, "x": 100, "y": 900 })).unwrap();
        assert_eq!(big.action, Action::Scroll { x: 100.0, y: 900.0, direction: ScrollDirection::Up, amount: MAX_SCROLL });
        assert!(parse("scroll", &json!({ "direction": "sideways" })).is_err());
        assert!(parse("scroll", &json!({})).is_err());
        assert!(parse("scroll", &json!({ "direction": "up", "x": 2000 })).unwrap_err().contains("outside the 0 to 1000"));
    }

    #[test]
    fn drag_needs_four_points_in_range() {
        let ok = parse("drag", &json!({ "x1": 10, "y1": 20, "x2": "30", "y2": 40 })).unwrap();
        assert_eq!(ok.action, Action::Drag { from: (10.0, 20.0), to: (30.0, 40.0) });
        assert!(parse("drag", &json!({ "x1": 10, "y1": 20, "x2": 30 })).unwrap_err().contains("y2"));
        assert!(parse("drag", &json!({ "x1": 10, "y1": 20, "x2": 30, "y2": 1001 })).unwrap_err().contains("outside the 0 to 1000"));
    }

    #[test]
    fn ask_user_needs_a_question() {
        let ok = parse("ask_user", &json!({ "question": " Delete the draft? " })).unwrap();
        assert_eq!(ok.action, Action::AskUser { question: "Delete the draft?".into() });
        assert!(parse("ask_user", &json!({ "question": " " })).is_err());
        assert!(parse("ask_user", &json!({})).is_err());
    }
}
