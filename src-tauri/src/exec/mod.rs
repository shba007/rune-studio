//! Executing actions on the desktop: mouse, keyboard, launching apps and the Stop token.

pub mod apps;
pub mod input;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

/// Set by the Stop hotkey, the tray's Stop and the Stop button. Every executor action checks it
/// between its steps, so Stop interrupts typing mid-sentence instead of after it.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Called when a new run starts, so an old Stop does not cancel it.
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
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

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("stopped")]
    Cancelled,
    #[error("input failed: {0}")]
    Input(String),
    #[error("unknown key '{0}'")]
    UnknownKey(String),
}
