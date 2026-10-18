//! The tools an outside agent sees, as MCP (JSON-RPC 2.0) methods.
//!
//! Each action tool reuses the built-in agent's own pieces: `agent::protocol::parse` to check the
//! arguments, and `agent::body` to announce, perform and settle the action. So an outside agent's
//! click looks and behaves exactly like the model's, and Stop reaches it the same way.

use std::time::Instant;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    agent::{body, protocol},
    events::{self, AgentState, AppState, ExternalAgent, TaskDone},
    exec::{ExecError, input::parse_key, uia},
    observe::screenshot::{self, Frame, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
};

use super::lease::{IDLE_LIMIT, LeaseError};
use crate::agent::quick;
use crate::safety::gate::{self, Verdict};

use super::skill::GUIDE;
const LATEST_PROTOCOL: &str = "2025-06-18";
const SUPPORTED_PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const KEY_NAMES: &str = "ctrl, shift, alt, win, enter, tab, esc, backspace, delete, space, up, down, left, right, home, end, pageup, pagedown, f1 to f12, or one character like s";

/// One tool answer: some text, maybe a screenshot, and whether it is an error.
struct Reply {
    text: String,
    image: Option<Vec<u8>>,
    is_error: bool,
}

impl Reply {
    fn text(text: impl Into<String>) -> Self {
        Self { text: text.into(), image: None, is_error: false }
    }

    fn shot(text: impl Into<String>, jpeg: Vec<u8>) -> Self {
        Self { text: text.into(), image: Some(jpeg), is_error: false }
    }

    /// An error carries a short code from the guide, then a sentence on what to do.
    fn error(code: &str, message: impl std::fmt::Display) -> Self {
        Self { text: format!("{code}: {message}"), image: None, is_error: true }
    }

    fn into_result(self) -> Value {
        let mut content = vec![json!({ "type": "text", "text": self.text })];
        if let Some(jpeg) = self.image {
            content.push(json!({ "type": "image", "data": BASE64.encode(jpeg), "mimeType": "image/jpeg" }));
        }
        json!({ "content": content, "isError": self.is_error })
    }
}

impl From<LeaseError> for Reply {
    fn from(error: LeaseError) -> Self {
        let advice = match error {
            LeaseError::Busy => "Another agent, or Rune Studio's own agent, is working. Wait, or tell the user.",
            LeaseError::NotBegun => "Call rune_begin first.",
            LeaseError::StoppedByUser => "The user pressed Stop. Stop working and report. Do not try again unless the user asks.",
            LeaseError::Expired => "Control ended because there were no calls for a while. Ask the user, then call rune_begin again.",
        };
        Reply::error(error.code(), advice)
    }
}

fn rpc_result(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_error(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Handles one JSON-RPC message. `None` means it was a notification and needs no answer.
pub async fn handle(app: &AppHandle, request: Value) -> Option<Value> {
    let id = request.get("id").cloned()?;
    let method = request["method"].as_str().unwrap_or("");
    Some(match method {
        "initialize" => {
            let asked = request["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = if SUPPORTED_PROTOCOLS.contains(&asked) { asked } else { LATEST_PROTOCOL };
            rpc_result(
                &id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false }, "resources": {}, "prompts": {} },
                    "serverInfo": { "name": "rune-studio", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "Rune Studio lets you see the user's screen and use their mouse and keyboard. Call rune_guide first for how to use it, then rune_begin. Take one small step at a time and look at every screenshot.",
                }),
            )
        }
        "ping" => rpc_result(&id, json!({})),
        "resources/list" => rpc_result(&id, json!({ "resources": resource_list() })),
        "resources/read" => match read_resource(request["params"]["uri"].as_str().unwrap_or("")) {
            Some(contents) => rpc_result(&id, json!({ "contents": [contents] })),
            None => rpc_error(&id, -32002, "resource not found"),
        },
        "prompts/list" => rpc_result(&id, json!({ "prompts": prompt_list() })),
        "prompts/get" => match get_prompt(request["params"]["name"].as_str().unwrap_or(""), &request["params"]["arguments"]) {
            Some(prompt) => rpc_result(&id, prompt),
            None => rpc_error(&id, -32602, "unknown prompt"),
        },
        "tools/list" => rpc_result(&id, json!({ "tools": tool_list() })),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            let arguments = request["params"]["arguments"].clone();
            rpc_result(&id, call(app, name, &arguments).await.into_result())
        }
        _ => rpc_error(&id, -32601, "method not found"),
    })
}

/// The guide is also offered as an MCP resource and prompt, so a client that knows nothing about
/// skills still gets it.
const GUIDE_URI: &str = "rune://guide";

fn resource_list() -> Value {
    json!([{
        "uri": GUIDE_URI,
        "name": "rune-studio-guide",
        "title": "How to use Rune Studio",
        "description": "The guide for agents: the loop, tools, coordinates, safety rules and errors.",
        "mimeType": "text/markdown",
    }])
}

fn read_resource(uri: &str) -> Option<Value> {
    (uri == GUIDE_URI).then(|| json!({ "uri": GUIDE_URI, "mimeType": "text/markdown", "text": GUIDE }))
}

fn prompt_list() -> Value {
    json!([{
        "name": "rune_start",
        "title": "Do something on the user's PC",
        "description": "Gives you the Rune Studio guide and the task, so you can start working on the PC safely.",
        "arguments": [{ "name": "task", "description": "What the user wants done on their PC.", "required": false }],
    }])
}

fn get_prompt(name: &str, arguments: &Value) -> Option<Value> {
    if name != "rune_start" {
        return None;
    }
    let task = arguments["task"].as_str().map(str::trim).filter(|task| !task.is_empty());
    let lead = match task {
        Some(task) => format!("Use Rune Studio to do this on the user's PC: {task}

"),
        None => "Use Rune Studio to work on the user's PC.

".to_string(),
    };
    Some(json!({
        "description": "Rune Studio guide and task",
        "messages": [{ "role": "user", "content": { "type": "text", "text": format!("{lead}Read this guide first and follow it:

{GUIDE}") } }],
    }))
}

fn tool_list() -> Value {
    let reason = json!({ "type": "string", "description": "One short sentence for the user: why you are doing this." });
    let schema = |properties: Value, required: &[&str]| {
        let mut properties = properties;
        properties["reason"] = reason.clone();
        properties["goal"] = json!({ "type": "integer", "description": "Number of the goal from your plan that this action is for (starting at 1)." });
        properties["risk"] = json!({ "type": "string", "enum": ["low", "medium", "high"], "description": "How risky this action is. High means it deletes, sends, pays, installs or cannot be undone: the user is asked first." });
        json!({ "type": "object", "properties": properties, "required": required })
    };
    json!([
        { "name": "rune_guide", "description": "How to use Rune Studio: the full guide, in simple words. Read it first.",
          "inputSchema": { "type": "object", "properties": {} } },
        { "name": "rune_begin", "description": "Take control of the PC. The user sees a frame around the screen. Do this once before acting.",
          "inputSchema": { "type": "object", "properties": {
              "task": { "type": "string", "description": "What you were asked to do." },
              "agent": { "type": "string", "description": "Your name, shown to the user." },
              "plan": { "type": "array", "items": { "type": "string" }, "description": "Optional: your goals, the same as calling rune_plan." } }, "required": ["task"] } },
        { "name": "rune_observe", "description": "Take a screenshot. Pass region to zoom into part of the screen, which helps with small text and tiny buttons.",
          "inputSchema": { "type": "object", "properties": {
              "region": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4,
                          "description": "[x0, y0, x1, y1], each 0 to 1000 across the whole screen." } } } },
        { "name": "rune_click", "description": "Click a point. x and y run 0 to 1000 across the whole screenshot, left to right and top to bottom. Returns a fresh screenshot.",
          "inputSchema": schema(json!({
              "x": { "type": "integer", "description": "0 to 1000, left to right" },
              "y": { "type": "integer", "description": "0 to 1000, top to bottom" },
              "button": { "type": "string", "enum": ["left", "right", "middle"] },
              "double": { "type": "boolean" } }), &["x", "y"]) },
        { "name": "rune_elements", "description": "List the controls (buttons, links, tabs, fields) of the front window with short ids like e3. Then rune_click_element clicks one exactly, which beats guessing a point for small targets. Call it again after the screen changes: ids belong to the latest list.",
          "inputSchema": { "type": "object", "properties": {} } },
        { "name": "rune_click_element", "description": "Click a control by its id from the latest rune_elements list. Returns a fresh screenshot.",
          "inputSchema": schema(json!({
              "id": { "type": "string", "description": "An id such as e3" },
              "button": { "type": "string", "enum": ["left", "right", "middle"] },
              "double": { "type": "boolean" } }), &["id"]) },
        { "name": "rune_scroll", "description": "Scroll the mouse wheel over a point: up, down, left or right, 1 to 10 notches (3 is about a third of a page). Returns a fresh screenshot.",
          "inputSchema": schema(json!({
              "direction": { "type": "string", "enum": ["up", "down", "left", "right"] },
              "amount": { "type": "integer", "description": "1 to 10, default 3" },
              "x": { "type": "integer", "description": "0 to 1000, default 500" },
              "y": { "type": "integer", "description": "0 to 1000, default 500" } }), &["direction"]) },
        { "name": "rune_drag", "description": "Press at one point, move to another and let go: move a file or slider, or select a range. All four numbers run 0 to 1000. Returns a fresh screenshot.",
          "inputSchema": schema(json!({
              "x1": { "type": "integer" }, "y1": { "type": "integer" },
              "x2": { "type": "integer" }, "y2": { "type": "integer" } }), &["x1", "y1", "x2", "y2"]) },
        { "name": "rune_type_text", "description": "Type text into whatever has keyboard focus. Click the field first if unsure. Returns a fresh screenshot.",
          "inputSchema": schema(json!({ "text": { "type": "string" } }), &["text"]) },
        { "name": "rune_press_keys", "description": "Press a key or combination, for example [\"enter\"] or [\"ctrl\", \"s\"]. Returns a fresh screenshot.",
          "inputSchema": schema(json!({ "keys": { "type": "array", "items": { "type": "string" } } }), &["keys"]) },
        { "name": "rune_open_app", "description": "Open an app, file, folder or web address by name, like notepad, calc or https://example.com. Returns a fresh screenshot.",
          "inputSchema": schema(json!({ "name": { "type": "string" } }), &["name"]) },
        { "name": "rune_wait", "description": "Wait for something to load, at most 5000 ms. Returns a fresh screenshot.",
          "inputSchema": schema(json!({ "ms": { "type": "integer" } }), &[]) },
        { "name": "rune_plan", "description": "Write your goals for the task, two to six short ones in order. The user sees them as a progress tracker; then pass goal=1, 2, ... on your actions.",
          "inputSchema": { "type": "object", "properties": { "goals": { "type": "array", "items": { "type": "string" } } }, "required": ["goals"] } },
        { "name": "rune_status", "description": "Whether you are in control, the state, and how long until your control times out.",
          "inputSchema": { "type": "object", "properties": {} } },
        { "name": "rune_done", "description": "Finish. Gives control back and shows your summary to the user.",
          "inputSchema": { "type": "object", "properties": {
              "summary": { "type": "string" }, "success": { "type": "boolean" } }, "required": ["summary"] } },
    ])
}

/// The names the action tools map onto in the built-in protocol.
fn protocol_name(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "rune_click" => "click",
        "rune_click_element" => "click_element",
        "rune_scroll" => "scroll",
        "rune_drag" => "drag",
        "rune_type_text" => "type_text",
        "rune_press_keys" => "press_keys",
        "rune_open_app" => "open_app",
        "rune_wait" => "wait",
        _ => return None,
    })
}

async fn call(app: &AppHandle, name: &str, arguments: &Value) -> Reply {
    match name {
        "rune_guide" => Reply::text(GUIDE),
        "rune_begin" => begin(app, arguments),
        "rune_observe" => observe(app, arguments).await,
        "rune_plan" => plan(app, arguments),
        "rune_elements" => elements(app).await,
        "rune_status" => status(app),
        "rune_done" => done(app, arguments),
        other => match protocol_name(other) {
            Some(tool) => act(app, tool, arguments).await,
            None => Reply::error("unknown_tool", format!("there is no tool called {other}")),
        },
    }
}

fn clip(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}

fn screen_note(frame: &Frame) -> String {
    format!(
        "The screen is {}x{} pixels, shown here at {}x{}. Coordinates run 0 to 1000 on both axes: x left to right, y top to bottom, centre (500, 500).",
        frame.monitor.width, frame.monitor.height, frame.width, frame.height
    )
}

async fn capture(region: Option<[f64; 4]>) -> Result<Frame, Reply> {
    match tauri::async_runtime::spawn_blocking(move || screenshot::capture_region(None, region, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY)).await {
        Ok(Ok(frame)) => Ok(frame),
        Ok(Err(err)) => Err(Reply::error("screen_unavailable", err)),
        Err(err) => Err(Reply::error("screen_unavailable", err)),
    }
}

fn begin(app: &AppHandle, arguments: &Value) -> Reply {
    let state = app.state::<AppState>();
    if matches!(
        events::agent_state(app),
        AgentState::Planning | AgentState::AwaitingConfirm | AgentState::Acting | AgentState::Observing
    ) && !state.lease.lock().expect("lease lock poisoned").is_held()
    {
        return LeaseError::Busy.into();
    }
    let agent = Some(clip(arguments["agent"].as_str().unwrap_or(""), 40)).filter(|name| !name.is_empty()).unwrap_or_else(|| "External agent".into());
    let task = clip(arguments["task"].as_str().unwrap_or(""), 200);
    if let Err(err) = state.lease.lock().expect("lease lock poisoned").begin(&agent, &task, Instant::now()) {
        return err.into();
    }
    state.cancel.reset();
    let _ = app.emit(events::EXTERNAL_AGENT, ExternalAgent { agent: Some(agent.clone()) });
    events::set_agent_state(app, AgentState::Observing);
    log::info!("agent bridge: {agent} took control for \"{task}\"");
    if let Ok(goals) = protocol::parse_plan(&json!({ "goals": arguments["plan"] })) {
        set_goals(app, goals);
    }
    Reply::text(format!(
        "You are in control, {agent}. The user can see a frame around the screen and can press Stop at any time. \
         Take one small step at a time and look at every screenshot. \
         Coordinates run 0 to 1000 on both axes across the whole screen. \
         Keys you can press: {KEY_NAMES}. Control ends if you make no call for {} seconds. Call rune_done when finished.",
        IDLE_LIMIT.as_secs()
    ))
}

/// Marks a call by the lease holder, or says why it may not go on.
fn touch(app: &AppHandle) -> Result<u32, Reply> {
    let state = app.state::<AppState>();
    let mut book = state.lease.lock().expect("lease lock poisoned");
    let lease = book.touch(Instant::now()).map_err(Reply::from)?;
    lease.step += 1;
    Ok(lease.step)
}

fn parse_region(value: &Value) -> Result<Option<[f64; 4]>, Reply> {
    if value.is_null() {
        return Ok(None);
    }
    let numbers: Vec<f64> = value.as_array().map(|list| list.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
    match numbers.as_slice() {
        [x0, y0, x1, y1] if numbers.iter().all(|n| (0.0..=1000.0).contains(n)) => Ok(Some([*x0, *y0, *x1, *y1])),
        _ => Err(Reply::error("out_of_range", "region must be [x0, y0, x1, y1], four numbers from 0 to 1000")),
    }
}

async fn observe(app: &AppHandle, arguments: &Value) -> Reply {
    if let Err(reply) = touch(app) {
        return reply;
    }
    let region = match parse_region(&arguments["region"]) {
        Ok(region) => region,
        Err(reply) => return reply,
    };
    body::announce_observe(app, region);
    let frame = match capture(region).await {
        Ok(frame) => frame,
        Err(reply) => return reply,
    };
    let what = match region {
        Some([x0, y0, x1, y1]) => format!(
            "This is a zoomed view of the part of the screen from ({x0}, {y0}) to ({x1}, {y1}). To click something here, convert its position back to whole-screen units: x = {x0} + cx/1000 * {}, y = {y0} + cy/1000 * {}.",
            x1 - x0,
            y1 - y0
        ),
        None => "Here is the whole screen right now.".to_string(),
    };
    let text = format!("{what} {}", screen_note(&frame));
    Reply::shot(text, frame.jpeg)
}

/// Lists the front window's controls with ids, and remembers them for `rune_click_element`.
async fn elements(app: &AppHandle) -> Reply {
    if let Err(reply) = touch(app) {
        return reply;
    }
    let found = tauri::async_runtime::spawn_blocking(uia::snapshot).await.unwrap_or_default();
    let text = uia::describe(&found);
    uia::remember(found);
    if text.is_empty() {
        return Reply::text("No controls could be listed for the front window (it may not support it, or it may be one of Rune's own). Use rune_observe and click a point instead.");
    }
    Reply::text(format!("Controls in the front window. Use the id with rune_click_element; the ids are valid until the next list.\n{text}"))
}

async fn act(app: &AppHandle, tool: &'static str, arguments: &Value) -> Reply {
    let step = match touch(app) {
        Ok(step) => step,
        Err(reply) => return reply,
    };
    let decision = match protocol::parse(tool, arguments) {
        Ok(decision) => decision,
        Err(problem) => {
            let code = if problem.contains("outside the 0 to 1000") { "out_of_range" } else { "bad_arguments" };
            return Reply::error(code, problem);
        }
    };
    advance_goal(app, decision.goal);
    if let protocol::Action::PressKeys { keys } = &decision.action {
        if let Some(bad) = keys.iter().find(|key| parse_key(key).is_err()) {
            return Reply::error("unknown_key", format!("\"{bad}\" is not a key name. Use: {KEY_NAMES}"));
        }
    }

    let state = app.state::<AppState>();
    let cancel = state.cancel.clone();
    // Risky actions wait for the user's yes, exactly as the built-in agent's do.
    let source = state.lease.lock().expect("lease lock poisoned").agent().unwrap_or("External agent").to_string();
    match gate::check(app, &cancel, &source, &decision.action, &decision.reason, decision.risk).await {
        Verdict::Allow => {}
        Verdict::Stopped => return LeaseError::StoppedByUser.into(),
        Verdict::Denied(why) => {
            events::set_agent_state(app, AgentState::Observing);
            return Reply::error("declined_by_user", format!("the user did not allow it ({why}). Do not try another way to do the same thing; ask the user what they want instead."));
        }
    }
    let monitor = match screenshot::monitor(None) {
        Ok(monitor) => monitor,
        Err(err) => return Reply::error("screen_unavailable", err),
    };

    let action = decision.action;
    let described = body::announce(app, step, &decision.reason, &action, &monitor);
    events::set_agent_state(app, AgentState::Acting);
    let settle = body::settle_time(&action);
    let worker_cancel = cancel.clone();
    let worker_action = action.clone();
    let worker_app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        body::execute_reporting(&worker_app, &worker_action, &monitor, &worker_cancel)?;
        worker_cancel.sleep(settle)
    })
    .await
    .unwrap_or_else(|err| Err(ExecError::Input(err.to_string())));

    if matches!(result, Err(ExecError::Cancelled)) || cancel.is_cancelled() {
        // Stop already moved the state to Stopped and revoked the lease. Leave both alone.
        return LeaseError::StoppedByUser.into();
    }
    body::finish_step(app, step, &result);
    events::set_agent_state(app, AgentState::Observing);
    if let Err(err) = result {
        return Reply::error("action_failed", format!("could not {described}: {err}. Look at the screen and try something different."));
    }

    // If the decision model sees the page still loading, give it a moment (a few times at most)
    // so the screenshot the agent gets is of the finished page, not a half-drawn one.
    let mut attempt = 0;
    loop {
        let frame = match capture(None).await {
            Ok(frame) => frame,
            Err(reply) => return reply,
        };
        let notes = quick::read_screen(app, &frame.data_uri()).await.unwrap_or_default();
        if notes.loading && attempt < quick::MAX_LOAD_WAITS && !cancel.is_cancelled() {
            attempt += 1;
            tokio::time::sleep(std::time::Duration::from_millis(900)).await;
            continue;
        }
        let mut text = format!("Done: {described}. Here is the screen now. {}", screen_note(&frame));
        if notes.loading {
            text.push_str(" It may still be loading: look again with rune_observe in a moment.");
        }
        if let Some(hint) = notes.hint {
            text.push_str(&format!(" Note: {hint}"));
        }
        return Reply::shot(text, frame.jpeg);
    }
}

/// Saves the agent's goals and shows them to the user as the progress tracker.
fn set_goals(app: &AppHandle, goals: Vec<String>) {
    let state = app.state::<AppState>();
    if let Some(lease) = state.lease.lock().expect("lease lock poisoned").current_mut() {
        lease.plan = goals.clone();
        lease.current_goal = 0;
    }
    events::emit_plan(app, &goals, 0, false);
}

fn plan(app: &AppHandle, arguments: &Value) -> Reply {
    if let Err(reply) = touch(app) {
        return reply;
    }
    match protocol::parse_plan(arguments) {
        Ok(goals) => {
            let listing = goals.iter().enumerate().map(|(index, goal)| format!("{}. {goal}", index + 1)).collect::<Vec<_>>().join(" ");
            set_goals(app, goals);
            Reply::text(format!("Plan saved: {listing} Now pass goal=1 (then 2, 3, ...) on your actions so the user sees your progress."))
        }
        Err(problem) => Reply::error("bad_arguments", problem),
    }
}

/// The agent says which goal an action is for. Goals only move forward.
fn advance_goal(app: &AppHandle, goal: Option<u32>) {
    let Some(goal) = goal else { return };
    let state = app.state::<AppState>();
    let update = {
        let mut book = state.lease.lock().expect("lease lock poisoned");
        book.current_mut().and_then(|lease| {
            if lease.plan.is_empty() {
                return None;
            }
            let index = (goal as usize - 1).min(lease.plan.len() - 1);
            (index > lease.current_goal).then(|| {
                lease.current_goal = index;
                (lease.plan.clone(), index)
            })
        })
    };
    if let Some((goals, index)) = update {
        events::emit_plan(app, &goals, index, false);
    }
}

fn status(app: &AppHandle) -> Reply {
    let state = app.state::<AppState>();
    let book = state.lease.lock().expect("lease lock poisoned");
    let text = match book.current() {
        Some(lease) => format!(
            "You are in control as {}, working on: {}. Actions so far: {}. The screen state is {:?}. Control ends after {} seconds with no call.",
            lease.agent,
            lease.task,
            lease.step,
            events::agent_state(app),
            IDLE_LIMIT.as_secs()
        ),
        None if book.is_revoked() => return LeaseError::StoppedByUser.into(),
        None => "You are not in control. Call rune_begin to take control.".to_string(),
    };
    Reply::text(text)
}

fn done(app: &AppHandle, arguments: &Value) -> Reply {
    if let Err(reply) = touch(app) {
        return reply;
    }
    let state = app.state::<AppState>();
    let summary = clip(arguments["summary"].as_str().unwrap_or(""), 400);
    let success = arguments["success"].as_bool().unwrap_or(true);
    let finished_plan = state.lease.lock().expect("lease lock poisoned").current().map(|lease| lease.plan.clone()).filter(|plan| !plan.is_empty());
    if let Some(plan) = finished_plan {
        events::emit_plan(app, &plan, plan.len(), true);
    }
    state.lease.lock().expect("lease lock poisoned").end();
    let _ = app.emit(events::EXTERNAL_AGENT, ExternalAgent { agent: None });
    let _ = app.emit(events::TASK_DONE, TaskDone { summary, success });
    events::set_agent_state(app, if success { AgentState::Done } else { AgentState::Failed });
    Reply::text("Thanks. Control is back with the user and your summary is shown to them.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_in_the_list_has_a_handler_or_is_special() {
        let special = ["rune_guide", "rune_begin", "rune_observe", "rune_plan", "rune_elements", "rune_status", "rune_done"];
        for tool in tool_list().as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            assert!(special.contains(&name) || protocol_name(name).is_some(), "{name} has no handler");
            assert_eq!(tool["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn the_guide_documents_every_tool() {
        for tool in tool_list().as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            assert!(GUIDE.contains(&format!("`{name}`")), "the guide does not mention {name}");
        }
    }

    #[test]
    fn the_guide_documents_every_error_code() {
        for code in ["not_enabled", "unauthorized", "busy", "not_begun", "stopped_by_user", "lease_expired", "out_of_range", "unknown_key", "action_failed", "screen_unavailable", "bad_arguments", "unknown_tool"] {
            assert!(GUIDE.contains(&format!("`{code}`")), "the guide does not mention {code}");
        }
    }

    #[test]
    fn the_guide_is_offered_as_a_resource_and_a_prompt() {
        assert_eq!(resource_list()[0]["uri"], "rune://guide");
        let contents = read_resource("rune://guide").unwrap();
        assert!(contents["text"].as_str().unwrap().contains("rune_begin"));
        assert!(read_resource("rune://nothing").is_none());

        assert_eq!(prompt_list()[0]["name"], "rune_start");
        let with_task = get_prompt("rune_start", &json!({ "task": "open Notepad" })).unwrap();
        let text = with_task["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("open Notepad") && text.contains("rune_begin"));
        let without = get_prompt("rune_start", &Value::Null).unwrap();
        assert!(without["messages"][0]["content"]["text"].as_str().unwrap().starts_with("Use Rune Studio to work"));
        assert!(get_prompt("other", &Value::Null).is_none());
    }

    #[test]
    fn regions_must_be_four_numbers_in_range() {
        assert!(matches!(parse_region(&json!([0, 0, 500, 500])), Ok(Some(_))));
        assert!(matches!(parse_region(&Value::Null), Ok(None)));
        assert!(parse_region(&json!([0, 0, 500])).is_err());
        assert!(parse_region(&json!([0, 0, 500, 1200])).is_err());
        assert!(parse_region(&json!("all")).is_err());
    }

    #[test]
    fn errors_are_flagged_and_screenshots_become_image_content() {
        let error = Reply::error("busy", "wait").into_result();
        assert_eq!(error["isError"], true);
        assert_eq!(error["content"][0]["text"], "busy: wait");
        let shot = Reply::shot("hi", vec![1, 2, 3]).into_result();
        assert_eq!(shot["content"][1]["type"], "image");
        assert_eq!(shot["content"][1]["mimeType"], "image/jpeg");
    }
}
