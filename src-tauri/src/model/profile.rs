use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::capabilities::{Locality, ModelCapabilities, TriState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingStyle {
    ThinkingType,
    ChatTemplateKwargs,
    BudgetTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingSwitch {
    None,
    Styles(Vec<ThinkingStyle>),
}

impl Default for ThinkingSwitch {
    fn default() -> Self {
        ThinkingSwitch::Styles(vec![
            ThinkingStyle::ThinkingType,
            ThinkingStyle::ChatTemplateKwargs,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInProfile {
    pub supported: TriState,
    pub format: String,
}

impl Default for AudioInProfile {
    fn default() -> Self {
        Self {
            supported: TriState::Unknown,
            format: "wav".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagesProfile {
    pub max_edge_px: u32,
    pub screenshot_at_app_size: bool,
}

impl Default for ImagesProfile {
    fn default() -> Self {
        Self {
            max_edge_px: 1280,
            screenshot_at_app_size: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PointingConvention {
    Norm1000,
    Pixels,
    Norm1,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleMaxTokens {
    pub agent: u32,
    pub agent_thinking: u32,
    pub verifier: u32,
    pub analysis: u32,
    pub voice: u32,
    pub decision: u32,
    pub advisor: u32,
}

impl Default for RoleMaxTokens {
    fn default() -> Self {
        Self {
            agent: 512,
            agent_thinking: 3000,
            verifier: 512,
            analysis: 1500,
            voice: 256,
            decision: 512,
            advisor: 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProfile {
    pub id: String,
    pub thinking: ThinkingSwitch,
    pub audio_in: AudioInProfile,
    pub images: ImagesProfile,
    pub pointing: PointingConvention,
    pub tool_choice_required: TriState,
    pub max_tokens: RoleMaxTokens,
    pub locality: Locality,
}

impl ModelProfile {
    pub fn conservative(id: &str) -> Self {
        Self {
            id: id.to_string(),
            thinking: ThinkingSwitch::Styles(vec![
                ThinkingStyle::ThinkingType,
                ThinkingStyle::ChatTemplateKwargs,
            ]),
            audio_in: AudioInProfile {
                supported: TriState::Unknown,
                format: "wav".into(),
            },
            images: ImagesProfile {
                max_edge_px: 1280,
                screenshot_at_app_size: true,
            },
            pointing: PointingConvention::Unknown,
            tool_choice_required: TriState::Unknown,
            max_tokens: RoleMaxTokens::default(),
            locality: Locality::Unknown,
        }
    }
}

const BUNDLED_PROFILES_JSON: &str = include_str!("profiles.json");

pub fn bundled_profiles() -> HashMap<String, ModelProfile> {
    serde_json::from_str(BUNDLED_PROFILES_JSON).unwrap_or_default()
}

pub fn resolve_profile(
    model_id: &str,
    user_override: Option<&ModelProfile>,
    capabilities: Option<&ModelCapabilities>,
) -> ModelProfile {
    let mut profile = ModelProfile::conservative(model_id);

    if let Some(caps) = capabilities {
        if caps.locality != Locality::Unknown {
            profile.locality = caps.locality;
        }
        if caps.audio_in != TriState::Unknown {
            profile.audio_in.supported = caps.audio_in;
        }
        if caps.tool_calls != TriState::Unknown {
            profile.tool_choice_required = caps.tool_calls;
        }
        if let Some(max_edge) = caps.max_image_edge_px {
            profile.images.max_edge_px = max_edge;
        }
        if caps.thinking_control == TriState::No {
            profile.thinking = ThinkingSwitch::None;
        }
    }

    let bundled = bundled_profiles();
    if let Some(measured) = bundled.get(model_id) {
        profile = measured.clone();
    }

    if let Some(user) = user_override
        && (user.id == model_id || user.id.is_empty())
    {
        profile = user.clone();
        profile.id = model_id.to_string();
    }

    // An explicit server declaration must suppress unsupported switches even for bundled profiles.
    if capabilities.is_some_and(|caps| caps.thinking_control == TriState::No) {
        profile.thinking = ThinkingSwitch::None;
    }
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_defaults_for_unknown_model() {
        let p = ModelProfile::conservative("unknown-model-xyz");
        assert_eq!(p.id, "unknown-model-xyz");
        assert_eq!(
            p.thinking,
            ThinkingSwitch::Styles(vec![
                ThinkingStyle::ThinkingType,
                ThinkingStyle::ChatTemplateKwargs,
            ])
        );
        assert_eq!(p.pointing, PointingConvention::Unknown);
        assert_eq!(p.tool_choice_required, TriState::Unknown);
        assert_eq!(p.audio_in.supported, TriState::Unknown);
        assert_eq!(p.locality, Locality::Unknown);
        assert_eq!(p.images.max_edge_px, 1280);
    }

    #[test]
    fn resolves_measured_bundled_profile_for_gemma() {
        let p = resolve_profile("gemma-4-12b-it", None, None);
        assert_eq!(p.id, "gemma-4-12b-it");
        assert_eq!(p.pointing, PointingConvention::Norm1000);
        assert_eq!(p.tool_choice_required, TriState::Yes);
        assert_eq!(p.audio_in.supported, TriState::Yes);
        assert_eq!(p.locality, Locality::Local);
    }

    #[test]
    fn capabilities_overlay_on_conservative() {
        let mut caps = ModelCapabilities::unknown("custom-cloud-model");
        caps.locality = Locality::Cloud;
        caps.tool_calls = TriState::Yes;
        caps.max_image_edge_px = Some(1920);

        let p = resolve_profile("custom-cloud-model", None, Some(&caps));
        assert_eq!(p.locality, Locality::Cloud);
        assert_eq!(p.tool_choice_required, TriState::Yes);
        assert_eq!(p.images.max_edge_px, 1920);
        assert_eq!(p.pointing, PointingConvention::Unknown);
    }

    #[test]
    fn user_override_takes_highest_precedence() {
        let mut user = ModelProfile::conservative("gemma-4-12b-it");
        user.pointing = PointingConvention::Pixels;
        user.locality = Locality::Cloud;

        let p = resolve_profile("gemma-4-12b-it", Some(&user), None);
        assert_eq!(p.pointing, PointingConvention::Pixels);
        assert_eq!(p.locality, Locality::Cloud);
    }

    #[test]
    fn only_measured_model_in_bundled_profiles() {
        let bundled = bundled_profiles();
        assert_eq!(bundled.len(), 1);
        assert!(bundled.contains_key("gemma-4-12b-it"));
    }
}
