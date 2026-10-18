//! The agent loop: look at the screen, ask the model for one action, do it, repeat.
//!
//! One run is one task. Stop (hotkey, tray or button) cancels the shared token; every wait in
//! here, including the model request, watches it, so Stop is felt within milliseconds.

pub mod body;
pub mod prompt;
pub mod protocol;

use std::{future::Future, time::Duration};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    events::{self, AgentState, AppState, TaskDone},
    exec::{CancelToken, ExecError},
    model::client::{ChatClient, ModelError},
    observe::screenshot::{self, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
    settings,
};

use prompt::Entry;
use protocol::{Action, Decision};

/// Give up after this many unusable model answers in a row.
const MAX_INVALID_IN_A_ROW: u32 = 3;

/// How a run ended.
#[derive(Debug, PartialEq)]
enum Outcome {
    Finished { summary: String, success: bool },
    Stopped,
    Failed(String),
}

/// Starts a task on a background task and returns at once.
pub fn start(app: &AppHandle, task: String) {
    // Synchronously, so a second start_task sees a running agent and is refused.
    app.state::<AppState>().cancel.reset();
    events::set_agent_state(app, AgentState::Planning);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let outcome = run(&app, &task).await;
        finish(&app, outcome);
    });
}

fn finish(app: &AppHandle, outcome: Outcome) {
    match outcome {
        // Stop already moved the state machine to Stopped.
        Outcome::Stopped => {}
        Outcome::Finished { summary, success } => {
            log::info!("task finished (success: {success}): {summary}");
            let _ = app.emit(events::TASK_DONE, TaskDone { summary, success });
            events::set_agent_state(app, if success { AgentState::Done } else { AgentState::Failed });
        }
        Outcome::Failed(message) => {
            log::warn!("task failed: {message}");
            let _ = app.emit(events::TASK_DONE, TaskDone { summary: message, success: false });
            events::set_agent_state(app, AgentState::Failed);
        }
    }
}

/// Resolves to `None` if Stop is pressed before the future completes. The future is dropped, so
/// an in-flight model request is aborted rather than waited for.
async fn unless_stopped<T>(cancel: &CancelToken, work: impl Future<Output = T>) -> Option<T> {
    tokio::select! {
        output = work => Some(output),
        () = async { while !cancel.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await } } => None,
    }
}

async fn run(app: &AppHandle, task: &str) -> Outcome {
    let state = app.state::<AppState>();
    let cancel = state.cancel.clone();
    let settings = state.settings.lock().expect("settings lock poisoned").clone();
    let api_key = settings::api_key();
    let tools = protocol::tools();

    let mut history: Vec<Entry> = Vec::new();
    let mut invalid_in_a_row = 0;

    for step in 1..=settings.max_steps {
        if cancel.is_cancelled() {
            return Outcome::Stopped;
        }

        // Look.
        events::set_agent_state(app, AgentState::Observing);
        body::announce_observe(app, None);
        let captured = tauri::async_runtime::spawn_blocking(|| screenshot::capture(None, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY)).await;
        let frame = match captured {
            Ok(Ok(frame)) => frame,
            Ok(Err(err)) => return Outcome::Failed(format!("could not see the screen: {err}")),
            Err(err) => return Outcome::Failed(err.to_string()),
        };

        // Decide.
        events::set_agent_state(app, AgentState::Planning);
        let client = ChatClient {
            http: &state.http,
            base_url: &settings.base_url,
            api_key: api_key.as_deref(),
            model: &settings.vision_model,
        };
        let text = prompt::user_message(task, &history, step, settings.max_steps);
        let started = std::time::Instant::now();
        let Some(reply) = unless_stopped(&cancel, client.next_tool_call(prompt::SYSTEM, &text, &frame.data_uri(), &tools)).await else {
            return Outcome::Stopped;
        };
        let reply = match reply {
            Ok(reply) => reply,
            Err(err @ ModelError::NoToolCall(_)) => {
                invalid_in_a_row += 1;
                if invalid_in_a_row >= MAX_INVALID_IN_A_ROW {
                    return Outcome::Failed(err.to_string());
                }
                history.push(Entry { action: "(no action)".into(), outcome: "you must call a tool".into() });
                continue;
            }
            Err(err) => return Outcome::Failed(err.to_string()),
        };
        log::info!("step {step}: {} in {} ms ({:?} tokens)", reply.call.name, started.elapsed().as_millis(), reply.completion_tokens);

        let Decision { action, reason } = match protocol::parse(&reply.call.name, &reply.call.arguments) {
            Ok(decision) => decision,
            Err(problem) => {
                invalid_in_a_row += 1;
                if invalid_in_a_row >= MAX_INVALID_IN_A_ROW {
                    return Outcome::Failed(problem);
                }
                history.push(Entry { action: reply.call.name, outcome: format!("invalid: {problem}") });
                continue;
            }
        };
        invalid_in_a_row = 0;

        if let Action::Done { summary, success } = action {
            return Outcome::Finished { summary, success };
        }

        // Act.
        let described = body::announce(app, step, &reason, &action, &frame.monitor);
        events::set_agent_state(app, AgentState::Acting);
        let settle = body::settle_time(&action);
        let monitor = frame.monitor;
        let worker_cancel = cancel.clone();
        let executed = tauri::async_runtime::spawn_blocking(move || body::execute(&action, &monitor, &worker_cancel)).await;
        let result = match executed {
            Ok(result) => result,
            Err(err) => return Outcome::Failed(err.to_string()),
        };
        if !matches!(result, Err(ExecError::Cancelled)) {
            body::finish_step(app, step, &result);
        }
        match result {
            Err(ExecError::Cancelled) => return Outcome::Stopped,
            Err(err) => {
                history.push(Entry { action: described, outcome: format!("failed: {err}") });
            }
            Ok(()) => {
                history.push(Entry { action: described, outcome: "done".into() });
            }
        }

        if cancel.sleep(settle).is_err() {
            return Outcome::Stopped;
        }
    }
    Outcome::Failed(format!("stopped after {} steps without finishing", settings.max_steps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stop_aborts_a_pending_future() {
        let cancel = CancelToken::default();
        let stopper = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            stopper.cancel();
        });
        let outcome = unless_stopped(&cancel, tokio::time::sleep(Duration::from_secs(30))).await;
        assert!(outcome.is_none());
    }

    #[tokio::test]
    async fn a_finished_future_wins_when_there_is_no_stop() {
        let cancel = CancelToken::default();
        assert_eq!(unless_stopped(&cancel, async { 7 }).await, Some(7));
    }
}
