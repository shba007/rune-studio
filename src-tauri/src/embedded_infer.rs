use crate::sync::LockExt;
use std::{path::Path, time::Duration};
use tauri::Manager;

const CONNECTION_ID: &str = "studio-embedded-infer";
static ROUTE: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);

pub(crate) fn route() -> Option<(String, String)> {
    ROUTE.locked().clone()
}

fn clear_route(base_url: &str) {
    let mut route = ROUTE.locked();
    if route.as_ref().is_some_and(|(url, _)| url == base_url) {
        *route = None;
    }
}

struct RouteGuard(String);
impl Drop for RouteGuard {
    fn drop(&mut self) {
        clear_route(&self.0);
    }
}

#[derive(Default)]
pub struct EmbeddedManager(tokio::sync::Mutex<Option<(String, EmbeddedInfer)>>);

impl EmbeddedManager {
    pub async fn reload(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let state = app.state::<crate::events::AppState>();
        {
            let analysis = state.analysis.locked();
            crate::connections::ensure_idle(app, &state, analysis.status)
                .map_err(|error| error.to_string())?;
            if state.settings.locked().active_connection_id != CONNECTION_ID {
                return Err("The active Infer connection changed; reload the catalog".into());
            }
        }
        let root = infer_root()?;
        let mut runtime = self.0.lock().await;
        let (model, previous) = runtime.take().ok_or("Local Infer is not running")?;
        previous.shutdown().await?;
        *runtime = Some((model, EmbeddedInfer::start(&root).await?));
        state.capabilities.invalidate();
        Ok(())
    }
    pub async fn start(&self, app: &tauri::AppHandle, activate: bool) -> Result<(), String> {
        self.start_model(app, activate, String::new(), false).await
    }

    pub async fn start_model(
        &self,
        app: &tauri::AppHandle,
        activate: bool,
        model: String,
        select: bool,
    ) -> Result<(), String> {
        let mut runtime = self.0.lock().await;
        let state = app.state::<crate::events::AppState>();
        if select && model.trim().is_empty() {
            return Err("Choose an agent model before starting local Infer".into());
        }
        if let Some((_, running)) = runtime.as_ref() {
            if !running.task.is_finished() {
                if activate {
                    if select {
                        crate::connections::install_embedded_connection(app, &state, true)
                            .map_err(|error| error.to_string())?;
                        crate::infer_catalog::select_agent(&state, &model).await?;
                        let root = infer_root()?;
                        if let Some((_, previous)) = runtime.take() {
                            previous.shutdown().await?;
                        }
                        *runtime = Some((model, EmbeddedInfer::start(&root).await?));
                        return Ok(());
                    }
                    let exists = state
                        .settings
                        .locked()
                        .connections
                        .iter()
                        .any(|connection| connection.id == CONNECTION_ID);
                    if exists {
                        let revision = state.settings.locked().connections_revision;
                        crate::connections::connections_activate(
                            app.clone(),
                            state,
                            revision,
                            CONNECTION_ID.into(),
                        )
                        .map_err(|error| error.to_string())?;
                    } else {
                        crate::connections::install_embedded_connection(app, &state, true)
                            .map_err(|error| error.to_string())?;
                    }
                }
                return Ok(());
            }
        }
        {
            let analysis = state.analysis.locked();
            crate::connections::ensure_idle(app, &state, analysis.status)
                .map_err(|error| error.to_string())?;
        }
        if let Some((_, previous)) = runtime.take() {
            previous.shutdown().await?;
        }
        let root = infer_root()?;
        let mut server = EmbeddedInfer::start(&root).await?;
        if let Err(error) = crate::connections::install_embedded_connection(app, &state, activate) {
            let _ = server.shutdown().await;
            return Err(error.to_string());
        }
        if select && let Err(error) = crate::infer_catalog::select_agent(&state, &model).await {
            let _ = server.shutdown().await;
            return Err(error);
        }
        if select {
            server.shutdown().await?;
            server = EmbeddedInfer::start(&root).await?;
        }
        *runtime = Some((model, server));
        Ok(())
    }

    pub async fn stop(&self) {
        if let Some((_, server)) = self.0.lock().await.take()
            && let Err(error) = server.shutdown().await
        {
            log::error!("{error}");
        }
    }
}

fn infer_root() -> Result<std::path::PathBuf, String> {
    if std::env::var_os("RUNE_HOME")
        .filter(|value| !value.is_empty())
        .is_some_and(|value| !Path::new(&value).is_absolute())
    {
        return Err("RUNE_HOME must be an absolute path for local Infer".into());
    }
    rune_infer::home::rune_home().map_err(|error| error.to_string())
}

pub struct EmbeddedInfer {
    pub base_url: String,
    #[cfg(test)]
    pub api_key: String,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<Result<(), String>>,
}

impl EmbeddedInfer {
    pub async fn start(root: &Path) -> Result<Self, String> {
        if !root.is_absolute() {
            return Err("Infer home must be an absolute path".into());
        }
        let server = rune_infer::embedding::LocalServer::start_catalog(root).await?;
        let base_url = server.base_url.clone();
        let api_key = server.api_key.clone();
        *ROUTE.locked() = Some((base_url.clone(), api_key.clone()));
        let route_guard = RouteGuard(base_url.clone());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _route = route_guard;
            tokio::select! {
                _ = stopped => {},
                _ = async { while !server.is_finished() { tokio::time::sleep(Duration::from_millis(100)).await; } } => {},
            }
            server.shutdown().await
        });
        Ok(Self {
            base_url,
            #[cfg(test)]
            api_key,
            stop: Some(stop),
            task,
        })
    }

    pub async fn shutdown(mut self) -> Result<(), String> {
        clear_route(&self.base_url);
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        match tokio::time::timeout(Duration::from_secs(10), &mut self.task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(format!("Local Infer stopped unexpectedly: {error}")),
            Err(_) => {
                self.task.abort();
                let _ = self.task.await;
                Err("Local Infer did not stop in time".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn private_server_serves_selected_model_and_stops() {
        let mut configured = crate::settings::Settings::default();
        configured.connections[0].id = "studio-embedded-infer".into();
        configured.connections[0].base_url = "http://127.0.0.1:1337".into();
        configured.active_connection_id = "studio-embedded-infer".into();
        configured.base_url = "http://127.0.0.1:1337".into();
        let configured = std::sync::Mutex::new(configured);
        let (before, key) = crate::settings::snapshot_with_api_key(&configured);
        assert_eq!(before.base_url, "http://127.0.0.1:0");
        assert!(key.is_none());
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join("config")).unwrap();
        std::fs::create_dir(home.path().join("models")).unwrap();
        std::fs::write(home.path().join("models/model.gguf"), b"fixture").unwrap();
        std::fs::write(home.path().join("config/server.json"), r#"{"schema_version":3,"server":{"host":"127.0.0.1","port":8080,"max_loaded_models":1,"idle_unload_seconds":300,"api_key":null}}"#).unwrap();
        std::fs::write(home.path().join("config/catalog.json"), r#"{"schema_version":3,"models":[{"id":"fixture","name":"Fixture","modality":"Text","engine":{"supported":["llama-server"]},"local":{"path":"models/model.gguf"}},{"id":"cloud","name":"Cloud","modality":"Text","remote":{"provider":"openai","upstream_model":"fixture","api_key":"test-key"}}],"roles":{"agent":"fixture","voice":"cloud"}}"#).unwrap();
        let infer = EmbeddedInfer::start(home.path()).await.unwrap();
        let (running, key) = crate::settings::snapshot_with_api_key(&configured);
        assert_eq!(running.base_url, infer.base_url);
        assert_eq!(key.as_deref(), Some(infer.api_key.as_str()));
        let client = reqwest::Client::new();
        let url = format!("{}/v1/models", infer.base_url);
        assert_eq!(client.get(&url).send().await.unwrap().status(), 401);
        let response: serde_json::Value = client
            .get(&url)
            .bearer_auth(&infer.api_key)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(response["data"][0]["id"], "fixture");
        assert_eq!(response["data"].as_array().unwrap().len(), 2);
        let effective = crate::infer_catalog::read_roles(&infer.base_url, Some(&infer.api_key))
            .await
            .unwrap();
        assert_eq!(
            crate::infer_catalog::configured_role(&effective, "agent").as_deref(),
            Some("fixture")
        );
        assert_eq!(
            crate::infer_catalog::configured_role(&effective, "voice").as_deref(),
            Some("cloud")
        );
        let voice: serde_json::Value = client
            .get(format!("{}/v1/models/voice", infer.base_url))
            .bearer_auth(&infer.api_key)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(voice["id"], "cloud");
        infer.shutdown().await.unwrap();
        let (stopped, key) = crate::settings::snapshot_with_api_key(&configured);
        assert_eq!(stopped.base_url, "http://127.0.0.1:0");
        assert!(key.is_none());
        assert!(client.get(&url).send().await.is_err());
        let crashed = EmbeddedInfer::start(home.path()).await.unwrap();
        crashed.task.abort();
        for _ in 0..100 {
            if route().is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            route().is_none(),
            "a failed runtime must not leave a routable endpoint"
        );
    }

    #[tokio::test]
    #[ignore = "loads a real local model and uses machine resources"]
    async fn live_existing_model_responds_and_shuts_down() {
        let home = rune_infer::home::rune_home().unwrap();
        let model = "qwen3.8-27b";
        let infer = EmbeddedInfer::start(&home).await.unwrap();
        use base64::Engine;
        let image = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD
                .encode(include_bytes!("../icons/128x128.png"))
        );
        let response = reqwest::Client::new()
            .post(format!("{}/v1/chat/completions", infer.base_url))
            .bearer_auth(&infer.api_key)
            .timeout(std::time::Duration::from_secs(300))
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role":"user","content":"Reply with the word hello."}],
                "max_tokens": 32,
                "temperature": 0
            }))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(status.is_success(), "Infer returned {status}: {body}");
        assert!(
            body["choices"]
                .as_array()
                .is_some_and(|choices| !choices.is_empty())
        );
        let response = reqwest::Client::new()
            .post(format!("{}/v1/chat/completions", infer.base_url))
            .bearer_auth(&infer.api_key)
            .timeout(std::time::Duration::from_secs(300))
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role":"user","content":[
                    {"type":"text","text":"Call the report tool with the word hello."},
                    {"type":"image_url","image_url":{"url":image}}
                ]}],
                "tools":[{"type":"function","function":{"name":"report","description":"Report the requested word","parameters":{"type":"object","properties":{"word":{"type":"string"}},"required":["word"]}}}],
                "tool_choice":"required",
                "max_tokens": 128,
                "temperature": 0
            }))
            .send().await.unwrap();
        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap();
        assert!(status.is_success(), "Infer returned {status}: {body}");
        assert_eq!(
            body["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "report"
        );
        infer.shutdown().await.unwrap();
    }
}
