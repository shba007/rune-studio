use crate::exec::CancelToken;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

static OWNED: Mutex<Vec<std::sync::Weak<Mutex<Child>>>> = Mutex::new(Vec::new());
static SERIAL: Mutex<()> = Mutex::new(());
pub fn shutdown() {
    if let Ok(mut children) = OWNED.lock() {
        for weak in children.drain(..) {
            if let Some(child) = weak.upgrade()
                && let Ok(mut child) = child.lock()
            {
                let _ = child.kill();
            }
        }
    }
}
const LIMIT: usize = 65536;
const PROTOCOL: &str = "2026-07-28";
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    pub executable: String,
    pub selected: Vec<String>,
    pub root: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub version: String,
    pub tools: Vec<Tool>,
    pub unavailable: Vec<String>,
}
pub fn load(app: &AppHandle) -> Config {
    app.path()
        .app_config_dir()
        .ok()
        .and_then(|p| std::fs::read(p.join("kit.json")).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
#[tauri::command]
pub fn kit_settings(app: AppHandle) -> Config {
    load(&app)
}
#[tauri::command]
pub fn kit_save(app: AppHandle, config: Config) -> Result<(), String> {
    if config.selected.iter().any(|n| !super::approved_identity(n)) {
        return Err(
            "Only approved time, git validators and bounded filesystem reads may be selected"
                .into(),
        );
    }
    configured_root(&config)?;
    shutdown();
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join("kit.json"),
        serde_json::to_vec(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn command(executable: &str) -> Command {
    let mut c = Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c
}
fn executable(config: &Config) -> Result<String, String> {
    let home = dirs::home_dir();
    resolve_executable(
        config,
        home.as_deref(),
        &std::env::var_os("PATH").unwrap_or_default(),
    )
}
fn resolve_executable(
    config: &Config,
    home: Option<&std::path::Path>,
    paths: &std::ffi::OsStr,
) -> Result<String, String> {
    if !config.executable.trim().is_empty() {
        return Ok(config.executable.clone());
    }
    let home = home.filter(|path| path.is_absolute()).ok_or(
        "Cannot resolve your home directory. Choose an explicit rune-kit executable path.",
    )?;
    let name = if cfg!(windows) {
        "rune-kit.exe"
    } else {
        "rune-kit"
    };
    for p in std::iter::once(home.join("rune").join(name))
        .chain(std::env::split_paths(paths).map(|dir| dir.join(name)))
    {
        if p.is_file() {
            return Ok(p.to_string_lossy().into());
        }
    }
    Err("Kit not found. Install rune-kit 0.5.0+ at ~/rune/rune-kit.exe (Windows) or ~/rune/rune-kit, or choose its explicit path. Legacy rune is not supported.".into())
}
fn configured_root(config: &Config) -> Result<Option<String>, String> {
    if config.root.trim().is_empty() {
        return Ok(None);
    }
    let path = std::path::Path::new(&config.root);
    if !path.is_absolute() || !path.is_dir() {
        return Err("Choose an existing absolute filesystem folder; no working-directory fallback is allowed".into());
    }
    // Kit passes this trusted host root into its WASI capability boundary.
    Ok(Some(config.root.replace('\\', "/")))
}
pub fn compatible_version(text: &str) -> bool {
    let mut words = text.split_whitespace();
    if words.next() != Some("rune-kit") {
        return false;
    }
    let Some(version) = words.next() else {
        return false;
    };
    let numbers: Vec<_> = version
        .trim_start_matches('v')
        .split('.')
        .map(str::parse::<u64>)
        .collect();
    matches!(numbers.as_slice(),[Ok(a),Ok(b),Ok(c)] if (*a,*b,*c)>=(0,5,0))
}
struct Gateway {
    child: Arc<Mutex<Child>>,
    stdin: Option<ChildStdin>,
    messages: mpsc::Receiver<Result<Value, String>>,
    next: u64,
    cancel: CancelToken,
    unavailable: Vec<String>,
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.stdin.take(); // EOF is the portable graceful stdio shutdown signal.
        if let Ok(mut c) = self.child.lock() {
            let deadline = Instant::now() + Duration::from_millis(100);
            while Instant::now() < deadline {
                if c.try_wait().ok().flatten().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}
impl Gateway {
    fn start(
        config: &Config,
        cancel: &CancelToken,
        discovery: bool,
    ) -> Result<(Self, String), String> {
        let exe = executable(config)?;
        let mut version = command(&exe)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Cannot start Kit: {e}"))?;
        let stdout = version.stdout.take().ok_or("Kit version output missing")?;
        let version = Arc::new(Mutex::new(version));
        if let Ok(mut owned) = OWNED.lock() {
            owned.retain(|w| w.strong_count() > 0);
            owned.push(Arc::downgrade(&version));
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut s = String::new();
            let result = stdout.take(4097).read_to_string(&mut s).map(|_| s);
            let _ = tx.send(result);
        });
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            if cancel.is_cancelled() || Instant::now() >= deadline {
                let _ = version.lock().map(|mut c| c.kill());
                let _ = version.lock().map(|mut c| c.wait());
                return Err("Kit version check cancelled or timed out".into());
            }
            match version
                .lock()
                .map_err(|e| e.to_string())?
                .try_wait()
                .map_err(|e| e.to_string())?
            {
                Some(status) => {
                    if !status.success() {
                        return Err("Kit --version failed".into());
                    }
                    break;
                }
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let version = rx
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| "Kit version output timeout")?
            .map_err(|e| e.to_string())?;
        if !compatible_version(&version) {
            return Err(format!(
                "rune-kit 0.5.0+ with MCP {PROTOCOL} is required; reported {}. Select the updated rune-kit executable.",
                version.trim()
            ));
        }
        let mut cmd = command(&exe);
        cmd.args(["run", "--all", "--read-only", "--timeout", "15"]);
        if let Some(root) = configured_root(config)? {
            cmd.args(["-p", &format!("allowed_dir={root}")]);
        }
        if !discovery {
            for name in &config.selected {
                cmd.args(["--allow-tool", name]);
            }
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().ok_or("Kit stdin missing")?;
        let stdout = child.stdout.take().ok_or("Kit stdout missing")?;
        let stderr = child.stderr.take().ok_or("Kit stderr missing")?;
        // Diagnostics are bounded and never mixed with protocol output.
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let mut stderr = stderr;
            let mut chunk = [0u8; 1024];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let take = n.min(8192usize.saturating_sub(bytes.len()));
                        bytes.extend_from_slice(&chunk[..take]);
                    }
                }
            }
            if !bytes.is_empty() {
                log::debug!("Kit diagnostics: {}", String::from_utf8_lossy(&bytes));
            }
        });
        let (tx, rx) = mpsc::sync_channel(16);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::new();
                match reader
                    .by_ref()
                    .take((LIMIT + 1) as u64)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) => break,
                    Ok(_) if bytes.len() > LIMIT => {
                        let _ = tx.send(Err("Kit message exceeds size limit".into()));
                        break;
                    }
                    Ok(_) => {
                        let parsed = serde_json::from_slice(&bytes)
                            .map_err(|e| format!("Malformed Kit message: {e}"));
                        if tx.send(parsed).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        break;
                    }
                }
            }
        });
        let child = Arc::new(Mutex::new(child));
        if let Ok(mut owned) = OWNED.lock() {
            owned.retain(|w| w.strong_count() > 0);
            owned.push(Arc::downgrade(&child));
        }
        let watched = Arc::downgrade(&child);
        let stopped = cancel.clone();
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(35);
            loop {
                std::thread::sleep(Duration::from_millis(10));
                let Some(child) = watched.upgrade() else {
                    break;
                };
                if stopped.is_cancelled() || Instant::now() >= deadline {
                    if let Ok(mut child) = child.lock() {
                        let _ = child.kill();
                    }
                    break;
                }
            }
        });
        let mut gateway = Self {
            child,
            stdin: Some(stdin),
            messages: rx,
            next: 1,
            cancel: cancel.clone(),
            unavailable: Vec::new(),
        };
        let discovered = gateway
            .request("server/discover", json!({}), Duration::from_secs(5))
            .map_err(|e| {
                format!("Kit discovery failed: {e}. Use rune-kit 0.5.0+ supporting MCP {PROTOCOL}.")
            })?;
        if !discovered["supportedVersions"]
            .as_array()
            .is_some_and(|versions| versions.iter().any(|v| v == PROTOCOL))
            || !discovered["capabilities"]["tools"].is_object()
        {
            return Err(format!(
                "Kit does not advertise MCP {PROTOCOL} tools. Select a compatible rune-kit 0.5.0+ binary."
            ));
        }
        Ok((gateway, version.trim().into()))
    }
    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("Kit stdin closed")?;
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","method":method,"params":params})
        )
        .and_then(|_| stdin.flush())
        .map_err(|e| e.to_string())
    }
    fn request(
        &mut self,
        method: &str,
        mut params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":PROTOCOL,"io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"rune-studio","version":crate::VERSION}});
        let id = self.next;
        self.next += 1;
        let stdin = self.stdin.as_mut().ok_or("Kit stdin closed")?;
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
        )
        .and_then(|_| stdin.flush())
        .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + timeout;
        loop {
            if self.cancel.is_cancelled() || Instant::now() >= deadline {
                let cancelled = self.cancel.is_cancelled();
                let _ = self.notify(
                    "notifications/cancelled",
                    json!({"requestId":id,"reason":if cancelled{"Stopped"}else{"Timeout"}}),
                );
                return Err(if cancelled {
                    "cancelled"
                } else {
                    "Kit request timeout"
                }
                .into());
            }
            match self.messages.recv_timeout(Duration::from_millis(10)) {
                Ok(Ok(v)) => {
                    if v["jsonrpc"] != "2.0" || !v.is_object() {
                        return Err("Malformed Kit JSON-RPC envelope".into());
                    }
                    if v["id"] != id {
                        continue;
                    }
                    if let Some(error) = v.get("error") {
                        let kind = match error["code"].as_i64() {
                            Some(-32800) => "cancelled",
                            Some(-32600) => "refused",
                            _ => "Kit RPC",
                        };
                        return Err(format!("{kind}: {}: {}", error["code"], error["message"]));
                    }
                    let result = v.get("result").cloned().ok_or("Kit result missing")?;
                    if result["resultType"] != "complete" {
                        return Err("Unsupported or missing Kit resultType; only complete results are supported".into());
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("Kit disconnected; call was not retried".into());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
    fn tools(&mut self, config: &Config) -> Result<Vec<Tool>, String> {
        let result = self.request("tools/list", json!({}), Duration::from_secs(5))?;
        if result.get("nextCursor").is_some_and(|v| !v.is_null()) {
            return Err("Paginated tool discovery is not supported in this slice".into());
        }
        let entries = result["tools"]
            .as_array()
            .ok_or("Kit discovery did not return tools")?;
        let mut names = HashSet::new();
        let mut tools = Vec::new();
        for entry in entries {
            let Some(name) = entry["name"].as_str() else {
                continue;
            };
            if !names.insert(name.to_owned()) {
                return Err("Duplicate Kit tool identity".into());
            }
            if !super::approved_identity(name) {
                continue;
            }
            let reason = if super::filesystem_identity(name) && configured_root(config)?.is_none() {
                Some("choose an existing absolute folder in Tools settings")
            } else if super::filesystem_identity(name)
                && entry["inputSchema"]["x-rune-rules"]["limits"]["root_param"] != "allowed_dir"
            {
                Some("plugin does not declare the required allowed_dir boundary")
            } else if !super::read_only_facts(entry) {
                Some("plugin metadata is missing, conflicting or not read-only")
            } else if !super::schema::supported(&entry["inputSchema"]) {
                Some("plugin input schema contains unsupported constraints")
            } else {
                None
            };
            if let Some(reason) = reason {
                if self.unavailable.len() < 16 {
                    self.unavailable.push(format!(
                        "{}: {reason}",
                        name.chars().take(128).collect::<String>()
                    ));
                }
            } else {
                tools.push(Tool {
                    name: name.into(),
                    description: entry["description"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(1024)
                        .collect(),
                    input_schema: entry["inputSchema"].clone(),
                });
            }
        }
        Ok(tools)
    }
}
pub fn discover(config: &Config, cancel: &CancelToken) -> Result<Discovery, String> {
    let (mut gateway, version) = Gateway::start(config, cancel, true)?;
    let tools = gateway.tools(config)?;
    Ok(Discovery {
        version,
        tools,
        unavailable: gateway.unavailable.clone(),
    })
}
#[tauri::command]
pub async fn kit_discover(config: Config) -> Result<Discovery, String> {
    tauri::async_runtime::spawn_blocking(move || discover(&config, &CancelToken::default()))
        .await
        .map_err(|e| e.to_string())?
}
pub fn call(
    config: &Config,
    name: &str,
    args: &Value,
    cancel: &CancelToken,
) -> Result<String, String> {
    let _serial = loop {
        match SERIAL.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::Poisoned(error)) => break error.into_inner(),
            Err(_) => {
                if cancel.is_cancelled() {
                    return Err("cancelled".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    };
    if args.to_string().len() > 8192 {
        return Err("Kit arguments exceed 8 KB".into());
    }
    if !config.enabled
        || !config.selected.iter().any(|n| n == name)
        || !super::approved_identity(name)
    {
        return Err("Tool is not selected or approved".into());
    }
    if super::filesystem_identity(name) && configured_root(config)?.is_none() {
        return Err("Select a filesystem folder in Tools settings before using file tools".into());
    }
    if super::filesystem_identity(name)
        && (args.get("allowed_dir").is_some() || args.get("allowedDir").is_some())
    {
        return Err("Filesystem root belongs to Tools settings, not tool arguments".into());
    }
    let (mut gateway, _) = Gateway::start(config, cancel, false)?;
    let tool = gateway
        .tools(config)?
        .into_iter()
        .find(|t| t.name == name)
        .ok_or("Tool is unavailable or has unsafe/unsupported metadata")?;
    super::schema::validate(&tool.input_schema, args)?;
    let result = gateway.request(
        "tools/call",
        json!({"name":name,"arguments":args}),
        Duration::from_secs(20),
    )?;
    if !result["content"].is_array() || result.get("isError").is_some_and(|v| !v.is_boolean()) {
        return Err("Malformed MCP tool result".into());
    }
    if result["isError"].as_bool() == Some(true) {
        return Err(format!("Kit tool error: {}", bounded(&result)));
    }
    if result["content"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item["text"].as_str().is_some_and(|text| {
                serde_json::from_str::<Value>(text).ok().is_some_and(|v| {
                    v.get("error").is_some() || v["success"] == false || v["status"] == "error"
                })
            })
        })
    }) {
        return Err(format!("Plugin error: {}", bounded(&result)));
    }
    Ok(bounded(&result))
}
fn bounded(value: &Value) -> String {
    let mut safe = value.clone();
    if let Some(items) = safe["content"].as_array_mut() {
        for item in items {
            if let Some(text) = item["text"].as_str()
                && let Ok(json) = serde_json::from_str::<Value>(text)
            {
                item["text"] = Value::String(crate::trace::redact(&json).0);
            }
        }
    }
    let text = crate::trace::redact(&safe).0;
    if text.len() > 16384 {
        format!(
            "{} [truncated]",
            text.chars().take(8192).collect::<String>()
        )
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(mode: &str) -> Config {
        static BUILT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        let binary = BUILT.get_or_init(|| {
            let dir =
                std::env::temp_dir().join(format!("studio-kit-fixture-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let exe = dir.join(if cfg!(windows) {
                "gateway.exe"
            } else {
                "gateway"
            });
            let status = Command::new("rustc")
                .args(["--edition=2024"])
                .arg(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/kit_gateway.rs"),
                )
                .arg("-o")
                .arg(&exe)
                .status()
                .unwrap();
            assert!(status.success());
            exe
        });
        let target = binary.with_file_name(if cfg!(windows) {
            format!("{mode}.exe")
        } else {
            mode.into()
        });
        std::fs::copy(binary, &target).unwrap();
        Config {
            enabled: true,
            executable: target.to_string_lossy().into(),
            selected: vec!["time__get_current_time".into()],
            root: String::new(),
        }
    }
    #[test]
    fn discovery_and_partial_framing_call_work() {
        let config = fixture("success");
        let found = discover(&config, &CancelToken::default()).unwrap();
        assert_eq!(found.tools.len(), 1);
        assert!(
            call(
                &config,
                "time__get_current_time",
                &json!({"timezone":"UTC"}),
                &CancelToken::default()
            )
            .unwrap()
            .contains("2026")
        );
    }
    #[test]
    fn errors_disconnect_malformed_and_oversized_are_not_success() {
        for mode in ["error", "disconnect", "malformed", "oversized"] {
            assert!(
                call(
                    &fixture(mode),
                    "time__get_current_time",
                    &json!({}),
                    &CancelToken::default()
                )
                .is_err(),
                "{mode}"
            );
        }
    }
    #[test]
    fn unselected_and_invalid_arguments_are_refused() {
        let mut config = fixture("refused");
        config.selected.clear();
        assert!(
            call(
                &config,
                "time__get_current_time",
                &json!({}),
                &CancelToken::default()
            )
            .unwrap_err()
            .contains("selected")
        );
        config.selected.push("time__get_current_time".into());
        assert!(
            call(
                &config,
                "time__get_current_time",
                &json!({"timezone":3}),
                &CancelToken::default()
            )
            .unwrap_err()
            .contains("type")
        );
    }
    #[test]
    fn stop_discards_a_late_reply_and_kills_only_owned_gateway() {
        let config = fixture("ignore");
        let cancel = CancelToken::default();
        let remote = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            remote.cancel()
        });
        let started = Instant::now();
        assert!(call(&config, "time__get_current_time", &json!({}), &cancel).is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn version_titles_and_minimum_are_checked() {
        for version in ["rune-kit 0.5.0", "rune-kit 0.6.0"] {
            assert!(compatible_version(version))
        }
        for version in [
            "rune 0.5.0",
            "rune-kit 0.4.0",
            "something 0.5.0",
            "rune-kit 0.x.1",
        ] {
            assert!(!compatible_version(version))
        }
    }
    #[test]
    fn discovery_errors_are_not_an_empty_success() {
        assert!(
            discover(&fixture("discovery-error"), &CancelToken::default())
                .unwrap_err()
                .contains("-32000")
        );
    }
    #[test]
    fn rpc_cancellation_and_refusal_keep_their_meaning() {
        for (mode, prefix) in [("rpc-cancel", "cancelled:"), ("rpc-refuse", "refused:")] {
            assert!(
                call(
                    &fixture(mode),
                    "time__get_current_time",
                    &json!({}),
                    &CancelToken::default()
                )
                .unwrap_err()
                .starts_with(prefix)
            );
        }
    }
    #[test]
    fn timeout_is_bounded_without_waiting_for_cancel_acknowledgement() {
        let config = fixture("ignore-timeout");
        let cancel = CancelToken::default();
        let (mut gateway, _) = Gateway::start(&config, &cancel, false).unwrap();
        let now = Instant::now();
        assert!(
            gateway
                .request(
                    "tools/call",
                    json!({"name":"time__get_current_time","arguments":{}}),
                    Duration::from_millis(50)
                )
                .unwrap_err()
                .contains("timeout")
        );
        assert!(now.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn large_output_is_explicitly_marked_as_truncated() {
        let text = bounded(&json!({"content":[{"type":"text","text":"x".repeat(30000)}]}));
        assert!(text.ends_with("[truncated]"));
        assert!(text.len() < 17000);
    }

    #[test]
    fn executable_discovery_prefers_shared_home_and_never_uses_legacy_rune() {
        let temp = tempfile::tempdir().unwrap();
        let shared = temp.path().join("rune");
        let path = temp.path().join("path");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::create_dir_all(&path).unwrap();
        let name = if cfg!(windows) {
            "rune-kit.exe"
        } else {
            "rune-kit"
        };
        std::fs::write(shared.join(name), "").unwrap();
        std::fs::write(path.join(name), "").unwrap();
        let paths = std::env::join_paths([&path]).unwrap();
        let config = Config::default();
        assert_eq!(
            resolve_executable(&config, Some(temp.path()), &paths).unwrap(),
            shared.join(name).to_string_lossy()
        );
        let explicit = Config {
            executable: "explicit-kit".into(),
            ..config.clone()
        };
        assert_eq!(
            resolve_executable(&explicit, None, &paths).unwrap(),
            "explicit-kit"
        );
        std::fs::remove_file(shared.join(name)).unwrap();
        assert_eq!(
            resolve_executable(&config, Some(temp.path()), &paths).unwrap(),
            path.join(name).to_string_lossy()
        );
        std::fs::remove_file(path.join(name)).unwrap();
        std::fs::write(
            path.join(if cfg!(windows) { "rune.exe" } else { "rune" }),
            "",
        )
        .unwrap();
        assert!(resolve_executable(&config, Some(temp.path()), &paths).is_err());
        assert!(
            resolve_executable(&config, None, &paths)
                .unwrap_err()
                .contains("home")
        );
    }

    #[test]
    fn filesystem_requires_an_existing_absolute_owner_root() {
        assert!(
            configured_root(&Config {
                root: ".".into(),
                ..Config::default()
            })
            .is_err()
        );
        let config = Config {
            enabled: true,
            selected: vec!["rune-filesystem__read_text_file".into()],
            ..Config::default()
        };
        assert!(
            call(
                &config,
                &config.selected[0],
                &json!({"path":"note.txt"}),
                &CancelToken::default()
            )
            .unwrap_err()
            .contains("folder")
        );
    }

    /// Uses the production Studio client, an isolated Kit registry, and actual plugin artifacts.
    #[test]
    #[ignore = "Requires STUDIO_KIT_TEST_EXE and isolated RUNE_KIT_DATA_DIR containing time/git/filesystem"]
    fn actual_kit_plugins_through_studio() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("note.txt"),
            "Studio inside marker\nsecond line",
        )
        .unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "outside marker").unwrap();
        let mut config = Config {
            enabled: true,
            executable: std::env::var("STUDIO_KIT_TEST_EXE").unwrap(),
            root: root.path().to_string_lossy().into(),
            ..Config::default()
        };
        let cancel = CancelToken::default();
        let without_root = Config {
            root: String::new(),
            ..config.clone()
        };
        let limited = discover(&without_root, &cancel).unwrap();
        assert_eq!(limited.tools.len(), 3);
        assert!(
            limited
                .unavailable
                .iter()
                .any(|reason| reason.contains("folder"))
        );
        let discovery = discover(&config, &cancel).unwrap();
        config.selected = discovery.tools.iter().map(|t| t.name.clone()).collect();
        assert_eq!(config.selected.len(), 5, "{:?}", config.selected);
        for (suffix, args, expected) in [
            (
                "get_current_time",
                json!({"timezone":"Asia/Kolkata"}),
                "2026",
            ),
            (
                "convert_time",
                json!({"time":"2026-10-23T12:00:00","sourceTimezone":"UTC","targetTimezone":"Asia/Kolkata"}),
                "17:30",
            ),
            (
                "validate_commit_message",
                json!({"message":"fix(studio): discover compatible tools","conventional":true}),
                "true",
            ),
            (
                "read_text_file",
                json!({"path":"note.txt","lineLimit":1}),
                "Studio inside marker",
            ),
            (
                "list_directory",
                json!({"path":".","maxEntries":20}),
                "note.txt",
            ),
        ] {
            let name = config
                .selected
                .iter()
                .find(|n| n.ends_with(&format!("__{suffix}")))
                .unwrap();
            let result = call(&config, name, &args, &cancel).unwrap();
            assert!(result.contains(expected), "{suffix}: {result}");
        }
        let read = config
            .selected
            .iter()
            .find(|n| n.ends_with("__read_text_file"))
            .unwrap()
            .clone();
        for path in [
            outside
                .path()
                .join("secret.txt")
                .to_string_lossy()
                .into_owned(),
            "missing.txt".into(),
            "../secret.txt".into(),
        ] {
            assert!(
                call(&config, &read, &json!({"path":path}), &cancel).is_err(),
                "{path}"
            );
        }
        config.selected.retain(|n| n != &read);
        assert!(
            call(&config, &read, &json!({"path":"note.txt"}), &cancel)
                .unwrap_err()
                .contains("selected")
        );
        let stopped = CancelToken::default();
        stopped.cancel();
        assert!(discover(&config, &stopped).is_err());
        assert!(discover(&config, &cancel).is_ok());
        println!(
            "Verified {} through Studio with five selected operations, outside-root/missing-file refusal, deselection and recovery",
            discovery.version
        );
    }
}
