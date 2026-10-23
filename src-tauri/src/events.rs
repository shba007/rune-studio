use crate::sync::LockExt;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    bridge::lease::LeaseBook,
    exec::{self, RunScope},
    hotkeys::Hotkeys,
    overlay,
    safety::gate::Pending,
    settings::Settings,
    voice::capture::Capture,
    window,
};

pub const STATE_CHANGED: &str = "state_changed";
pub const TASK_VERIFIED: &str = "task_verified";
pub const ANALYSIS_UPDATED: &str = "analysis_updated";
pub const SUMMON: &str = "summon";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Listening,
    Planning,
    AwaitingConfirm,
    Acting,
    Observing,
    Reflecting,
    LoadingModel,
    Done,
    Failed,
    Stopped,
}

impl AgentState {
    pub fn is_running(self) -> bool {
        matches!(
            self,
            Self::Planning
                | Self::AwaitingConfirm
                | Self::Acting
                | Self::Observing
                | Self::LoadingModel
        )
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub hotkeys: Mutex<Hotkeys>,
    pub mic: Mutex<Option<Capture>>,
    pub run: Mutex<RunScope>,
    pub verifier: Mutex<exec::CancelToken>,
    pub analysis: Mutex<crate::analysis::state::AnalysisState>,
    pub external: Mutex<Option<crate::bridge::identity::AgentIdentity>>,
    pub reflect: Mutex<crate::reflect::State>,
    pub agent_state: Mutex<AgentState>,
    pub lease: Mutex<LeaseBook>,
    pub confirm: Mutex<Option<Pending>>,
    pub steer: Mutex<Vec<String>>,
    #[cfg(feature = "agent-bridge")]
    pub act_turn: tokio::sync::Mutex<()>,
    pub trace: crate::trace::Recorder,
    pub http: reqwest::Client,
    pub capabilities: crate::model::capabilities::SharedCapabilityCache,
}

impl AppState {
    pub fn begin_run(&self) -> RunScope {
        self.verifier.locked().cancel();
        self.reflect.locked().cancel.cancel();
        exec::begin_run(&self.run)
    }

    pub fn stop_current(&self) {
        crate::voice::note_stop();
        self.reflect.locked().cancel.cancel();
        self.verifier.locked().cancel();
        self.analysis.locked().cancel.cancel();
        exec::stop_current(&self.run);
    }

    pub fn begin_verification(&self) -> exec::CancelToken {
        let mut current = self.verifier.locked();
        current.cancel();
        *current = exec::CancelToken::default();
        current.clone()
    }

    #[cfg(feature = "agent-bridge")]
    pub fn current_scope(&self) -> RunScope {
        exec::current_scope(&self.run)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskVerified {
    pub run_id: String,
    pub result: &'static str,
    pub evidence: String,
}

#[derive(Clone, Serialize)]
struct StateChanged {
    state: AgentState,
}

pub fn set_agent_state(app: &AppHandle, state: AgentState) {
    apply_state(app, state, true);
}

pub fn preview_agent_state(app: &AppHandle, state: AgentState) {
    // Animation previews never take execution ownership or change the real state.
    overlay::sync(app, state);
    let _ = app.emit(STATE_CHANGED, StateChanged { state });
}

fn apply_state(app: &AppHandle, state: AgentState, allow_click_through: bool) {
    let app_state = app.state::<AppState>();
    *app_state.agent_state.locked() = state;
    window::set_click_through(app, allow_click_through && state == AgentState::Acting);
    overlay::sync(app, state);
    let _ = app.emit(STATE_CHANGED, StateChanged { state });
}

pub fn agent_state(app: &AppHandle) -> AgentState {
    *app.state::<AppState>().agent_state.locked()
}

pub const STEP_STARTED: &str = "step_started";
pub const STEP_FINISHED: &str = "step_finished";
pub const TASK_DONE: &str = "task_done";
pub const EXTERNAL_AGENT: &str = "external_agent";

pub const PLAN_UPDATED: &str = "plan_updated";

#[derive(Clone, Serialize)]
pub struct PlanUpdated {
    pub goals: Vec<String>,
    pub current: usize,
    pub finished: bool,
}

pub fn emit_plan(app: &AppHandle, goals: &[String], current: usize, finished: bool) {
    let _ = app.emit(
        PLAN_UPDATED,
        PlanUpdated {
            goals: goals.to_vec(),
            current,
            finished,
        },
    );
}

pub const THINKING: &str = "thinking";

#[derive(Clone, Serialize)]
pub struct Thinking {
    pub text: String,
    pub done: bool,
}

pub fn emit_thinking(app: &AppHandle, text: &str, done: bool) {
    let _ = app.emit(
        THINKING,
        Thinking {
            text: text.to_string(),
            done,
        },
    );
}

pub const TYPED: &str = "typed";

#[derive(Clone, Serialize)]
pub struct Typed {
    pub point: Option<Point>,
}

pub const OBSERVE_STARTED: &str = "observe_started";

#[derive(Clone, Serialize)]
pub struct ObserveStarted {
    pub region: Option<[f64; 4]>,
    pub continuous: bool,
}

#[derive(Clone, Serialize)]
pub struct ExternalAgent {
    pub agent: Option<String>,
    pub id: Option<String>,
    pub known: bool,
    pub verified: bool,
    pub declared: Option<String>,
}

pub fn set_external(app: &AppHandle, who: Option<crate::bridge::identity::AgentIdentity>) {
    let event = who
        .as_ref()
        .map_or_else(ExternalAgent::none, ExternalAgent::of);
    *app.state::<AppState>().external.locked() = who;
    let _ = app.emit(EXTERNAL_AGENT, event);
}

impl ExternalAgent {
    pub fn none() -> Self {
        Self {
            agent: None,
            id: None,
            known: false,
            verified: false,
            declared: None,
        }
    }

    pub fn of(who: &crate::bridge::identity::AgentIdentity) -> Self {
        Self {
            agent: Some(who.name.clone()),
            id: Some(who.id.clone()),
            known: who.known,
            verified: who.verified,
            declared: Some(who.declared.clone()),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepStarted {
    pub step: u32,
    pub tool: &'static str,
    pub reason: String,
    pub text: String,
    pub detail: String,
    pub point: Option<Point>,
    pub double: bool,
    pub button: &'static str,
    pub duration_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Serialize)]
pub struct StepFinished {
    pub step: u32,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct TaskDone {
    pub summary: String,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    #[test]
    fn step_started_is_sent_with_the_camel_case_names_the_ui_reads() {
        let started = super::StepStarted {
            step: 1,
            tool: "click",
            reason: String::new(),
            text: String::new(),
            detail: String::new(),
            point: None,
            double: false,
            button: "left",
            duration_ms: 250,
        };
        let json = serde_json::to_value(started).unwrap();
        assert_eq!(json["durationMs"], 250, "the overlay reads durationMs");
        assert!(json.get("duration_ms").is_none());
    }

    use super::*;

    #[test]
    fn only_the_four_working_states_count_as_running() {
        use AgentState::*;
        let table = [
            (Idle, false),
            (Listening, false),
            (Planning, true),
            (AwaitingConfirm, true),
            (Acting, true),
            (Observing, true),
            (Reflecting, false),
            (Done, false),
            (Failed, false),
            (Stopped, false),
        ];
        for (state, running) in table {
            assert_eq!(state.is_running(), running, "{state:?}");
        }
        assert_eq!(
            serde_json::to_string(&Reflecting).unwrap(),
            "\"reflecting\""
        );
    }

    fn idle_state() -> AppState {
        let settings = Settings::default();
        AppState {
            hotkeys: Mutex::new(Hotkeys::from_settings(&settings)),
            settings: Mutex::new(settings),
            mic: Mutex::new(None),
            run: Mutex::new(RunScope::new()),
            verifier: Mutex::new(exec::CancelToken::default()),
            analysis: Default::default(),
            external: Default::default(),
            reflect: Default::default(),
            agent_state: Mutex::new(AgentState::Idle),
            lease: Mutex::new(LeaseBook::default()),
            confirm: Mutex::new(None),
            steer: Default::default(),
            #[cfg(feature = "agent-bridge")]
            act_turn: Default::default(),
            trace: crate::trace::Recorder::noop(),
            http: reqwest::Client::new(),
            capabilities: std::sync::Arc::new(
                crate::model::capabilities::CapabilityCache::default(),
            ),
        }
    }

    #[test]
    fn starting_a_task_cancels_the_background_check_and_the_check_after_it_is_fresh() {
        let state = idle_state();
        let check = state.begin_verification();
        assert!(!check.is_cancelled());
        let run = state.begin_run();
        assert!(
            check.is_cancelled(),
            "a new task cancels the check of the last one"
        );
        assert!(!run.token.is_cancelled(), "and its own token is untouched");
        let next = state.begin_verification();
        assert!(!next.is_cancelled());
        let after = state.begin_verification();
        assert!(
            next.is_cancelled() && !after.is_cancelled(),
            "only the newest check is live"
        );
    }

    #[test]
    fn stop_cancels_the_background_check_even_when_nothing_is_running() {
        let state = idle_state();
        let check = state.begin_verification();
        let scope = exec::current_scope(&state.run);
        state.stop_current();
        assert!(
            check.is_cancelled(),
            "Stop reaches the check from Idle, like it reaches typing"
        );
        assert!(scope.token.is_cancelled());
        assert!(!state.begin_verification().is_cancelled());
    }
}
