//! What a model call reports about every attempt it makes, so the flight recorder can keep the retries
//! (which `call_retrying` used to make invisibly). This module knows nothing about the trace: the
//! client only calls an [`AttemptSink`], and the app plugs the recorder in behind it.

use std::sync::Mutex;

use serde_json::Value;

/// One try at one request.
#[derive(Debug, Clone)]
pub struct Attempt {
    /// 1-based.
    pub attempt: u32,
    /// The backoff that preceded this attempt.
    pub wait_before_ms: u64,
    pub started_utc: String,
    pub duration_ms: u64,
    pub status: Option<u16>,
    /// `ok`, `retried` (failed, and another attempt follows), `failed`, `timeout` or `cancelled`.
    pub outcome: &'static str,
    pub error: Option<String>,
    /// The request body exactly as sent (not yet redacted: the sink does that).
    pub request: Value,
    /// The reply as received: the whole JSON, the text of an error, or what a stream collected.
    pub response: Value,
    pub model: String,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

/// Where attempts go.
pub trait AttemptSink: Send + Sync {
    fn record(&self, attempt: Attempt);
}

/// Collects attempts in memory (tests, and anything that wants to look at them afterwards).
#[derive(Default)]
pub struct Collect(pub Mutex<Vec<Attempt>>);

impl AttemptSink for Collect {
    fn record(&self, attempt: Attempt) {
        if let Ok(mut list) = self.0.lock() {
            list.push(attempt);
        }
    }
}

/// Reports `cancelled` for an attempt whose future is dropped before it finishes (Stop, a timeout
/// around the call): the drop is the only moment anyone can tell.
pub struct Guard<'a> {
    pub sink: Option<&'a dyn AttemptSink>,
    pub attempt: Option<Attempt>,
    pub started: std::time::Instant,
}

impl Guard<'_> {
    /// The attempt finished; nothing is reported on drop.
    pub fn finish(&mut self) -> Option<Attempt> {
        self.attempt.take()
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let (Some(sink), Some(mut attempt)) = (self.sink, self.attempt.take()) {
            attempt.outcome = "cancelled";
            attempt.duration_ms = self.started.elapsed().as_millis() as u64;
            sink.record(attempt);
        }
    }
}
