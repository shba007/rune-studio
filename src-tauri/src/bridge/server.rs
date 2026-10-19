//! The local door: an HTTP endpoint that speaks MCP (JSON-RPC) to one agent at a time.
//!
//! It listens on 127.0.0.1 only. Every request needs the bearer token, a `Host` of this machine
//! (blocks DNS rebinding) and no browser `Origin` (blocks web pages). The Settings switch must
//! be on. Nothing here decides what the agent may do; that is the lease, the safety gate and Stop.

use crate::sync::LockExt;
use std::time::Duration;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};

use crate::events::{self, AgentState, AppState, ExternalAgent};

use super::{mcp, token};

pub const PORT: u16 = 47600;
/// Requests are small JSON; anything bigger is refused.
const MAX_BODY_BYTES: usize = 256 * 1024;

pub fn url() -> String {
    format!("http://127.0.0.1:{PORT}/mcp")
}

/// Starts the listener and the idle watcher on the app's async runtime.
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let router = Router::new()
            .route("/mcp", post(post_mcp).get(not_allowed).delete(not_allowed))
            .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
            .with_state(handle.clone());
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(listener) => listener,
            Err(err) => {
                log::warn!("agent bridge: could not listen on 127.0.0.1:{PORT}: {err}");
                return;
            }
        };
        log::info!("agent bridge: listening on {}", url());
        tauri::async_runtime::spawn(watch_for_idle(handle));
        if let Err(err) = axum::serve(listener, router).await {
            log::warn!("agent bridge: server stopped: {err}");
        }
    });
}

/// Takes control back from an agent that has gone silent.
async fn watch_for_idle(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let expired = app.state::<AppState>().lease.locked().expire_if_idle(std::time::Instant::now());
        if expired {
            log::info!("agent bridge: the agent was silent too long, control released");
            let _ = app.emit(events::EXTERNAL_AGENT, ExternalAgent { agent: None });
            if matches!(events::agent_state(&app), AgentState::Acting | AgentState::Observing) {
                events::set_agent_state(&app, AgentState::Idle);
            }
        }
    }
}

/// Whether the user allows outside agents. Read from the settings file on every request, so the
/// switch in Settings and `rune-studio-cli enable` / `disable` take effect at once. Turning it back on,
/// or running `rune-studio-cli enable` again, also re-arms an agent that Stop locked out.
fn switch_is_on(app: &AppHandle) -> bool {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    static WAS_ON: AtomicBool = AtomicBool::new(false);
    static LAST_SINCE: AtomicU64 = AtomicU64::new(0);
    let settings = crate::settings::load(app);
    let on = settings.external_agent;
    let was_on = WAS_ON.swap(on, Ordering::SeqCst);
    let enabled_again = settings.external_agent_since > LAST_SINCE.swap(settings.external_agent_since, Ordering::SeqCst);
    if on && (!was_on || enabled_again) {
        app.state::<AppState>().lease.locked().rearm();
    }
    on
}

async fn not_allowed() -> StatusCode {
    StatusCode::METHOD_NOT_ALLOWED
}

fn refuse(status: StatusCode, code: &str, message: &str) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], json!({ "error": code, "message": message }).to_string()).into_response()
}

/// Whether the `Host` header names this machine and our port.
fn host_is_local(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    [format!("127.0.0.1:{PORT}"), format!("localhost:{PORT}"), format!("[::1]:{PORT}")].iter().any(|allowed| allowed == host)
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

async fn post_mcp(State(app): State<AppHandle>, headers: HeaderMap, body: Bytes) -> Response {
    if headers.contains_key(header::ORIGIN) {
        return refuse(StatusCode::FORBIDDEN, "origin_refused", "Browser requests are not allowed.");
    }
    if !host_is_local(&headers) {
        return refuse(StatusCode::FORBIDDEN, "host_refused", "This door only answers on 127.0.0.1.");
    }
    if !bearer(&headers).is_some_and(token::matches) {
        return refuse(StatusCode::UNAUTHORIZED, "unauthorized", "Missing or wrong token. Ask the user for the token in Settings > External agent.");
    }
    if !switch_is_on(&app) {
        return refuse(StatusCode::FORBIDDEN, "not_enabled", "The user has not switched External agent on in Settings.");
    }
    let request: Value = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(err) => return refuse(StatusCode::BAD_REQUEST, "bad_json", &err.to_string()),
    };
    match mcp::handle(&app, request).await {
        Some(reply) => ([(header::CONTENT_TYPE, "application/json")], reply.to_string()).into_response(),
        // A notification: nothing to say back.
        None => StatusCode::ACCEPTED.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, value.parse().unwrap());
        }
        map
    }

    #[test]
    fn only_local_hosts_on_our_port_pass() {
        assert!(host_is_local(&headers(&[("host", "127.0.0.1:47600")])));
        assert!(host_is_local(&headers(&[("host", "localhost:47600")])));
        assert!(!host_is_local(&headers(&[("host", "evil.example:47600")])));
        assert!(!host_is_local(&headers(&[("host", "127.0.0.1:80")])));
        assert!(!host_is_local(&headers(&[])));
    }

    #[test]
    fn the_bearer_scheme_is_required() {
        assert_eq!(bearer(&headers(&[("authorization", "Bearer abc")])), Some("abc"));
        assert_eq!(bearer(&headers(&[("authorization", "Basic abc")])), None);
        assert_eq!(bearer(&headers(&[])), None);
    }
}
