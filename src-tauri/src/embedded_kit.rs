use std::sync::Mutex;

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
pub struct EmbeddedKitManager(tokio::sync::Mutex<Option<EmbeddedKitRuntime>>);

impl EmbeddedKitManager {
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
