//! Executing actions on the desktop: mouse, keyboard, launching apps and the Stop token.

pub mod apps;
pub mod caret;
pub mod input;
pub mod uia;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// Set by the Stop hotkey, the tray's Stop and the Stop button. Every executor action checks it
/// between its steps, so Stop interrupts typing mid-sentence instead of after it.
/// A token is never un-cancelled: each run gets its own (see `RunScope`).
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Fails with `Cancelled` if Stop was pressed.
    pub fn check(&self) -> Result<(), ExecError> {
        if self.is_cancelled() { Err(ExecError::Cancelled) } else { Ok(()) }
    }

    /// Sleeps in short slices so Stop is noticed within milliseconds, not at the end.
    pub fn sleep(&self, total: Duration) -> Result<(), ExecError> {
        let deadline = Instant::now() + total;
        loop {
            self.check()?;
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(());
            }
            thread::sleep(left.min(Duration::from_millis(10)));
        }
    }
}

static NEXT_RUN_ID: AtomicU64 = AtomicU64::new(1);

/// One run's cancellation scope: the built-in agent's task, an outside agent's lease, or a debug
/// command. Stop cancels the current scope; starting the next run cancels the previous one and
/// installs a fresh scope, so a run that is still winding down can never be revived by its successor.
#[derive(Clone)]
pub struct RunScope {
    pub id: u64,
    pub token: CancelToken,
}

impl RunScope {
    pub fn new() -> Self {
        Self { id: NEXT_RUN_ID.fetch_add(1, Ordering::SeqCst), token: CancelToken::default() }
    }
}

impl Default for RunScope {
    fn default() -> Self {
        Self::new()
    }
}

/// Cancels the scope in `slot` (the run that is ending or being replaced), installs a fresh one and returns it.
pub fn begin_run(slot: &Mutex<RunScope>) -> RunScope {
    let mut current = slot.lock().expect("run scope lock poisoned");
    current.token.cancel();
    let next = RunScope::new();
    *current = next.clone();
    next
}

/// Cancels the scope in `slot` and nothing else. Works from any state, so Stop interrupts typing at once.
pub fn stop_current(slot: &Mutex<RunScope>) {
    slot.lock().expect("run scope lock poisoned").token.cancel();
}

/// The scope a call to Stop would cancel right now.
#[cfg(any(test, feature = "agent-bridge"))]
pub fn current_scope(slot: &Mutex<RunScope>) -> RunScope {
    slot.lock().expect("run scope lock poisoned").clone()
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("stopped")]
    Cancelled,
    #[error("input failed: {0}")]
    Input(String),
    #[error("unknown key '{0}'")]
    UnknownKey(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_run_b_cancels_run_a_for_good() {
        let slot = Mutex::new(RunScope::new());
        let a = begin_run(&slot);
        assert!(!a.token.is_cancelled());
        let b = begin_run(&slot);
        assert!(a.token.is_cancelled(), "the previous run is cancelled");
        assert!(!b.token.is_cancelled(), "the new run starts clean");
        let c = begin_run(&slot);
        assert!(a.token.is_cancelled() && b.token.is_cancelled(), "starting C does not revive A");
        assert!(!c.token.is_cancelled());
    }

    #[test]
    fn stop_cancels_only_the_current_scope() {
        let slot = Mutex::new(RunScope::new());
        let a = begin_run(&slot);
        let b = begin_run(&slot);
        let before_stop = current_scope(&slot);
        assert_eq!(before_stop.id, b.id);
        stop_current(&slot);
        assert!(b.token.is_cancelled());
        assert!(current_scope(&slot).token.is_cancelled());
        // A scope installed afterwards is unaffected by the earlier Stop.
        let c = begin_run(&slot);
        assert!(!c.token.is_cancelled());
        assert!(a.token.is_cancelled(), "A was already cancelled by B's start");
    }

    #[test]
    fn scope_ids_strictly_increase() {
        let slot = Mutex::new(RunScope::new());
        let mut last = current_scope(&slot).id;
        for _ in 0..5 {
            let next = begin_run(&slot).id;
            assert!(next > last);
            last = next;
        }
    }
}
