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
/// The independent check of a finished task came back: `{ runId, result, evidence }`.
pub const TASK_VERIFIED: &str = "task_verified";
/// The screen analysis changed (see `analysis`): the whole snapshot.
pub const ANALYSIS_UPDATED: &str = "analysis_updated";
pub const SUMMON: &str = "summon";
pub const OPEN_SETTINGS: &str = "open_settings";
pub const OPEN_ARTIFACTS: &str = "open_artifacts";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Listening,
    Planning,
    AwaitingConfirm,
    Acting,
    Observing,
    /// After a task: looking back at what happened (nothing produces it yet). Not a running state.
    Reflecting,
    Done,
    Failed,
    Stopped,
}

impl AgentState {
    /// Whether a task is under way: a new one is refused and Stop has something to stop.
    /// `Reflecting` is not running: the task is over and the agent is only looking back.
    pub fn is_running(self) -> bool {
        matches!(
            self,
            Self::Planning | Self::AwaitingConfirm | Self::Acting | Self::Observing
        )
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub hotkeys: Mutex<Hotkeys>,
    /// Present while the microphone is open.
    pub mic: Mutex<Option<Capture>>,
    /// The current run's cancellation scope. Stop cancels it; the next run replaces it (`begin_run`).
    pub run: Mutex<RunScope>,
    /// Cancels the background check of the last finished task (see `agent::verify`): starting a task
    /// or pressing Stop cancels it first, so the check never competes with the user.
    pub verifier: Mutex<exec::CancelToken>,
    /// The screen analysis window's state (see `analysis`).
    pub analysis: Mutex<crate::analysis::state::AnalysisState>,
    /// Who is in control from outside right now (see `bridge::identity`), for a window that opens late.
    pub external: Mutex<Option<crate::bridge::identity::AgentIdentity>>,
    /// The idle reflection shell (see `reflect`).
    pub reflect: Mutex<crate::reflect::State>,
    pub agent_state: Mutex<AgentState>,
    /// Who may drive the PC from outside (see `bridge::lease`).
    pub lease: Mutex<LeaseBook>,
    /// The question waiting for the user's yes or no, if any (see `safety::gate`).
    pub confirm: Mutex<Option<Pending>>,
    /// Things the user said while the agent was working; read before its next step.
    pub steer: Mutex<Vec<String>>,
    /// Held for a whole outside-agent action (gate, announce, execute, settle), so two parallel
    /// calls take turns instead of driving the mouse and the confirm slot at once.
    #[cfg(feature = "agent-bridge")]
    pub act_turn: tokio::sync::Mutex<()>,
    /// The flight recorder (see `trace`); it records nothing if its database could not be opened.
    pub trace: crate::trace::Recorder,
    pub http: reqwest::Client,
}

impl AppState {
    /// Cancels the previous run's scope and returns a fresh one for the run that is starting.
    pub fn begin_run(&self) -> RunScope {
        // First: a check left over from the last task must not run next to the new one.
        self.verifier.locked().cancel();
        // A task, an outside agent or a debug command pre-empts the idle reflection (within its next 10 ms check).
        self.reflect.locked().cancel.cancel();
        exec::begin_run(&self.run)
    }

    /// Cancels the current run, whatever the agent state (even Idle: typing from debug commands relies on it).
    pub fn stop_current(&self) {
        crate::voice::note_stop();
        self.reflect.locked().cancel.cancel();
        self.verifier.locked().cancel();
        self.analysis.locked().cancel.cancel();
        exec::stop_current(&self.run);
    }

    /// A fresh token for a background check about to start; the previous check, if any, is cancelled.
    pub fn begin_verification(&self) -> exec::CancelToken {
        let mut current = self.verifier.locked();
        current.cancel();
        *current = exec::CancelToken::default();
        current.clone()
    }

    /// The scope Stop would cancel right now.
    #[cfg(feature = "agent-bridge")]
    pub fn current_scope(&self) -> RunScope {
        exec::current_scope(&self.run)
    }
}

/// The payload of `TASK_VERIFIED`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskVerified {
    pub run_id: String,
    /// `verified`, `contradicted` or `unsure`.
    pub result: &'static str,
    pub evidence: String,
}

#[derive(Clone, Serialize)]
struct StateChanged {
    state: AgentState,
}

/// Single place the agent state changes. Keeps the UI event and the window's
/// click-through behaviour in sync: the pill must never block a click while acting.
pub fn set_agent_state(app: &AppHandle, state: AgentState) {
    apply_state(app, state, true);
}

/// Like `set_agent_state`, but never turns click-through on. For the dev state switcher, which
/// forces states by hand and must not trap the user in a window that ignores the mouse.
#[cfg(debug_assertions)]
pub fn preview_agent_state(app: &AppHandle, state: AgentState) {
    apply_state(app, state, false);
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
/// Who is driving from outside: `{ agent: "Claude" }`, or `{ agent: null }` when nobody is.
pub const EXTERNAL_AGENT: &str = "external_agent";

/// The plan for the task: its goals and which one is being worked on.
pub const PLAN_UPDATED: &str = "plan_updated";

#[derive(Clone, Serialize)]
pub struct PlanUpdated {
    pub goals: Vec<String>,
    /// Index of the goal being worked on. Equal to the number of goals once everything is done.
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

/// What the model is thinking, as it streams in (only when "Show thinking" is on).
pub const THINKING: &str = "thinking";

#[derive(Clone, Serialize)]
pub struct Thinking {
    /// All of it so far.
    pub text: String,
    /// The model has finished thinking.
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

/// Text was just typed: the overlay marks the spot, like a click does.
pub const TYPED: &str = "typed";

#[derive(Clone, Serialize)]
pub struct Typed {
    /// Where the text went (the caret, or the mouse if the caret cannot be found), in physical desktop pixels.
    pub point: Option<Point>,
}

/// The agent is taking a screenshot: the overlay flashes the captured area.
pub const OBSERVE_STARTED: &str = "observe_started";

#[derive(Clone, Serialize)]
pub struct ObserveStarted {
    /// The zoomed part, as `[x0, y0, x1, y1]` in model units (0 to 1000), or `None` for the whole screen.
    pub region: Option<[f64; 4]>,
    /// A recording rather than a single screenshot: the overlay keeps the capture wave going.
    pub continuous: bool,
}

#[derive(Clone, Serialize)]
pub struct ExternalAgent {
    /// The display name (the old field the overlay already reads); `None` when nobody is in control.
    pub agent: Option<String>,
    /// The brand id (`claude`, `antigravity`, ..., `external`); `None` when nobody is in control.
    pub id: Option<String>,
    pub known: bool,
    /// The registration (`mcp --agent <id>`) named the agent, not only the agent itself.
    pub verified: bool,
    /// What the agent called itself, for a tooltip.
    pub declared: Option<String>,
}

/// Records who is in control from outside (`None`: nobody) and tells every window.
pub fn set_external(app: &AppHandle, who: Option<crate::bridge::identity::AgentIdentity>) {
    let event = who
        .as_ref()
        .map_or_else(ExternalAgent::none, ExternalAgent::of);
    *app.state::<AppState>().external.locked() = who;
    let _ = app.emit(EXTERNAL_AGENT, event);
}

impl ExternalAgent {
    /// Nobody is in control.
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
    /// What the action carries, for the on-screen caption: the typed text, the keys, the app.
    pub detail: String,
    /// Where a click lands, in physical desktop pixels. The control overlay draws a ripple there.
    pub point: Option<Point>,
    pub double: bool,
    pub button: &'static str,
    /// How long the action takes to carry out: the cursor's glide for a click, the typing time for
    /// text. The overlay lands the click ripple when the cursor arrives and types the caption in step.
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

    /// An `AppState` with nothing running, for the tests of how a new task and Stop reach the checks.
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
        // The next check gets a fresh token; the one before it is cancelled by then.
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
        // A check started after the Stop is not affected by it.
        assert!(!state.begin_verification().is_cancelled());
    }
}
