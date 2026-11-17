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
    crate::embedded_kit::endpoint().unwrap_or_else(|| "http://127.0.0.1:47601/mcp".into())
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
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
    #[serde(alias = "plugin")]
    pub mcp: String,
    pub key: String,
    pub kind: String,
    pub description: String,
    pub required: bool,
    pub secret: bool,
    pub locked: bool,
    pub value: String,
}

#[derive(Deserialize)]
struct ConfigurationFields {
    fields: Vec<ConfigurationField>,
}

#[derive(Deserialize, Serialize)]
pub struct ConfigurationUpdate {
    #[serde(alias = "plugin")]
    pub mcp: String,
    pub key: String,
    pub value: String,
    pub shared: bool,
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

fn send_error(error: reqwest::Error, config: &Config, after: &str) -> String {
    if error.is_connect() {
        format!(
            "Kit is not running at {}. Start Kit, then try again.",
            config.endpoint
        )
    } else {
        format!("Kit response unavailable: {error}. {after}")
    }
}

async fn management_response(response: reqwest::Response) -> Result<reqwest::Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else if response.status() == reqwest::StatusCode::NOT_FOUND {
        Err(
            "Kit could not find this management endpoint or item. Refresh the list or update Kit."
                .into(),
        )
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

#[derive(Serialize, Deserialize)]
pub struct KitSandboxApproval {
    path: String,
    kind: String,
}

#[derive(Serialize, Deserialize)]
pub struct KitSandboxPlugin {
    plugin: String,
    workspace: Option<String>,
    approvals: Vec<KitSandboxApproval>,
}

#[derive(Serialize, Deserialize)]
pub struct KitSandboxInventory {
    workspace_only: bool,
    plugins: Vec<KitSandboxPlugin>,
}

async fn sandbox_inventory_request(
    config: &Config,
    token: Option<&str>,
) -> Result<KitSandboxInventory, String> {
    let (client, mut url) = management_client(config)?;
    url.set_path("/management/sandbox");
    let mut request = client.get(url);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let mut response = management_response(
        request
            .send()
            .await
            .map_err(|e| send_error(e, config, "Check that Kit is running."))?,
    )
    .await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Kit sandbox inventory is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit sandbox inventory".into())
}

#[tauri::command]
pub async fn kit_sandbox_inventory(config: Config) -> Result<KitSandboxInventory, String> {
    let token = stored_kit_token()?;
    sandbox_inventory_request(&config, token.as_deref()).await
}

#[derive(Serialize, Deserialize)]
pub struct KitStagedChange {
    path: String,
    staged_path: String,
    deleted: bool,
    baseline: Option<String>,
    staged_digest: Option<String>,
    approval: String,
}

#[derive(Serialize, Deserialize)]
pub struct KitRecovery {
    path: String,
    id: String,
    backup_path: String,
    completed: Option<u64>,
    approval: String,
}

#[derive(Serialize, Deserialize)]
pub struct KitPluginChanges {
    plugin: String,
    reviews: Vec<KitStagedChange>,
    recoveries: Vec<KitRecovery>,
}

#[derive(Serialize, Deserialize)]
pub struct KitChangesInventory {
    plugins: Vec<KitPluginChanges>,
}

async fn changes_inventory_request(
    config: &Config,
    token: Option<&str>,
) -> Result<KitChangesInventory, String> {
    let (client, mut url) = management_client(config)?;
    url.set_path("/management/changes");
    let mut request = client.get(url);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let mut response = management_response(
        request
            .send()
            .await
            .map_err(|e| send_error(e, config, "Check that Kit is running."))?,
    )
    .await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Kit change inventory is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit change inventory".into())
}

#[tauri::command]
pub async fn kit_changes_inventory(config: Config) -> Result<KitChangesInventory, String> {
    let token = stored_kit_token()?;
    changes_inventory_request(&config, token.as_deref()).await
}

#[derive(Serialize, Deserialize)]
pub struct KitChangeAction {
    plugin: String,
    action: String,
    approval: String,
}

async fn change_action_request(
    config: &Config,
    token: Option<&str>,
    action: &KitChangeAction,
) -> Result<(), String> {
    if action.plugin.is_empty()
        || !matches!(action.action.as_str(), "apply" | "restore")
        || action.approval.len() != 64
        || !action.approval.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Invalid Kit change approval".into());
    }
    let (client, mut url) = management_client(config)?;
    url.set_path("/management/changes");
    let mut request = client.post(url).json(action);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    management_response(
        request
            .send()
            .await
            .map_err(|e| send_error(e, config, "Reload the review before retrying."))?,
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn kit_change_action(config: Config, action: KitChangeAction) -> Result<(), String> {
    let token = stored_kit_token()?;
    change_action_request(&config, token.as_deref(), &action).await
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
    let mut response = management_response(
        request
            .send()
            .await
            .map_err(|e| send_error(e, &config, "Check that Kit is running."))?,
    )
    .await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Kit package list is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let mut value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit package list")?;
    if !value["mcps"].is_array() && value["plugins"].is_array() {
        value["mcps"] = value["plugins"].clone();
    }
    if !value["mcps"].is_array() || !value["skills"].is_array() {
        return Err("Invalid Kit package list".into());
    }
    Ok(value)
}

#[tauri::command]
pub async fn kit_registry(config: Config) -> Result<Value, String> {
    let (client, mut url) = management_client(&config)?;
    url.set_path("/management/registry");
    let mut request = client.get(url).timeout(Duration::from_secs(20));
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let mut response = management_response(
        request
            .send()
            .await
            .map_err(|e| send_error(e, &config, "Check that Kit is running and up to date."))?,
    )
    .await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > LIMIT {
            return Err("Kit registry list is too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid Kit registry list")?;
    if !value.is_array() {
        return Err("Invalid Kit registry list".into());
    }
    Ok(value)
}

#[tauri::command]
pub async fn kit_icon(config: Config, name: String) -> Result<Option<String>, String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Ok(None);
    }
    let (client, mut url) = management_client(&config)?;
    url.set_path(&format!("/management/icons/{name}"));
    let mut request = client.get(url);
    if let Some(token) = stored_kit_token()? {
        request = request.bearer_auth(token);
    }
    let Ok(response) = request.send().await else {
        return Ok(None);
    };
    if !response.status().is_success() {
        return Ok(None);
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Ok(None);
    }
    use base64::Engine;
    Ok(Some(format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
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
    let response = management_response(request.send().await.map_err(|e| {
        send_error(
            e,
            &config,
            "Check Kit's installed packages before retrying.",
        )
    })?)
    .await?;
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
                    output_schema: entry.get("outputSchema").cloned(),
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
#[tauri::command]
pub async fn kit_call_tool(
    app: AppHandle,
    name: String,
    args: Value,
) -> Result<ToolCallResult, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("kit.json");
    call_tool_from_settings(path, name, args).await
}

async fn call_tool_from_settings(
    path: std::path::PathBuf,
    name: String,
    args: Value,
) -> Result<ToolCallResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let config = read_config(&path)?;
        call_result(&config, &name, &args, &CancelToken::default())
            .map(|(result, _)| playground_result(result))
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn call(
    config: &Config,
    name: &str,
    args: &Value,
    cancel: &CancelToken,
) -> Result<String, String> {
    call_result(config, name, args, cancel).map(|(result, _)| bounded(&result))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub structured_content_omitted: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepToolResult {
    #[serde(flatten)]
    pub output: ToolCallResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
}

pub fn call_for_step(
    config: &Config,
    name: &str,
    args: &Value,
    cancel: &CancelToken,
) -> Result<StepToolResult, String> {
    call_result(config, name, args, cancel).map(|(result, output_schema)| StepToolResult {
        output: playground_result(result),
        output_schema,
    })
}

fn playground_result(result: Value) -> ToolCallResult {
    let mut output = ToolCallResult {
        text: bounded(&result),
        structured_content: None,
        structured_content_omitted: false,
    };
    if let Some(value) = result.get("structuredContent") {
        let safe = crate::trace::redact(value).0;
        if safe.len() > 16384 {
            output.structured_content_omitted = true;
        } else {
            output.structured_content = serde_json::from_str(&safe).ok();
        }
    }
    output
}

fn call_result(
    config: &Config,
    name: &str,
    args: &Value,
    cancel: &CancelToken,
) -> Result<(Value, Option<Value>), String> {
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
        return Err(format!("MCP error: {}", bounded(&result)));
    }
    Ok((result, tool.output_schema))
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
    fn tool_step_event_has_schema_and_redacted_result_without_changing_model_text() {
        let (config, handle) = server("structured", 6);
        let args = json!({"timezone":"UTC"});
        let output = call_for_step(
            &config,
            "time__get_current_time",
            &args,
            &CancelToken::default(),
        )
        .unwrap();
        let model_text = call(
            &config,
            "time__get_current_time",
            &args,
            &CancelToken::default(),
        )
        .unwrap();
        handle.join().unwrap();
        assert_eq!(output.output.text, model_text);
        let event = serde_json::to_value(crate::events::StepFinished {
            request_id: None,
            step_id: None,
            purpose: None,
            model: None,
            completion_tokens: None,
            tokens_estimated: None,
            attempts: None,
            task_id: Some(12),
            step: 1,
            ok: true,
            error: None,
            result: Some(output),
        })
        .unwrap();
        assert_eq!(event["result"]["text"], model_text);
        assert_eq!(
            event["result"]["structuredContent"],
            json!({"count":3,"password":"[removed]"})
        );
        assert_eq!(
            event["result"]["outputSchema"]["properties"]["count"]["type"],
            "integer"
        );
        assert!(event["result"].get("structuredContentOmitted").is_none());
    }

    #[test]
    fn tool_event_omits_oversized_structured_content_and_screen_event_omits_result() {
        let result = StepToolResult {
            output: playground_result(
                json!({"content":[{"type":"text","text":"summary"}],"structuredContent":{"items":"x".repeat(16385)}}),
            ),
            output_schema: None,
        };
        let mut finished = crate::events::StepFinished {
            request_id: None,
            step_id: None,
            purpose: None,
            model: None,
            completion_tokens: None,
            tokens_estimated: None,
            attempts: None,
            task_id: Some(12),
            step: 1,
            ok: true,
            error: None,
            result: Some(result),
        };
        let event = serde_json::to_value(&finished).unwrap();
        assert_eq!(event["result"]["structuredContentOmitted"], true);
        assert!(event["result"].get("structuredContent").is_none());
        assert!(
            event["result"]["text"]
                .as_str()
                .unwrap()
                .contains("summary")
        );
        finished.result = None;
        let screen = serde_json::to_value(finished).unwrap();
        assert!(screen.get("result").is_none());
        assert_eq!(screen["ok"], true);
    }

    #[test]
    fn output_schema_survives_discovery_and_absent_stays_absent() {
        for mode in ["structured", "json"] {
            let (config, handle) = server(mode, 2);
            let discovery = discover(&config, &CancelToken::default()).unwrap();
            let json = serde_json::to_value(&discovery.tools[0]).unwrap();
            if mode == "structured" {
                assert_eq!(
                    json["outputSchema"]["properties"]["count"]["title"],
                    "Count"
                );
            } else {
                assert!(json.get("outputSchema").is_none());
            }
            handle.join().unwrap();
        }
    }

    #[tokio::test]
    async fn playground_returns_redacted_structured_content_and_legacy_text_is_unchanged() {
        let (config, handle) = server("structured", 3);
        let path = std::env::temp_dir().join(format!(
            "studio-kit-structured-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let result = call_tool_from_settings(
            path.clone(),
            "time__get_current_time".into(),
            json!({"timezone":"UTC"}),
        )
        .await
        .unwrap();
        std::fs::remove_file(path).unwrap();
        handle.join().unwrap();
        assert_eq!(
            result.structured_content,
            Some(json!({"count":3,"password":"[removed]"}))
        );
        assert!(!result.text.contains("private"));
        let (config, handle) = server("structured", 3);
        let legacy = tauri::async_runtime::spawn_blocking(move || {
            call(
                &config,
                "time__get_current_time",
                &json!({"timezone":"UTC"}),
                &CancelToken::default(),
            )
        })
        .await
        .unwrap()
        .unwrap();
        handle.join().unwrap();
        let expected = bounded(
            &json!({"resultType":"complete","content":[{"type":"text","text":"UTC"}],"structuredContent":{"count":3,"password":"private"},"isError":false}),
        );
        assert_eq!(legacy, expected);
        assert_eq!(result.text, legacy);
        assert!(
            serde_json::to_value(result)
                .unwrap()
                .get("structuredContentOmitted")
                .is_none()
        );
    }

    #[test]
    fn oversized_structured_content_is_omitted_whole_and_text_keeps_its_old_bound() {
        let value = json!({"content":[{"type":"text","text":"summary"}],"structuredContent":{"items":"x".repeat(16385)}});
        let expected = bounded(&value);
        let result = serde_json::to_value(playground_result(value)).unwrap();
        assert_eq!(result["text"], expected);
        assert_eq!(result["structuredContentOmitted"], true);
        assert!(result.get("structuredContent").is_none());
        let result = serde_json::to_value(playground_result(json!({"content":[]}))).unwrap();
        assert!(result.get("structuredContent").is_none());
        assert!(result.get("structuredContentOmitted").is_none());
    }

    #[tokio::test]
    async fn playground_call_refuses_unselected_tools_from_saved_settings() {
        let path = std::env::temp_dir().join(format!(
            "studio-kit-playground-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = Config {
            enabled: true,
            endpoint: "http://127.0.0.1:1/mcp".into(),
            selected: vec!["time__convert_time".into()],
        };
        std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let result = call_tool_from_settings(
            path.clone(),
            "time__get_current_time".into(),
            json!({"timezone":"UTC"}),
        )
        .await;
        std::fs::remove_file(path).unwrap();
        assert_eq!(result.unwrap_err(), "Tool is not selected or approved");
    }

    #[tokio::test]
    async fn sandbox_inventory_uses_authenticated_kit_management() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0; 4096];
            let count = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]).to_ascii_lowercase();
            assert!(request.starts_with("get /management/sandbox "));
            assert!(request.contains("authorization: bearer test-key"));
            let body = r#"{"workspace_only":true,"plugins":[{"plugin":"fixture","workspace":null,"approvals":[{"path":"/source","kind":"file"}]}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let config = Config {
            endpoint: format!("http://{address}/mcp"),
            ..Config::default()
        };
        let result = sandbox_inventory_request(&config, Some("test-key"))
            .await
            .unwrap();
        assert!(result.workspace_only);
        assert_eq!(result.plugins[0].plugin, "fixture");
        assert_eq!(result.plugins[0].approvals[0].path, "/source");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn changes_inventory_uses_authenticated_kit_management() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0; 4096];
            let count = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]).to_ascii_lowercase();
            assert!(request.starts_with("get /management/changes "));
            assert!(request.contains("authorization: bearer test-key"));
            let body = r#"{"plugins":[{"plugin":"fixture","reviews":[{"path":"/a","staged_path":"/b","deleted":false,"baseline":"old","staged_digest":"new","approval":"test"}],"recoveries":[]}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let config = Config {
            endpoint: format!("http://{address}/mcp"),
            ..Config::default()
        };
        let result = changes_inventory_request(&config, Some("test-key"))
            .await
            .unwrap();
        assert_eq!(result.plugins[0].reviews[0].path, "/a");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn reviewed_change_action_sends_token_to_kit() {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line);
            }
            let lower = headers.to_ascii_lowercase();
            assert!(lower.starts_with("post /management/changes "));
            assert!(lower.contains("authorization: bearer test-key"));
            let size: usize = lower
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            let mut body = vec![0; size];
            reader.read_exact(&mut body).unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["action"], "apply");
            assert_eq!(body["approval"], "a".repeat(64));
            write!(
                stream,
                "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });
        let config = Config {
            endpoint: format!("http://{address}/mcp"),
            ..Config::default()
        };
        let action = KitChangeAction {
            plugin: "fixture".into(),
            action: "apply".into(),
            approval: "a".repeat(64),
        };
        change_action_request(&config, Some("test-key"), &action)
            .await
            .unwrap();
        server.join().unwrap();
    }

    #[cfg(feature = "embedded-kit")]
    #[tokio::test]
    #[ignore = "requires an isolated RUNE_KIT_DATA_DIR and RUNE_KIT_HTTP_TOKEN"]
    async fn embedded_kit_sandbox_inventory_is_read_only_and_authenticated() {
        let root = std::path::PathBuf::from(
            std::env::var_os("RUNE_KIT_DATA_DIR").expect("isolated Kit data directory"),
        );
        assert!(root.is_absolute() && root.is_dir() && !root.join("sandbox").exists());
        let token = std::env::var("RUNE_KIT_HTTP_TOKEN").expect("test token");
        let mut configuration =
            rune_kit::configuration::Configuration::read(root.join("test-config.json")).unwrap();
        configuration.plugins.push(rune_kit::configuration::Plugin {
            name: "fixture".into(),
            path: Default::default(),
            kind: rune_kit::ExecutionKind::Native,
            manifest: serde_json::from_value(serde_json::json!({})).unwrap(),
            params: Default::default(),
            workspace: None,
        });
        let options = rune_kit::server::Options {
            bind: "127.0.0.1".parse().unwrap(),
            port: 0,
            headless: true,
            workspace_only: false,
            read_only: true,
            allowed_tools: vec![],
            timeout: 1,
            plugins: vec![],
            roots: vec![],
        };
        let server =
            rune_kit::server::start_embedded(rune_kit::McpRouter::new(), options, configuration)
                .await
                .unwrap();
        let config = Config {
            endpoint: server.endpoint.clone(),
            ..Config::default()
        };
        assert!(sandbox_inventory_request(&config, None).await.is_err());
        let inventory = sandbox_inventory_request(&config, Some(&token))
            .await
            .unwrap();
        assert_eq!(inventory.plugins.len(), 1);
        assert_eq!(inventory.plugins[0].plugin, "fixture");
        assert!(inventory.plugins[0].workspace.is_none());
        assert!(changes_inventory_request(&config, None).await.is_err());
        let changes = changes_inventory_request(&config, Some(&token))
            .await
            .unwrap();
        assert_eq!(changes.plugins.len(), 1);
        assert_eq!(changes.plugins[0].plugin, "fixture");
        assert!(changes.plugins[0].reviews.is_empty() && changes.plugins[0].recoveries.is_empty());
        assert!(!root.join("sandbox").exists());
        let original = root.join("fixture.txt");
        std::fs::write(&original, b"original").unwrap();
        let mut sandbox = rune_kit::sandbox::Sandbox::open(root.join("sandbox")).unwrap();
        sandbox.approve("fixture", &original).unwrap();
        let broker = rune_kit::file_access::Broker::open(root.join("sandbox"), "fixture").unwrap();
        broker
            .request(rune_kit::file_access::Request {
                schema_version: 1,
                operation: rune_kit::file_access::Operation::WriteFile,
                path: original.clone(),
                data: b"changed".to_vec(),
            })
            .unwrap();
        let listed = changes_inventory_request(&config, Some(&token))
            .await
            .unwrap();
        let approval = listed.plugins[0].reviews[0].approval.clone();
        let action = KitChangeAction {
            plugin: "fixture".into(),
            action: "apply".into(),
            approval,
        };
        assert!(change_action_request(&config, None, &action).await.is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        change_action_request(&config, Some(&token), &action)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), b"changed");
        assert!(
            change_action_request(&config, Some(&token), &action)
                .await
                .is_err()
        );
        let listed = changes_inventory_request(&config, Some(&token))
            .await
            .unwrap();
        let restore = KitChangeAction {
            plugin: "fixture".into(),
            action: "restore".into(),
            approval: listed.plugins[0].recoveries[0].approval.clone(),
        };
        change_action_request(&config, Some(&token), &restore)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        server.shutdown().await.unwrap();
    }

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
                        let mut safe = json!({"name":name,"description":"time","annotations":{"readOnlyHint":true,"destructiveHint":false},"inputSchema":{"type":"object","properties":{"timezone":{"type":"string","x-mcp-header":"Zone"}},"additionalProperties":false,"x-rune-rules":{"read_only":true,"risk":"low","effects":["read"]}}});
                        if mode == "structured" {
                            safe["outputSchema"] = json!({"type":"object","properties":{"count":{"type":"integer","title":"Count"},"password":{"type":"string"}}});
                        }
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
                        if mode == "structured" {
                            json!({"resultType":"complete","content":[{"type":"text","text":"UTC"}],"structuredContent":{"count":3,"password":"private"},"isError":false})
                        } else {
                            json!({"resultType":"complete","content":[{"type":"text","text":"UTC"}],"isError":mode=="error"})
                        }
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
