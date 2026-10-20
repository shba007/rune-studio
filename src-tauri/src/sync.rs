//! Locking that cannot take Stop down.
//!
//! A `std::sync::Mutex` is poisoned when a thread panics while holding it, and `.lock().unwrap()`
//! then panics on every later call, including `commands::stop`. None of the state behind our locks
//! is left half-written in a way that matters more than being able to stop, so a poisoned lock is
//! logged once and used anyway.

use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering},
};

static WARNED: AtomicBool = AtomicBool::new(false);

pub trait LockExt<T> {
    /// Locks, and on a poisoned lock logs once at error level and returns the data regardless.
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            if !WARNED.swap(true, Ordering::Relaxed) {
                log::error!("a lock was poisoned by a panic elsewhere; carrying on with its data");
            }
            poisoned.into_inner()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn a_poisoned_lock_still_gives_its_data() {
        let lock = Arc::new(Mutex::new(41));
        let held = lock.clone();
        let panicked = std::thread::spawn(move || {
            let mut value = held.lock().unwrap();
            *value += 1;
            panic!("poison it");
        })
        .join();
        assert!(panicked.is_err());
        assert!(lock.is_poisoned());
        assert_eq!(*lock.locked(), 42);
        *lock.locked() = 7;
        assert_eq!(*lock.locked(), 7);
    }
}
