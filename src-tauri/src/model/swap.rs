use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::header::HeaderMap;
use serde_json::Value;

static LAST_CALL_AT: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapSignal {
    None,
    Loading { estimated_ms: Option<u64> },
    Swapped { swap_ms: Option<u64> },
}

pub fn detect_swap_signal(headers: &HeaderMap, _status: u16, body: Option<&Value>) -> SwapSignal {
    for key in [
        "x-rune-infer-loading",
        "x-rune-model-loading",
        "x-model-loading",
        "x-loading-model",
    ] {
        if let Some(val) = headers.get(key) {
            let text = val.to_str().unwrap_or("");
            let ms = text.parse::<u64>().ok();
            return SwapSignal::Loading { estimated_ms: ms };
        }
    }

    for key in [
        "x-rune-infer-swapped",
        "x-rune-model-swapped",
        "x-model-swap-ms",
        "x-swap-duration-ms",
    ] {
        if let Some(val) = headers.get(key) {
            let text = val.to_str().unwrap_or("");
            let ms = text.parse::<u64>().ok();
            return SwapSignal::Swapped { swap_ms: ms };
        }
    }

    if let Some(val) = body {
        let s = val.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if s.eq_ignore_ascii_case("loading") || s.eq_ignore_ascii_case("swapping") {
            let ms = val.get("load_ms_estimate").and_then(|v| v.as_u64());
            return SwapSignal::Loading { estimated_ms: ms };
        }
        if let Some(ms) = val.get("swap_ms").and_then(|v| v.as_u64()) {
            return SwapSignal::Swapped { swap_ms: Some(ms) };
        }
    }

    SwapSignal::None
}

pub fn is_idle_or_first_call(idle_threshold: Duration) -> bool {
    let mut guard = match LAST_CALL_AT.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let was_idle = match *guard {
        None => true,
        Some(last) => last.elapsed() >= idle_threshold,
    };
    *guard = Some(Instant::now());
    was_idle
}

pub fn record_call_activity() {
    if let Ok(mut guard) = LAST_CALL_AT.lock() {
        *guard = Some(Instant::now());
    }
}

pub fn reset_call_activity_for_test() {
    if let Ok(mut guard) = LAST_CALL_AT.lock() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};
    use serde_json::json;

    #[test]
    fn detects_loading_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-model-loading", HeaderValue::from_static("12000"));
        let signal = detect_swap_signal(&headers, 200, None);
        assert_eq!(
            signal,
            SwapSignal::Loading {
                estimated_ms: Some(12000)
            }
        );
    }

    #[test]
    fn detects_swapped_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-model-swap-ms", HeaderValue::from_static("8500"));
        let signal = detect_swap_signal(&headers, 200, None);
        assert_eq!(
            signal,
            SwapSignal::Swapped {
                swap_ms: Some(8500)
            }
        );
    }

    #[test]
    fn detects_loading_status_503() {
        let headers = HeaderMap::new();
        let body = json!({
            "status": "loading",
            "load_ms_estimate": 15000
        });
        let signal = detect_swap_signal(&headers, 503, Some(&body));
        assert_eq!(
            signal,
            SwapSignal::Loading {
                estimated_ms: Some(15000)
            }
        );
    }

    #[test]
    fn detects_swapped_body() {
        let headers = HeaderMap::new();
        let body = json!({
            "swap_ms": 7200
        });
        let signal = detect_swap_signal(&headers, 200, Some(&body));
        assert_eq!(
            signal,
            SwapSignal::Swapped {
                swap_ms: Some(7200)
            }
        );
    }

    #[test]
    fn returns_none_when_normal() {
        let headers = HeaderMap::new();
        let body = json!({ "status": "ready" });
        let signal = detect_swap_signal(&headers, 200, Some(&body));
        assert_eq!(signal, SwapSignal::None);
    }

    #[test]
    fn first_call_is_idle_and_immediate_second_is_not() {
        reset_call_activity_for_test();
        assert!(is_idle_or_first_call(Duration::from_secs(10)));
        assert!(!is_idle_or_first_call(Duration::from_secs(10)));
    }
}
