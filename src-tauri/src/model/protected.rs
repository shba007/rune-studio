//! Resolve server attribution before sending an input that may contain sensitive data.
use super::{
    capabilities::{Locality, ModelCapabilities, parse_models_response},
    profile::{ModelProfile, resolve_profile},
};

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
    resolve(model, &parse_models_response(&body))
}

#[cfg(test)]
mod tests {
    use super::*;
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
