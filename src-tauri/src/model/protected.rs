//! Resolve server attribution before sending an input that may contain sensitive data.
use super::{
    capabilities::{Locality, ModelCapabilities, parse_models_response},
    profile::{ModelProfile, resolve_profile},
};

fn local_client(base: &str) -> Result<reqwest::Client, String> {
    if !crate::life::policy::parse_and_validate_loopback_host(base).unwrap_or(false) {
        return Err("Protected classification requires a loopback model server".into());
    }
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())
}

fn supported_contract(value: &serde_json::Value) -> bool {
    let contract = &value["local_execution"];
    contract["version"] == 1
        && contract["request_header"] == "x-rune-local-only"
        && contract["request_value"] == "1"
        && contract["endpoints"].as_array().is_some_and(|endpoints| {
            endpoints.iter().any(|endpoint| {
                endpoint["method"] == "POST" && endpoint["path"] == "/v1/chat/completions"
            })
        })
}

pub async fn verify_local_execution(base: &str, key: Option<&str>) -> Result<(), String> {
    let http = local_client(base)?;
    let mut request = http.get(format!("{}/v1/capabilities", base.trim_end_matches('/')));
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Could not verify the local execution contract".to_string())?;
    if !response.status().is_success() {
        return Err("Model server does not advertise the required local execution contract".into());
    }
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|_| "Invalid local execution capability response".to_string())?;
    if !supported_contract(&value) {
        return Err("Model server does not advertise local execution contract version 1 for chat completions".into());
    }
    Ok(())
}

pub async fn discover_classification(
    base: &str,
    key: Option<&str>,
    model: &str,
) -> Result<ModelProfile, String> {
    let http = local_client(base)?;
    let mut request = http.get(format!("{}/v1/models", base.trim_end_matches('/')));
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err("Could not verify the local model catalog".into());
    }
    let models = parse_models_response(&response.text().await.map_err(|e| e.to_string())?);
    resolve_classification(model, &models)
}

fn resolve_classification(
    model: &str,
    models: &[ModelCapabilities],
) -> Result<ModelProfile, String> {
    let caps = models
        .iter()
        .find(|caps| caps.id == model)
        .ok_or("Classification requires an exact advertised model ID")?;
    if caps.locality != Locality::Local {
        return Err("Classification requires a model advertised as local".into());
    }
    if !caps.supports_structured_output() {
        return Err(format!(
            "Agent model '{model}' does not advertise structured output, which is required to process requests. In Settings > Models, select a local Agent model with structured output; you can keep this model as the Voice model."
        ));
    }
    resolve(model, models)
}

pub fn resolve(model: &str, models: &[ModelCapabilities]) -> Result<ModelProfile, String> {
    let caps = models
        .iter()
        .find(|c| c.id == model || c.aliases.iter().any(|alias| alias == model))
        .ok_or("The selected model was not advertised by Infer")?;
    let mut profile = resolve_profile(model, None, Some(caps));
    // Explicit attribution must win over a bundled profile for another deployment.
    if caps.locality != Locality::Unknown {
        profile.locality = caps.locality;
    }
    if profile.locality != Locality::Local {
        return Err("Goal and classification requests require a model verified as local. Check Infer's model locality in Settings.".into());
    }
    Ok(profile)
}

pub async fn discover(
    http: &reqwest::Client,
    base: &str,
    key: Option<&str>,
    model: &str,
) -> Result<ModelProfile, String> {
    discover_with_capabilities(http, base, key, model)
        .await
        .map(|(profile, _)| profile)
}

pub async fn discover_with_capabilities(
    http: &reqwest::Client,
    base: &str,
    key: Option<&str>,
    model: &str,
) -> Result<(ModelProfile, ModelCapabilities), String> {
    let mut request = http
        .get(format!("{}/v1/models", base.trim_end_matches('/')))
        .timeout(std::time::Duration::from_secs(4));
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let body = response.text().await.map_err(|e| e.to_string())?;
    let models = parse_models_response(&body);
    let profile = resolve(model, &models)?;
    let caps = models
        .into_iter()
        .find(|item| item.id == model || item.aliases.iter().any(|alias| alias == model))
        .ok_or("The selected model was not advertised by Infer")?;
    Ok((profile, caps))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_exact_contract_version_header_and_chat_coverage() {
        let valid = serde_json::json!({"local_execution":{"version":1,"request_header":"x-rune-local-only","request_value":"1","endpoints":[{"method":"POST","path":"/v1/chat/completions"}]}});
        assert!(supported_contract(&valid));
        for invalid in [
            serde_json::json!({}),
            serde_json::json!({"local_execution":{"version":2}}),
            serde_json::json!({"local_execution":{"version":1,"request_header":"x-rune-local-only","request_value":"1","endpoints":[{"method":"POST","path":"/v1/responses"}]}}),
        ] {
            assert!(!supported_contract(&invalid));
        }
    }
    #[test]
    fn classification_requires_exact_id_and_explicit_locality() {
        let mut model = ModelCapabilities::unknown("gemma-4-12b-it");
        model.aliases = vec!["alias".into()];
        assert!(resolve_classification("gemma-4-12b-it", &[model.clone()]).is_err());
        model.locality = Locality::Local;
        assert!(resolve_classification("alias", &[model.clone()]).is_err());
        let error = resolve_classification("gemma-4-12b-it", &[model.clone()]).unwrap_err();
        assert!(error.contains("Settings > Models"));
        assert!(error.contains("structured output"));
        model.raw_capabilities.push("structured json".into());
        assert!(resolve_classification("gemma-4-12b-it", &[model]).is_ok());
    }
    #[test]
    fn classification_accepts_infer_structured_capability_names() {
        for capability in [
            "Structured Output",
            "Structured JSON",
            "Structured Output Mode",
            "Structured Extraction",
        ] {
            let mut model = ModelCapabilities::unknown("agent");
            model.locality = Locality::Local;
            model.raw_capabilities.push(capability.into());
            assert!(resolve_classification("agent", &[model]).is_ok());
        }
    }
    #[test]
    fn local_unbundled_models_work_and_explicit_cloud_cannot_inherit_bundled_trust() {
        let mut model = ModelCapabilities::unknown("new-qwen");
        model.locality = Locality::Local;
        model.aliases = vec!["alias".into()];
        assert_eq!(
            resolve("alias", &[model]).unwrap().locality,
            Locality::Local
        );
        let mut model = ModelCapabilities::unknown("gemma-4-12b-it");
        model.locality = Locality::Cloud;
        assert!(resolve("gemma-4-12b-it", &[model]).is_err());
        assert!(
            resolve(
                "unverified-model",
                &[ModelCapabilities::unknown("unverified-model")]
            )
            .is_err()
        );
    }
}
