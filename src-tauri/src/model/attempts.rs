#[cfg(test)]
use std::sync::Mutex;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Attempt {
    pub attempt: u32,
    pub wait_before_ms: u64,
    pub started_utc: String,
    pub duration_ms: u64,
    pub status: Option<u16>,
    pub outcome: &'static str,
    pub error: Option<String>,
    pub request: Value,
    pub response: Value,
    pub model: String,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

pub trait AttemptSink: Send + Sync {
    fn record(&self, attempt: Attempt);
}

#[cfg(test)]
#[derive(Default)]
pub struct Collect(pub Mutex<Vec<Attempt>>);

#[cfg(test)]
impl AttemptSink for Collect {
    fn record(&self, attempt: Attempt) {
        if let Ok(mut list) = self.0.lock() {
            list.push(attempt);
        }
    }
}

pub struct Guard<'a> {
    pub sink: Option<&'a dyn AttemptSink>,
    pub attempt: Option<Attempt>,
    pub started: std::time::Instant,
}

impl Guard<'_> {
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
