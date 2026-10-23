#![cfg_attr(not(feature = "agent-bridge"), allow(dead_code))]

use std::time::{Duration, Instant};

pub const IDLE_LIMIT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LeaseError {
    #[error(
        "another agent is already in control: wait 5 seconds and retry up to three times, then tell the user"
    )]
    Busy,
    #[error("you have not taken control yet: call rune_begin first")]
    NotBegun,
    #[error(
        "the user pressed Stop: stop working and report; if they request a restart, rune_begin asks them to approve a new session"
    )]
    StoppedByUser,
    #[error("control ended because there were no calls for a while")]
    Expired,
}

impl LeaseError {
    pub fn code(self) -> &'static str {
        match self {
            LeaseError::Busy => "busy",
            LeaseError::NotBegun => "not_begun",
            LeaseError::StoppedByUser => "stopped_by_user",
            LeaseError::Expired => "lease_expired",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lease {
    pub agent: String,
    pub task: String,
    pub step: u32,
    pub plan: Vec<String>,
    pub current_goal: usize,
    last_call: Instant,
}

#[derive(Debug, Default)]
pub struct LeaseBook {
    current: Option<Lease>,
    revoked: bool,
    restart_pending: Option<u64>,
    revision: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BeginRequest {
    Begun,
    ConfirmRestart(u64),
}

impl LeaseBook {
    pub fn is_held(&self) -> bool {
        self.current.is_some() || self.restart_pending.is_some()
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked
    }

    pub fn agent(&self) -> Option<&str> {
        self.current.as_ref().map(|lease| lease.agent.as_str())
    }

    pub fn current(&self) -> Option<&Lease> {
        self.current.as_ref()
    }

    pub fn current_mut(&mut self) -> Option<&mut Lease> {
        self.current.as_mut()
    }

    pub fn begin(&mut self, agent: &str, task: &str, now: Instant) -> Result<(), LeaseError> {
        if self.revoked {
            return Err(LeaseError::StoppedByUser);
        }
        self.expire_if_idle(now);
        if self.current.is_some() {
            return Err(LeaseError::Busy);
        }
        self.current = Some(Lease {
            agent: agent.to_string(),
            task: task.to_string(),
            step: 0,
            plan: Vec::new(),
            current_goal: 0,
            last_call: now,
        });
        Ok(())
    }

    pub fn request_begin(
        &mut self,
        agent: &str,
        task: &str,
        now: Instant,
    ) -> Result<BeginRequest, LeaseError> {
        if self.restart_pending.is_some() {
            return Err(LeaseError::Busy);
        }
        if self.revoked {
            self.revision = self.revision.wrapping_add(1);
            self.restart_pending = Some(self.revision);
            return Ok(BeginRequest::ConfirmRestart(self.revision));
        }
        self.begin(agent, task, now)?;
        Ok(BeginRequest::Begun)
    }

    /// Approval belongs only to this retry. Stop/settings changes invalidate it.
    pub fn resolve_restart(
        &mut self,
        ticket: u64,
        approved: bool,
        agent: &str,
        task: &str,
        now: Instant,
    ) -> Result<(), LeaseError> {
        if self.restart_pending != Some(ticket) || !self.revoked {
            return Err(LeaseError::StoppedByUser);
        }
        self.restart_pending = None;
        if !approved {
            return Err(LeaseError::StoppedByUser);
        }
        self.rearm();
        self.begin(agent, task, now)
    }

    pub fn touch(&mut self, now: Instant) -> Result<&mut Lease, LeaseError> {
        if self.revoked {
            return Err(LeaseError::StoppedByUser);
        }
        if self.expire_if_idle(now) {
            return Err(LeaseError::Expired);
        }
        let lease = self.current.as_mut().ok_or(LeaseError::NotBegun)?;
        lease.last_call = now;
        Ok(lease)
    }

    pub fn end(&mut self) {
        self.current = None;
    }

    pub fn revoke(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        let held = self.current.take().is_some();
        let pending = self.restart_pending.take().is_some();
        if held || pending {
            self.revoked = true;
        }
    }

    pub fn rearm(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.restart_pending = None;
        self.revoked = false;
    }

    pub fn expire_if_idle(&mut self, now: Instant) -> bool {
        let idle = self
            .current
            .as_ref()
            .is_some_and(|lease| now.duration_since(lease.last_call) >= IDLE_LIMIT);
        if idle {
            self.current = None;
        }
        idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn only_one_agent_may_hold_control() {
        let mut book = LeaseBook::default();
        let now = t0();
        assert_eq!(book.begin("claude", "task", now), Ok(()));
        assert_eq!(book.begin("other", "task", now), Err(LeaseError::Busy));
        book.end();
        assert_eq!(book.begin("other", "task", now), Ok(()));
    }

    #[test]
    fn acting_without_begin_is_refused() {
        let mut book = LeaseBook::default();
        assert_eq!(book.touch(t0()).err(), Some(LeaseError::NotBegun));
    }

    #[test]
    fn stop_revokes_and_only_the_user_can_rearm() {
        let mut book = LeaseBook::default();
        let now = t0();
        book.begin("claude", "task", now).unwrap();
        book.revoke();
        assert_eq!(book.touch(now).err(), Some(LeaseError::StoppedByUser));
        assert_eq!(
            book.begin("claude", "task", now),
            Err(LeaseError::StoppedByUser)
        );
        book.rearm();
        assert_eq!(book.begin("claude", "task", now), Ok(()));
    }

    #[test]
    fn stop_with_nobody_in_control_does_not_lock_anyone_out() {
        let mut book = LeaseBook::default();
        book.revoke();
        assert_eq!(book.begin("claude", "task", t0()), Ok(()));
    }

    fn stopped_book() -> LeaseBook {
        let mut book = LeaseBook::default();
        book.begin("agent", "old task", t0()).unwrap();
        book.current_mut().unwrap().step = 7;
        book.current_mut().unwrap().plan = vec!["old goal".into()];
        book.revoke();
        book
    }

    #[test]
    fn retry_after_stop_waits_for_approval_then_starts_a_fresh_lease() {
        let mut book = stopped_book();
        let BeginRequest::ConfirmRestart(ticket) =
            book.request_begin("agent", "retry task", t0()).unwrap()
        else {
            panic!("must ask user");
        };
        assert!(
            book.is_held(),
            "reserve control while asking so another task cannot start"
        );
        assert_eq!(book.touch(t0()).err(), Some(LeaseError::StoppedByUser));
        assert!(
            book.current().is_none(),
            "no resumed actions while awaiting consent"
        );
        book.resolve_restart(ticket, true, "agent", "retry task", t0())
            .unwrap();
        let lease = book.touch(t0()).unwrap();
        assert_eq!(lease.task, "retry task");
        assert_eq!(lease.step, 0);
        assert!(lease.plan.is_empty());
    }

    #[test]
    fn decline_or_timeout_keeps_stop_latched_and_concurrent_retries_are_busy() {
        let mut book = stopped_book();
        let BeginRequest::ConfirmRestart(ticket) =
            book.request_begin("agent", "retry", t0()).unwrap()
        else {
            panic!();
        };
        assert_eq!(
            book.request_begin("other", "another retry", t0()),
            Err(LeaseError::Busy)
        );
        assert_eq!(
            book.resolve_restart(ticket, false, "agent", "retry", t0()),
            Err(LeaseError::StoppedByUser)
        );
        assert!(book.is_revoked());
        assert!(!book.is_held());
        assert_eq!(book.touch(t0()).err(), Some(LeaseError::StoppedByUser));
        assert!(matches!(
            book.request_begin("agent", "retry", t0()),
            Ok(BeginRequest::ConfirmRestart(_))
        ));
    }

    #[test]
    fn stop_again_or_settings_rearm_invalidates_an_old_approval() {
        for settings_rearm in [false, true] {
            let mut book = stopped_book();
            let BeginRequest::ConfirmRestart(ticket) =
                book.request_begin("agent", "retry", t0()).unwrap()
            else {
                panic!();
            };
            if settings_rearm {
                book.rearm();
            } else {
                book.revoke();
            }
            assert_eq!(
                book.resolve_restart(ticket, true, "agent", "retry", t0()),
                Err(LeaseError::StoppedByUser)
            );
            assert!(book.current().is_none());
        }
        let mut book = stopped_book();
        let BeginRequest::ConfirmRestart(old) = book.request_begin("agent", "retry", t0()).unwrap()
        else {
            panic!();
        };
        book.revoke();
        let BeginRequest::ConfirmRestart(new) = book.request_begin("agent", "retry", t0()).unwrap()
        else {
            panic!();
        };
        assert_ne!(old, new);
        assert!(
            book.resolve_restart(old, true, "agent", "retry", t0())
                .is_err()
        );
        book.resolve_restart(new, true, "agent", "retry", t0())
            .unwrap();
    }

    #[test]
    fn a_silent_agent_loses_control_after_the_idle_limit() {
        let mut book = LeaseBook::default();
        let start = t0();
        book.begin("claude", "task", start).unwrap();
        assert!(
            book.touch(start + IDLE_LIMIT - Duration::from_secs(1))
                .is_ok()
        );
        let late = start + IDLE_LIMIT - Duration::from_secs(1) + IDLE_LIMIT;
        assert_eq!(book.touch(late).err(), Some(LeaseError::Expired));
        assert_eq!(book.begin("next", "task", late), Ok(()));
    }

    #[test]
    fn every_error_has_a_code_from_the_guide() {
        for error in [
            LeaseError::Busy,
            LeaseError::NotBegun,
            LeaseError::StoppedByUser,
            LeaseError::Expired,
        ] {
            assert!(
                ["busy", "not_begun", "stopped_by_user", "lease_expired"].contains(&error.code())
            );
        }
    }
}
