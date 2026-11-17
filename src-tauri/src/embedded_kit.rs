use std::sync::{Arc, Mutex};

const NOT_EMBEDDED: &str = "Kit is not embedded; set the folder on your Kit server";

#[cfg(any(feature = "embedded-kit", test))]
trait PluginFolders {
    fn params(&self, plugin: &str) -> std::collections::BTreeMap<String, String>;
    fn set(&self, plugin: &str, key: &str, value: &str) -> Result<(), String>;
}
#[cfg(feature = "embedded-kit")]
impl PluginFolders for rune_kit::server::EmbeddedServer {
    fn params(&self, plugin: &str) -> std::collections::BTreeMap<String, String> {
        self.plugin_params(plugin)
    }
    fn set(&self, plugin: &str, key: &str, value: &str) -> Result<(), String> {
        self.set_plugin_param(plugin, key, value)
    }
}
fn folder_key(key: &str) -> Result<(), String> {
    if key == "allowed_dir" {
        Ok(())
    } else {
        Err("Only allowed_dir can be changed here".into())
    }
}
#[cfg(any(feature = "embedded-kit", test))]
fn folder(
    server: &impl PluginFolders,
    active_endpoint: &str,
    requested_endpoint: &str,
    plugin: &str,
) -> Result<Option<String>, String> {
    if active_endpoint != requested_endpoint {
        return Err(NOT_EMBEDDED.into());
    }
    Ok(server.params(plugin).get("allowed_dir").cloned())
}
#[cfg(any(feature = "embedded-kit", test))]
fn save_folder(
    server: &impl PluginFolders,
    active_endpoint: &str,
    requested_endpoint: &str,
    plugin: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    folder_key(key)?;
    if active_endpoint != requested_endpoint {
        return Err(NOT_EMBEDDED.into());
    }
    server.set(plugin, key, value)
}

static ENDPOINT: Mutex<Option<String>> = Mutex::new(None);

pub fn endpoint() -> Option<String> {
    ENDPOINT.lock().unwrap().clone()
}

pub struct EmbeddedKitRuntime {
    #[cfg(feature = "embedded-kit")]
    server: Option<rune_kit::server::EmbeddedServer>,
    #[allow(dead_code)]
    pub endpoint: String,
}

#[derive(Default)]
pub struct EmbeddedKitManager(Arc<tokio::sync::Mutex<Option<EmbeddedKitRuntime>>>);

impl EmbeddedKitManager {
    pub async fn plugin_folder(
        &self,
        endpoint: String,
        plugin: String,
    ) -> Result<Option<String>, String> {
        #[cfg(feature = "embedded-kit")]
        {
            let runtime = self.0.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let runtime = runtime.blocking_lock();
                let active = runtime.as_ref().ok_or(NOT_EMBEDDED)?;
                let server = active.server.as_ref().ok_or(NOT_EMBEDDED)?;
                folder(server, &active.endpoint, &endpoint, &plugin)
            })
            .await
            .map_err(|error| error.to_string())?
        }
        #[cfg(not(feature = "embedded-kit"))]
        {
            let _ = (endpoint, plugin);
            Err(NOT_EMBEDDED.into())
        }
    }
    pub async fn set_plugin_folder(
        &self,
        endpoint: String,
        plugin: String,
        key: String,
        value: String,
    ) -> Result<(), String> {
        folder_key(&key)?;
        #[cfg(feature = "embedded-kit")]
        {
            let runtime = self.0.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let runtime = runtime.blocking_lock();
                let active = runtime.as_ref().ok_or(NOT_EMBEDDED)?;
                let server = active.server.as_ref().ok_or(NOT_EMBEDDED)?;
                save_folder(server, &active.endpoint, &endpoint, &plugin, &key, &value)
            })
            .await
            .map_err(|error| error.to_string())?
        }
        #[cfg(not(feature = "embedded-kit"))]
        {
            let _ = (endpoint, plugin, value);
            Err(NOT_EMBEDDED.into())
        }
    }
    #[allow(dead_code)]
    pub async fn start(&self) -> Result<String, String> {
        #[cfg_attr(not(feature = "embedded-kit"), allow(unused_mut))]
        let mut runtime = self.0.lock().await;
        if let Some(active) = runtime.as_ref() {
            return Ok(active.endpoint.clone());
        }

        #[cfg(feature = "embedded-kit")]
        {
            let bind: std::net::IpAddr = "127.0.0.1".parse().unwrap();
            let port = 47601;
            // Attempt to bind to default port 47601, or fallback to ephemeral port 0 if 47601 is in use
            let server = match rune_kit::server::start_default_embedded(bind, port).await {
                Ok(srv) => srv,
                Err(err) => {
                    log::warn!("Kit port 47601 unavailable ({err}), trying dynamic port...");
                    rune_kit::server::start_default_embedded(bind, 0).await?
                }
            };
            let endpoint = server.endpoint.clone();
            *ENDPOINT.lock().unwrap() = Some(endpoint.clone());
            *runtime = Some(EmbeddedKitRuntime {
                server: Some(server),
                endpoint: endpoint.clone(),
            });
            log::info!("Embedded Kit server started at {endpoint}");
            Ok(endpoint)
        }
        #[cfg(not(feature = "embedded-kit"))]
        {
            Err("Embedded Kit is not enabled in this build".into())
        }
    }

    pub async fn stop(&self) {
        let mut runtime = self.0.lock().await;
        if let Some(mut active) = runtime.take() {
            *ENDPOINT.lock().unwrap() = None;
            #[cfg(feature = "embedded-kit")]
            if let Some(server) = active.server.take() {
                let _ = server.shutdown().await;
            }
            #[cfg(not(feature = "embedded-kit"))]
            let _ = &mut active;
        }
    }
}

#[tauri::command]
pub async fn kit_plugin_folder(
    manager: tauri::State<'_, EmbeddedKitManager>,
    endpoint: String,
    plugin: String,
) -> Result<Option<String>, String> {
    manager.plugin_folder(endpoint, plugin).await
}
#[tauri::command]
pub async fn kit_set_plugin_folder(
    manager: tauri::State<'_, EmbeddedKitManager>,
    endpoint: String,
    plugin: String,
    key: String,
    value: String,
) -> Result<(), String> {
    manager
        .set_plugin_folder(endpoint, plugin, key, value)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        params: Mutex<std::collections::BTreeMap<String, String>>,
        calls: Mutex<Vec<(String, String, String)>>,
        error: Option<String>,
    }
    impl PluginFolders for Fake {
        fn params(&self, _: &str) -> std::collections::BTreeMap<String, String> {
            self.params.lock().unwrap().clone()
        }
        fn set(&self, plugin: &str, key: &str, value: &str) -> Result<(), String> {
            self.calls
                .lock()
                .unwrap()
                .push((plugin.into(), key.into(), value.into()));
            if let Some(error) = &self.error {
                return Err(error.clone());
            }
            self.params.lock().unwrap().insert(key.into(), value.into());
            Ok(())
        }
    }
    #[test]
    fn folder_commands_delegate_paths_and_errors_unchanged_to_kit() {
        let fake = Fake::default();
        assert_eq!(
            folder(&fake, "embedded", "embedded", "rune-filesystem").unwrap(),
            None
        );
        let path = r"D:\Chosen folder\..\Chosen folder";
        save_folder(
            &fake,
            "embedded",
            "embedded",
            "rune-filesystem",
            "allowed_dir",
            path,
        )
        .unwrap();
        assert_eq!(
            fake.calls.lock().unwrap()[0],
            ("rune-filesystem".into(), "allowed_dir".into(), path.into())
        );
        assert_eq!(
            folder(&fake, "embedded", "embedded", "rune-filesystem")
                .unwrap()
                .as_deref(),
            Some(path)
        );
        let fake = Fake {
            error: Some("Configuration is busy; retry".into()),
            ..Default::default()
        };
        assert_eq!(
            save_folder(
                &fake,
                "embedded",
                "embedded",
                "rune-filesystem",
                "allowed_dir",
                path
            )
            .unwrap_err(),
            "Configuration is busy; retry"
        );
    }
    #[test]
    fn only_allowed_dir_is_forwarded_and_external_endpoints_are_refused() {
        let fake = Fake::default();
        for key in ["allowedDir", "ALLOWED_DIR", "token", "", " allowed_dir"] {
            assert!(
                save_folder(
                    &fake,
                    "embedded",
                    "embedded",
                    "rune-filesystem",
                    key,
                    "path"
                )
                .is_err()
            );
        }
        assert_eq!(
            folder(&fake, "embedded", "external", "rune-filesystem").unwrap_err(),
            NOT_EMBEDDED
        );
        assert_eq!(
            save_folder(
                &fake,
                "embedded",
                "external",
                "rune-filesystem",
                "allowed_dir",
                "path"
            )
            .unwrap_err(),
            NOT_EMBEDDED
        );
        assert!(fake.calls.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn commands_without_an_embedded_server_return_the_disabled_reason() {
        let manager = EmbeddedKitManager::default();
        assert_eq!(
            manager
                .plugin_folder("external".into(), "rune-filesystem".into())
                .await
                .unwrap_err(),
            NOT_EMBEDDED
        );
        assert_eq!(
            manager
                .set_plugin_folder(
                    "external".into(),
                    "rune-filesystem".into(),
                    "allowed_dir".into(),
                    "path".into()
                )
                .await
                .unwrap_err(),
            NOT_EMBEDDED
        );
        assert!(
            manager
                .set_plugin_folder(
                    "external".into(),
                    "rune-filesystem".into(),
                    "token".into(),
                    "value".into()
                )
                .await
                .unwrap_err()
                .contains("Only allowed_dir")
        );
    }
}
