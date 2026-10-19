//! Executing actions on the desktop: mouse, keyboard, launching apps and the Stop token.

pub mod apps;
pub mod caret;
pub mod input;
pub mod uia;

use crate::sync::LockExt;
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
    let mut current = slot.locked();
    current.token.cancel();
    let next = RunScope::new();
    *current = next.clone();
    next
}

/// Cancels the scope in `slot` and nothing else. Works from any state, so Stop interrupts typing at once.
pub fn stop_current(slot: &Mutex<RunScope>) {
    slot.locked().token.cancel();
}

/// The scope a call to Stop would cancel right now.
#[cfg(any(test, feature = "agent-bridge"))]
pub fn current_scope(slot: &Mutex<RunScope>) -> RunScope {
    slot.locked().clone()
}

/// Waits for `lock`, but gives up with `Cancelled` as soon as `cancel` is set (also if it already
/// is). Outside agents' actions take turns through this: a call that is queued behind another one
/// must not outlive Stop.
#[cfg(any(test, feature = "agent-bridge"))]
pub async fn lock_unless_stopped<'a>(lock: &'a tokio::sync::Mutex<()>, cancel: &CancelToken) -> Result<tokio::sync::MutexGuard<'a, ()>, ExecError> {
    cancel.check()?;
    tokio::select! {
        guard = lock.lock() => Ok(guard),
        () = async { while !cancel.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await } } => Err(ExecError::Cancelled),
    }
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

    #[tokio::test]
    async fn turns_are_strictly_sequential() {
        use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        let inside = Arc::new(AtomicUsize::new(0));
        let overlapped = Arc::new(AtomicBool::new(false));
        let mut turns = Vec::new();
        for _ in 0..4 {
            let (lock, inside, overlapped) = (lock.clone(), inside.clone(), overlapped.clone());
            turns.push(tokio::spawn(async move {
                let _turn = lock_unless_stopped(&lock, &CancelToken::default()).await.expect("nobody pressed Stop");
                if inside.fetch_add(1, SeqCst) != 0 {
                    overlapped.store(true, SeqCst);
                }
                tokio::time::sleep(Duration::from_millis(30)).await;
                inside.fetch_sub(1, SeqCst);
            }));
        }
        for turn in turns {
            turn.await.unwrap();
        }
        assert!(!overlapped.load(SeqCst), "two turns ran at the same time");
    }

    #[tokio::test]
    async fn a_waiter_is_released_by_stop() {
        let lock = tokio::sync::Mutex::new(());
        let first = lock_unless_stopped(&lock, &CancelToken::default()).await.unwrap();
        let stop = CancelToken::default();
        let stopper = stop.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(60)).await;
            stopper.cancel();
        });
        let started = Instant::now();
        let second = lock_unless_stopped(&lock, &stop).await;
        assert!(matches!(second, Err(ExecError::Cancelled)));
        assert!(started.elapsed() < Duration::from_secs(2), "released by Stop, not by the first holder");
        drop(first);
        // And a token that is already cancelled never takes the lock, even when it is free.
        assert!(matches!(lock_unless_stopped(&lock, &stop).await, Err(ExecError::Cancelled)));
    }
}
