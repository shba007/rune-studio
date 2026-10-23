//! A bounded, in-memory inbox for user-initiated screen requests.
use serde::Serialize;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(120);
pub const UPDATED: &str = "screen_request_updated";

pub fn publish(app: &tauri::AppHandle) {
    use crate::sync::LockExt;
    use tauri::{Emitter, Manager};
    let snapshot = app
        .state::<crate::events::AppState>()
        .screen_requests
        .locked()
        .snapshot(Instant::now());
    let _ = app.emit(UPDATED, snapshot);
}

#[cfg(feature = "agent-bridge")]
pub async fn wait(
    inbox: &std::sync::Mutex<Inbox>,
    epoch: u64,
    seconds: u64,
) -> Result<Option<Request>, &'static str> {
    use crate::sync::LockExt;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        if let Some(request) = inbox.locked().poll(epoch, Instant::now())? {
            return Ok(Some(request));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Idle,
    Queued,
    Delivered,
    Captured,
    Cancelled,
    Expired,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: Option<u64>,
    pub status: Status,
    pub waiting: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub id: u64,
    pub question: String,
    #[serde(skip)]
    created: Instant,
    #[serde(skip)]
    status: Status,
}
#[derive(Default)]
pub struct Inbox {
    next: u64,
    epoch: u64,
    waiter: Option<u64>,
    request: Option<Request>,
}
impl Inbox {
    pub fn snapshot(&mut self, now: Instant) -> Snapshot {
        if let Some(request) = &mut self.request
            && matches!(request.status, Status::Queued | Status::Delivered)
            && now.saturating_duration_since(request.created) >= TTL
        {
            request.status = Status::Expired;
        }
        Snapshot {
            id: self.request.as_ref().map(|r| r.id),
            status: self.request.as_ref().map_or(Status::Idle, |r| r.status),
            waiting: self.waiter.is_some(),
        }
    }
    pub fn enqueue(&mut self, question: String, now: Instant) -> Snapshot {
        let current = self.snapshot(now);
        if matches!(current.status, Status::Queued | Status::Delivered) {
            return current;
        }
        self.next += 1;
        self.request = Some(Request {
            id: self.next,
            question,
            created: now,
            status: Status::Queued,
        });
        self.snapshot(now)
    }
    pub fn start_wait(&mut self) -> Result<(u64, u64), &'static str> {
        if self.waiter.is_some() {
            return Err("Another host call is already waiting for a screen request");
        }
        self.next += 1;
        self.waiter = Some(self.next);
        Ok((self.next, self.epoch))
    }
    pub fn end_wait(&mut self, ticket: u64) {
        if self.waiter == Some(ticket) {
            self.waiter = None;
        }
    }
    pub fn poll(&mut self, epoch: u64, now: Instant) -> Result<Option<Request>, &'static str> {
        if self.epoch != epoch {
            return Err(
                "The user stopped or changed mode. Do not restart or wait again unless asked.",
            );
        }
        self.snapshot(now);
        Ok(self
            .request
            .as_mut()
            .filter(|r| matches!(r.status, Status::Queued | Status::Delivered))
            .map(|r| {
                r.status = Status::Delivered;
                r.clone()
            }))
    }
    pub fn question(&mut self, id: u64, now: Instant) -> Result<String, &'static str> {
        self.snapshot(now);
        self.request
            .as_ref()
            .filter(|r| r.id == id && matches!(r.status, Status::Queued | Status::Delivered))
            .map(|r| r.question.clone())
            .ok_or("This screen request was cancelled, expired or already captured")
    }
    pub fn captured(&mut self, id: u64, now: Instant) -> Result<(), &'static str> {
        self.question(id, now)?;
        self.request.as_mut().unwrap().status = Status::Captured;
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.epoch += 1;
        self.waiter = None;
        if let Some(request) = &mut self.request
            && matches!(request.status, Status::Queued | Status::Delivered)
        {
            request.status = Status::Cancelled;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "agent-bridge")]
    #[tokio::test]
    async fn a_real_wait_wakes_on_a_click_and_stop_and_can_time_out() {
        use crate::sync::LockExt;
        let inbox = std::sync::Mutex::new(Inbox::default());
        let (ticket, epoch) = inbox.locked().start_wait().unwrap();
        assert!(wait(&inbox, epoch, 0).await.unwrap().is_none());
        let (result, ()) = tokio::join!(wait(&inbox, epoch, 1), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            inbox
                .locked()
                .enqueue("Explain the screen".into(), Instant::now());
        });
        let request = result.unwrap().unwrap();
        assert_eq!(request.question, "Explain the screen");
        inbox.locked().captured(request.id, Instant::now()).unwrap();
        let (result, ()) = tokio::join!(wait(&inbox, epoch, 1), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            inbox.locked().cancel();
        });
        assert!(result.is_err());
        inbox.locked().end_wait(ticket);
    }
    #[test]
    fn click_wakes_a_waiting_host_and_repeated_clicks_coalesce() {
        let mut inbox = Inbox::default();
        let now = Instant::now();
        let (ticket, epoch) = inbox.start_wait().unwrap();
        assert!(inbox.start_wait().is_err());
        assert!(inbox.poll(epoch, now).unwrap().is_none());
        let id = inbox.enqueue("Describe".into(), now).id.unwrap();
        assert_eq!(inbox.enqueue("Duplicate".into(), now).id, Some(id));
        let request = inbox.poll(epoch, now).unwrap().unwrap();
        assert_eq!((request.id, request.question.as_str()), (id, "Describe"));
        inbox.end_wait(ticket);
        assert!(!inbox.snapshot(now).waiting);
        inbox.captured(id, now).unwrap();
        assert_eq!(inbox.snapshot(now).status, Status::Captured);
        assert!(inbox.poll(epoch, now).unwrap().is_none());
    }
    #[test]
    fn stop_invalidates_delivery_and_old_waiter_cleanup_cannot_clear_a_new_waiter() {
        let mut inbox = Inbox::default();
        let now = Instant::now();
        let (old, epoch) = inbox.start_wait().unwrap();
        let id = inbox.enqueue("Describe".into(), now).id.unwrap();
        inbox.cancel();
        assert!(inbox.poll(epoch, now).is_err());
        assert!(inbox.captured(id, now).is_err());
        assert_eq!(inbox.snapshot(now).status, Status::Cancelled);
        let (new, _) = inbox.start_wait().unwrap();
        inbox.end_wait(old);
        assert!(inbox.snapshot(now).waiting);
        inbox.end_wait(new);
        assert!(!inbox.snapshot(now).waiting);
    }
    #[test]
    fn requests_expire_and_are_retryable_until_successfully_captured() {
        let mut inbox = Inbox::default();
        let now = Instant::now();
        let (_, epoch) = inbox.start_wait().unwrap();
        let id = inbox.enqueue("Describe".into(), now).id.unwrap();
        assert_eq!(inbox.poll(epoch, now).unwrap().unwrap().id, id);
        assert_eq!(inbox.poll(epoch, now).unwrap().unwrap().id, id);
        assert!(inbox.poll(epoch, now + TTL).unwrap().is_none());
        assert!(inbox.captured(id, now + TTL).is_err());
        assert_eq!(inbox.snapshot(now + TTL).status, Status::Expired);
        assert_ne!(inbox.enqueue("New".into(), now + TTL).id, Some(id));
    }
}
