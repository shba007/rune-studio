use crate::exec::CancelToken;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, time::Duration};
use tauri::{AppHandle, Manager};
const LIMIT: usize = 65536;
const PROTOCOL: &str = include_str!("../../../docs/mcp-kit-version.txt");
const TOKEN_SERVICE: &str = "rune-studio";
const TOKEN_ACCOUNT: &str = "rune-kit-http-token";

fn stored_kit_token() -> Result<Option<String>, String> {
    let entry = crate::credentials::Entry::new(TOKEN_SERVICE, TOKEN_ACCOUNT)
        .map_err(|e| format!("Cannot access the system credential store: {e}"))?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(std::env::var("RUNE_KIT_HTTP_TOKEN")
            .ok()
            .filter(|v| !v.is_empty())),
        Err(e) => Err(format!("Cannot read the Kit token: {e}")),
    }
}

#[tauri::command]
pub fn kit_token_set(token: String) -> Result<(), String> {
    if token.chars().any(char::is_control) {
        return Err("Kit token cannot contain control characters".into());
    }
    let entry = crate::credentials::Entry::new(TOKEN_SERVICE, TOKEN_ACCOUNT)
        .map_err(|e| format!("Cannot access the system credential store: {e}"))?;
    if token.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Cannot remove the Kit token: {e}")),
        }
    } else {
        entry
            .set_password(&token)
            .map_err(|e| format!("Cannot save the Kit token: {e}"))
    }
}

#[tauri::command]
pub fn kit_token_present() -> Result<bool, String> {
    stored_kit_token().map(|token| token.is_some())
}
fn default_endpoint() -> String {
    "http://127.0.0.1:47601/mcp".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub enabled: bool,
    pub endpoint: String,
    pub selected: Vec<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_endpoint(),
            selected: vec![],
        }
    }
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
fn read_config(path: &std::path::Path) -> Result<Config, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
            "Kit settings are incompatible or malformed; the existing file was preserved".into()
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error.to_string()),
    }
}
pub fn load(app: &AppHandle) -> Config {
    app.path()
        .app_config_dir()
        .ok()
        .and_then(|p| read_config(&p.join("kit.json")).ok())
        .unwrap_or_default()
}
#[tauri::command]
pub fn kit_settings(app: AppHandle) -> Result<Config, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    read_config(&dir.join("kit.json"))
}
#[tauri::command]
pub fn kit_save(app: AppHandle, config: Config) -> Result<(), String> {
    if config
        .selected
        .iter()
        .any(|n| n.is_empty() || n.len() > 256 || n.chars().any(char::is_control))
    {
        return Err("Invalid Kit tool identity".into());
    }
    endpoint(&config.endpoint)?;
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("kit.json");
    read_config(&path)?;
    std::fs::write(
        path,
        serde_json::to_vec(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn endpoint(text: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(text.trim()).map_err(
        |_| "Enter an HTTP loopback MCP endpoint, for example http://127.0.0.1:47601/mcp",
    )?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/mcp"
    {
        return Err("Kit must use an HTTP loopback /mcp endpoint without embedded credentials, query or fragment".into());
    }
    Ok(url)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigurationField {
    plugin: String,
    key: String,
    kind: String,
    description: String,
    required: bool,
    secret: bool,
    locked: bool,
    value: String,
}

#[derive(Deserialize)]
struct ConfigurationFields {
    fields: Vec<ConfigurationField>,
}

#[derive(Deserialize, Serialize)]
pub struct ConfigurationUpdate {
    plugin: String,
    key: String,
    value: String,
    shared: bool,
}

fn management_client(config: &Config) -> Result<(reqwest::Client, reqwest::Url), String> {
    let mut url = endpoint(&config.endpoint)?;
    url.set_path("/management/config");
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    Ok((client, url))
}

async fn management_response(response: reqwest::Response) -> Result<reqwest::Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else if response.status() == reqwest::StatusCode::NOT_FOUND {
        Err("Kit could not find this management endpoint or item. Refresh the list or update Kit.".into())
    } else {
        let status = response.status();
        let message = response.text().await.unwrap_or_default();
        Err(format!(
            "Kit configuration request failed ({status}): {}",
            message.chars().take(512).collect::<String>()
        ))
    }
}

#[tauri::command]
pub async fn kit_configuration(config: Config) -> Result<Vec<ConfigurationField>, String> {
    let (client, url) = management_client(&config)?;
    let mut request = client.get(url);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let response = management_response(request.send().await.map_err(|e| e.to_string())?).await?;
    if response
        .content_length()
        .is_some_and(|length| length > LIMIT as u64)
    {
        return Err("Kit configuration response is too large".into());
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Kit configuration response is too large".into());
    }
    serde_json::from_slice::<ConfigurationFields>(&bytes)
        .map(|result| result.fields)
        .map_err(|_| "Invalid Kit configuration response".into())
}

#[tauri::command]
pub async fn kit_update_configuration(
    config: Config,
    update: ConfigurationUpdate,
) -> Result<(), String> {
    let (client, url) = management_client(&config)?;
    let mut request = client.put(url).json(&update);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    management_response(request.send().await.map_err(|e| e.to_string())?).await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct KitRemote {
    name: String,
    endpoint: String,
    protocol: String,
    token_env: Option<String>,
}

#[derive(Deserialize)]
struct KitRemotes {
    servers: Vec<KitRemote>,
}

#[tauri::command]
pub async fn kit_remotes(config: Config) -> Result<Vec<KitRemote>, String> {
    let (client, mut url) = management_client(&config)?;
    url.set_path("/management/remotes");
    let mut request = client.get(url);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let mut response =
        management_response(request.send().await.map_err(|e| e.to_string())?).await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Kit remote list is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice::<KitRemotes>(&bytes)
        .map(|result| result.servers)
        .map_err(|_| "Invalid Kit remote list".into())
}

#[tauri::command]
pub async fn kit_packages(config: Config) -> Result<Value, String> {
    let (client, mut url) = management_client(&config)?;
    url.set_path("/management/packages");
    let mut request = client.get(url);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let mut response =
        management_response(request.send().await.map_err(|e| e.to_string())?).await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Kit package list is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit package list")?;
    if !value["plugins"].is_array() || !value["skills"].is_array() {
        return Err("Invalid Kit package list".into());
    }
    Ok(value)
}

#[derive(Deserialize, Serialize)]
pub struct PackageInstall {
    kind: String,
    target: String,
    version: Option<String>,
    prefer_native: bool,
}

#[tauri::command]
pub async fn kit_install_package(config: Config, package: PackageInstall) -> Result<Value, String> {
    let (client, mut url) = management_client(&config)?;
    url.set_path("/management/packages");
    let mut request = client
        .post(url)
        .timeout(Duration::from_secs(300))
        .json(&package);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let response = management_response(request.send().await.map_err(|e| format!("Kit installation response unavailable: {e}. Check Kit's installed packages before retrying."))?).await?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Kit installation response is too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit installation response".into())
}

#[tauri::command]
pub async fn kit_package_operation(
    config: Config,
    action: String,
    kind: String,
    name: String,
) -> Result<Value, String> {
    if !matches!(action.as_str(), "check" | "update" | "remove") {
        return Err("Invalid package action".into());
    }
    let (client, mut url) = management_client(&config)?;
    url.set_path(&format!("/management/packages/{action}"));
    let mut request = client
        .post(url)
        .timeout(Duration::from_secs(300))
        .json(&json!({"kind":kind,"name":name}));
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let response = management_response(request.send().await.map_err(|e| {
        format!("Kit response unavailable: {e}. Check installed packages before retrying.")
    })?)
    .await?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Kit response is too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit package response".into())
}

#[tauri::command]
pub async fn kit_remote_update(
    config: Config,
    name: String,
    remote: Option<KitRemote>,
) -> Result<(), String> {
    let (client, mut url) = management_client(&config)?;
    url.set_path("/management/remotes");
    let body = json!({ "name": name, "config": remote.map(|remote| json!({
        "endpoint": remote.endpoint, "protocol": remote.protocol, "token_env": remote.token_env,
    })) });
    let mut request = client.put(url).json(&body);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    management_response(request.send().await.map_err(|e| e.to_string())?).await?;
    Ok(())
}
pub fn compatible_version(text: &str) -> bool {
    let parts: Vec<_> = text
        .trim()
        .trim_start_matches("rune-kit ")
        .trim_start_matches('v')
        .split('.')
        .map(str::parse::<u64>)
        .collect();
    matches!(parts.as_slice(), [Ok(a),Ok(b),Ok(c)] if (*a,*b,*c)>=(0,6,0))
}
struct Gateway {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    cancel: CancelToken,
    next: u64,
    unavailable: Vec<String>,
    selected: Option<Vec<String>>,
    headers: Vec<(String, String)>,
}
impl Gateway {
    fn remote_names(&self) -> HashSet<String> {
        let mut url = self.endpoint.clone();
        url.set_path("/management/remotes");
        let mut request = self.client.get(url).timeout(Duration::from_secs(5));
        match stored_kit_token() {
            Ok(Some(token)) => request = request.bearer_auth(token),
            Ok(None) => (),
            Err(_) => return HashSet::new(),
        }
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return HashSet::new();
        };
        runtime.block_on(async {
            let Ok(mut response) = request.send().await else {
                return HashSet::new();
            };
            if !response.status().is_success() {
                return HashSet::new();
            }
            let mut bytes = Vec::new();
            while let Ok(Some(chunk)) = response.chunk().await {
                if bytes.len() + chunk.len() > LIMIT {
                    return HashSet::new();
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice::<KitRemotes>(&bytes)
                .map(|list| list.servers.into_iter().map(|server| server.name).collect())
                .unwrap_or_default()
        })
    }
    fn start(
        config: &Config,
        cancel: &CancelToken,
        discovery: bool,
    ) -> Result<(Self, String), String> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|e| e.to_string())?;
        let mut gateway = Self {
            client,
            endpoint: endpoint(&config.endpoint)?,
            cancel: cancel.clone(),
            next: 1,
            unavailable: vec![],
            selected: (!discovery).then(|| config.selected.clone()),
            headers: vec![],
        };
        let found = gateway.request("server/discover", json!({}), Duration::from_secs(5))?;
        if !found["supportedVersions"]
            .as_array()
            .is_some_and(|v| v.iter().any(|v| v == PROTOCOL))
            || !found["capabilities"]["tools"].is_object()
        {
            return Err("Incompatible Kit HTTP protocol/capabilities. Start an HTTP-capable Kit 0.6.0+ server and check the endpoint.".into());
        }
        let info = &found["_meta"]["io.modelcontextprotocol/serverInfo"];
        let version = info["version"]
            .as_str()
            .ok_or("Kit did not report its server version")?;
        if info["name"] != "rune-kit" {
            return Err("The endpoint is not a rune-kit server".into());
        }
        let preview = cfg!(debug_assertions)
            && std::env::var("RUNE_STUDIO_ALLOW_KIT_HTTP_PREVIEW").as_deref() == Ok("1");
        if !compatible_version(version) && !preview {
            return Err(format!(
                "Kit {version} is older than the HTTP-supported release 0.6.0. Upgrade the server; Studio never starts a legacy subprocess."
            ));
        }
        Ok((gateway, format!("rune-kit {version}")))
    }
    fn request(
        &mut self,
        method: &str,
        mut params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":PROTOCOL,"io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"rune-studio","version":crate::VERSION}});
        if let Some(selected) = &self.selected {
            params["_meta"]["io.rune-kit/allowedTools"] = json!(selected);
        }
        let id = self.next;
        self.next += 1;
        let mut request = self
            .client
            .post(self.endpoint.clone())
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", PROTOCOL)
            .header("Mcp-Method", method);
        if let Some(name) = params["name"].as_str() {
            request = request.header("Mcp-Name", super::headers::encode(name));
        }
        for (key, value) in std::mem::take(&mut self.headers) {
            request = request.header(key, value);
        }
        if let Some(token) = stored_kit_token()? {
            request = request.bearer_auth(token);
        }
        let work = async {
            let mut response = request.json(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})).send().await.map_err(|e|format!("Cannot reach Kit HTTP endpoint: {e}. Start Kit separately with serve --headless."))?;
            let status = response.status();
            if matches!(status.as_u16(), 401 | 403) {
                return Err("Kit HTTP authentication refused. Set the Kit token in Studio settings to match the server.".into());
            }

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .to_owned();
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| format!("Kit connection lost; operation was not retried: {e}"))?
            {
                if bytes.len() + chunk.len() > LIMIT {
                    return Err("Kit response exceeded 64 KB".into());
                }
                bytes.extend_from_slice(&chunk);
                if content_type == "text/event-stream"
                    && let Some(value) = parse_sse(&bytes, id)?
                {
                    return envelope(value, id, status.is_success());
                }
            }
            if content_type == "application/json" {
                envelope(
                    serde_json::from_slice(&bytes)
                        .map_err(|e| format!("Malformed Kit JSON: {e}"))?,
                    id,
                    status.is_success(),
                )
            } else {
                Err(format!(
                    "Kit HTTP {status}: missing final JSON/SSE response"
                ))
            }
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        runtime.block_on(async {
            match crate::agent::unless_stopped(&self.cancel, tokio::time::timeout(timeout, work))
                .await
            {
                None => Err("cancelled".into()),
                Some(Err(_)) => Err("Kit HTTP request timed out; operation was not retried".into()),
                Some(Ok(result)) => result,
            }
        })
    }
    fn tools(&mut self, _config: &Config) -> Result<Vec<Tool>, String> {
        let result = self.request("tools/list", json!({}), Duration::from_secs(5))?;
        if result.get("nextCursor").is_some_and(|v| !v.is_null()) {
            return Err("Paginated tool discovery is not supported in this slice".into());
        }
        let entries = result["tools"]
            .as_array()
            .ok_or("Kit discovery did not return tools")?;
        let remote_names = if entries.iter().any(|entry| {
            entry["name"]
                .as_str()
                .is_some_and(|name| !super::approved_identity(name))
        }) {
            self.remote_names()
        } else {
            HashSet::new()
        };
        let mut names = HashSet::new();
        let mut tools = Vec::new();
        for entry in entries {
            let Some(name) = entry["name"].as_str() else {
                continue;
            };
            if !names.insert(name.to_owned()) {
                return Err("Duplicate Kit tool identity".into());
            }
            let remote = name.split_once("__").is_some_and(|(namespace, tool)| {
                !tool.is_empty() && remote_names.contains(namespace)
            });
            if !super::approved_identity(name) && !remote {
                continue;
            }
            let reason = if super::headers::extract(&entry["inputSchema"], &json!({})).is_err() {
                Some("invalid MCP header annotations")
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

fn envelope(value: Value, id: u64, successful: bool) -> Result<Value, String> {
    if value["jsonrpc"] != "2.0" || value["id"] != id {
        return Err("Malformed or mismatched Kit response id".into());
    }
    if let Some(error) = value.get("error") {
        return Err(format!("Kit RPC {}: {}", error["code"], error["message"]));
    }
    if !successful {
        return Err("Kit returned an unsuccessful HTTP status".into());
    }
    let result = value.get("result").cloned().ok_or("Kit result missing")?;
    if result["resultType"] != "complete" {
        return Err(
            "Unsupported or missing Kit resultType; only complete results are supported".into(),
        );
    }
    Ok(result)
}
fn parse_sse(bytes: &[u8], id: u64) -> Result<Option<Value>, String> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) if error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()])
                .map_err(|_| "Kit sent invalid UTF-8")?
        }
        Err(_) => return Err("Kit sent invalid UTF-8".into()),
    }
    .replace("\r\n", "\n");
    let mut frames = text.split("\n\n").collect::<Vec<_>>();
    frames.pop();
    for frame in frames {
        let data = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(&data).map_err(|e| format!("Malformed Kit SSE JSON: {e}"))?;
        if value.get("id").is_some() {
            if value["id"] != id {
                return Err("Mismatched Kit SSE response id".into());
            }
            return Ok(Some(value));
        }
        if value["jsonrpc"] != "2.0" || !value["method"].is_string() {
            return Err("Malformed Kit SSE notification".into());
        }
    }
    Ok(None)
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
    if args.to_string().len() > 8192 {
        return Err("Kit arguments exceed 8 KB".into());
    }
    if !config.enabled || !config.selected.iter().any(|n| n == name) {
        return Err("Tool is not selected or approved".into());
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
    gateway.headers = super::headers::extract(&tool.input_schema, args)?;
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

    #[test]
    fn kit_token_uses_credential_store_and_can_be_removed() {
        kit_token_set("test-token".into()).unwrap();
        assert_eq!(stored_kit_token().unwrap().as_deref(), Some("test-token"));
        assert!(kit_token_set("bad\ntoken".into()).is_err());
        assert_eq!(stored_kit_token().unwrap().as_deref(), Some("test-token"));
        kit_token_set(String::new()).unwrap();
        assert!(matches!(
            crate::credentials::Entry::new(TOKEN_SERVICE, TOKEN_ACCOUNT)
                .unwrap()
                .get_password(),
            Err(keyring::Error::NoEntry)
        ));
    }
    #[test]
    fn http_endpoint_and_minimum_version_are_enforced() {
        assert!(endpoint("http://example.com/mcp").is_err());
        assert!(endpoint("http://127.0.0.1:47601/mcp").is_ok());
        assert!(!compatible_version("0.5.0"));
        assert!(compatible_version("0.6.0"));
    }
    fn server(mode: &str, count: usize) -> (Config, std::thread::JoinHandle<()>) {
        use std::io::{BufRead, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
        let mode = mode.to_string();
        let thread = std::thread::spawn(move || {
            for _ in 0..count {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut headers = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    headers.push_str(&line);
                }
                if mode == "remote" && headers.starts_with("GET /management/remotes ") {
                    let text = json!({"servers":[{"name":"external","endpoint":"https://example.test/mcp","protocol":"2026-07-28","token_env":null}]}).to_string();
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", text.len(), text).unwrap();
                    continue;
                }
                let len: usize = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(str::trim)
                            .map(str::to_owned)
                    })
                    .unwrap()
                    .parse()
                    .unwrap();
                let mut bytes = vec![0; len];
                reader.read_exact(&mut bytes).unwrap();
                let body: Value = serde_json::from_slice(&bytes).unwrap();
                let method = body["method"].as_str().unwrap();
                assert!(
                    headers
                        .to_ascii_lowercase()
                        .contains(&format!("mcp-method: {method}"))
                );
                assert!(headers.contains(PROTOCOL));
                assert_eq!(
                    body["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
                    PROTOCOL
                );
                let result = match method {
                    "server/discover" => {
                        json!({"supportedVersions":[PROTOCOL],"capabilities":{"tools":{}},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"rune-kit","version":"0.6.0"}},"resultType":"complete"})
                    }
                    "tools/list" => {
                        let name = if mode == "remote" {
                            "external__clock"
                        } else {
                            "time__get_current_time"
                        };
                        let safe = json!({"name":name,"description":"time","annotations":{"readOnlyHint":true,"destructiveHint":false},"inputSchema":{"type":"object","properties":{"timezone":{"type":"string","x-mcp-header":"Zone"}},"additionalProperties":false,"x-rune-rules":{"read_only":true,"risk":"low","effects":["read"]}}});
                        let mut tools = vec![safe.clone()];
                        if mode == "remote" {
                            let mut unsafe_tool = safe.clone();
                            unsafe_tool["name"] = json!("external__unsafe");
                            unsafe_tool["annotations"]["readOnlyHint"] = json!(false);
                            tools.push(unsafe_tool);
                            let mut impostor = safe;
                            impostor["name"] = json!("other__clock");
                            tools.push(impostor);
                        }
                        json!({"resultType":"complete","tools":tools})
                    }
                    "tools/call" => {
                        assert!(headers.contains("Mcp-Name") || headers.contains("mcp-name"));
                        assert!(headers.to_ascii_lowercase().contains("mcp-param-zone: utc"));
                        if mode == "slow" {
                            std::thread::sleep(Duration::from_millis(500));
                        }
                        json!({"resultType":"complete","content":[{"type":"text","text":"UTC"}],"isError":mode=="error"})
                    }
                    _ => panic!("unexpected method"),
                };
                let response = json!({"jsonrpc":"2.0","id":if mode=="wrong-id" {999}else{body["id"].as_u64().unwrap()},"result":result});
                let text = if mode == "oversized" {
                    "x".repeat(LIMIT + 1)
                } else if mode == "json" {
                    response.to_string()
                } else {
                    format!(": keepalive\n\nevent: message\ndata: {response}\n\n")
                };
                let ct = if mode == "json" {
                    "application/json"
                } else {
                    "text/event-stream"
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    text.len()
                );
                for part in text.as_bytes().chunks(13) {
                    if stream.write_all(part).is_err() {
                        break;
                    }
                    let _ = stream.flush();
                }
            }
        });
        (
            Config {
                enabled: true,
                endpoint,
                selected: vec!["time__get_current_time".into()],
                ..Config::default()
            },
            thread,
        )
    }

    #[test]
    fn selected_remote_tools_are_discovered_and_called_through_kit() {
        let (mut config, server) = server("remote", 7);
        config.selected = vec!["external__clock".into()];
        let cancel = CancelToken::default();
        let discovery = discover(&config, &cancel).unwrap();
        assert_eq!(discovery.tools.len(), 1);
        assert_eq!(discovery.tools[0].name, "external__clock");
        assert!(
            call(
                &config,
                "external__clock",
                &json!({"timezone":"UTC"}),
                &cancel
            )
            .is_ok()
        );
        server.join().unwrap();
    }
    #[test]
    fn http_discovery_sse_json_selected_call_and_errors() {
        for mode in ["json", "sse", "error"] {
            let (config, server) = server(mode, 3);
            let result = call(
                &config,
                "time__get_current_time",
                &json!({"timezone":"UTC"}),
                &CancelToken::default(),
            );
            if mode == "error" {
                assert!(result.unwrap_err().contains("Kit tool error"));
            } else {
                assert!(result.unwrap().contains("UTC"));
            }
            server.join().unwrap();
        }
        let (config, server) = server("wrong-id", 1);
        assert!(
            discover(&config, &CancelToken::default())
                .unwrap_err()
                .contains("id")
        );
        server.join().unwrap();
    }
    #[test]
    fn cancellation_closes_only_the_request_and_never_retries_a_tool() {
        let (config, server) = server("slow", 3);
        let cancel = CancelToken::default();
        let stop = cancel.clone();
        let task = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            stop.cancel();
        });
        assert!(
            call(
                &config,
                "time__get_current_time",
                &json!({"timezone":"UTC"}),
                &cancel
            )
            .unwrap_err()
            .contains("cancelled")
        );
        task.join().unwrap();
        server.join().unwrap();
    }

    #[test]
    fn timeout_and_response_bounds_do_not_turn_into_empty_success() {
        let (config, server) = server("slow", 2);
        let (mut gateway, _) = Gateway::start(&config, &CancelToken::default(), false).unwrap();
        gateway.headers = vec![("Mcp-Param-Zone".into(), "UTC".into())];
        assert!(
            gateway
                .request(
                    "tools/call",
                    json!({"name":"time__get_current_time","arguments":{"timezone":"UTC"}}),
                    Duration::from_millis(30)
                )
                .unwrap_err()
                .contains("timed out")
        );
        server.join().unwrap();
        let (config, server) = self::server("oversized", 1);
        assert!(
            discover(&config, &CancelToken::default())
                .unwrap_err()
                .contains("64 KB")
        );
        server.join().unwrap();
    }
    #[test]
    fn incompatible_settings_are_rejected_without_modification() {
        let dir = std::env::temp_dir().join(format!(
            "studio-http-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("kit.json");
        let original = b"{invalid";
        std::fs::write(&path, original).unwrap();
        assert!(read_config(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn unselected_calls_do_not_connect_and_sse_rejects_empty_or_wrong_replies() {
        let config = Config::default();
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
        assert!(parse_sse(b": ping\n\n", 1).unwrap().is_none());
        assert!(parse_sse(b"data: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n", 1).is_err());
    }
    #[test]
    #[ignore = "requires an owner-started Kit HTTP server and plugin artifact"]
    fn actual_kit_http_server() {
        let endpoint = std::env::var("RUNE_STUDIO_KIT_TEST_ENDPOINT").expect("endpoint");
        let tool =
            std::env::var("RUNE_STUDIO_KIT_TEST_TOOL").unwrap_or("time__get_current_time".into());
        let args: Value = serde_json::from_str(
            &std::env::var("RUNE_STUDIO_KIT_TEST_ARGS").unwrap_or("{\"timezone\":\"UTC\"}".into()),
        )
        .unwrap();
        let config = Config {
            enabled: true,
            endpoint,
            selected: vec![tool.clone()],
            ..Config::default()
        };
        let found = discover(&config, &CancelToken::default()).unwrap();
        println!(
            "{}: {:?}",
            found.version,
            found.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        let result = call(&config, &tool, &args, &CancelToken::default()).unwrap();
        println!("{result}");
        assert!(!result.is_empty());
        let mut unselected = config.clone();
        unselected.selected.clear();
        assert!(call(&unselected, &tool, &args, &CancelToken::default()).is_err());
        if tool.ends_with("__read_text_file") {
            let listing = tool.replace("__read_text_file", "__list_directory");
            let mut listing_config = config.clone();
            listing_config.selected.push(listing.clone());
            assert!(
                call(
                    &listing_config,
                    &listing,
                    &json!({"path":"."}),
                    &CancelToken::default()
                )
                .unwrap()
                .contains("hello.txt")
            );
            assert!(
                call(
                    &config,
                    &tool,
                    &json!({"path":"../Cargo.toml"}),
                    &CancelToken::default()
                )
                .is_err()
            );
            assert!(
                call(
                    &config,
                    &tool,
                    &json!({"path":"missing.txt"}),
                    &CancelToken::default()
                )
                .is_err()
            );
            assert!(
                call(
                    &config,
                    &tool,
                    &json!({"path":"hello.txt","allowed_dir":"C:/"}),
                    &CancelToken::default()
                )
                .is_err()
            );
        }
    }
}
