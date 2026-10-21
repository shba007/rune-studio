use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(30);
pub const CAPABILITIES_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriState {
    Yes,
    No,
    Unknown,
}

impl TriState {
    pub fn is_yes(&self) -> bool {
        matches!(self, TriState::Yes)
    }

    pub fn is_no(&self) -> bool {
        matches!(self, TriState::No)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, TriState::Unknown)
    }

    pub fn from_opt_bool(opt: Option<bool>) -> Self {
        match opt {
            Some(true) => TriState::Yes,
            Some(false) => TriState::No,
            None => TriState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Locality {
    Local,
    Cloud,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    pub id: String,
    #[serde(default)]
    pub modalities_in: Vec<String>,
    #[serde(default)]
    pub modalities_out: Vec<String>,
    pub audio_in: TriState,
    pub image_in: TriState,
    pub speech_out: TriState,
    pub tool_calls: TriState,
    pub thinking_control: TriState,
    pub max_image_edge_px: Option<u32>,
    pub context_tokens: Option<u64>,
    pub locality: Locality,
    pub resident: TriState,
    pub vram_mb_estimate: Option<u64>,
    pub load_ms_estimate: Option<u64>,
    pub status: Option<String>,
    #[serde(default)]
    pub raw_capabilities: Vec<String>,
}

impl ModelCapabilities {
    pub fn unknown(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            modalities_in: Vec::new(),
            modalities_out: Vec::new(),
            audio_in: TriState::Unknown,
            image_in: TriState::Unknown,
            speech_out: TriState::Unknown,
            tool_calls: TriState::Unknown,
            thinking_control: TriState::Unknown,
            max_image_edge_px: None,
            context_tokens: None,
            locality: Locality::Unknown,
            resident: TriState::Unknown,
            vram_mb_estimate: None,
            load_ms_estimate: None,
            status: None,
            raw_capabilities: Vec::new(),
        }
    }
}

fn contains_ci(list: &[String], needle: &str) -> bool {
    list.iter().any(|s| s.eq_ignore_ascii_case(needle))
}

fn parse_tri_state(val: Option<&Value>) -> TriState {
    match val {
        Some(Value::Bool(b)) => TriState::from_opt_bool(Some(*b)),
        Some(Value::String(s)) => {
            if s.eq_ignore_ascii_case("yes") || s.eq_ignore_ascii_case("true") {
                TriState::Yes
            } else if s.eq_ignore_ascii_case("no") || s.eq_ignore_ascii_case("false") {
                TriState::No
            } else {
                TriState::Unknown
            }
        }
        _ => TriState::Unknown,
    }
}

pub fn parse_model_entry(entry: &Value) -> Option<ModelCapabilities> {
    let obj = entry.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();

    let raw_capabilities: Vec<String> = obj
        .get("capabilities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let modalities_in: Option<Vec<String>> = obj
        .get("modalities_in")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                .collect()
        });

    let modalities_out: Option<Vec<String>> = obj
        .get("modalities_out")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                .collect()
        });

    let audio_in = if let Some(ref in_mods) = modalities_in {
        if in_mods.iter().any(|m| m == "audio" || m == "voice") {
            TriState::Yes
        } else {
            TriState::No
        }
    } else if !raw_capabilities.is_empty() {
        if contains_ci(&raw_capabilities, "audio") || contains_ci(&raw_capabilities, "voice") {
            TriState::Yes
        } else {
            TriState::No
        }
    } else {
        TriState::Unknown
    };

    let image_in = if let Some(ref in_mods) = modalities_in {
        if in_mods.iter().any(|m| m == "image" || m == "vision") {
            TriState::Yes
        } else {
            TriState::No
        }
    } else if !raw_capabilities.is_empty() {
        if contains_ci(&raw_capabilities, "vision")
            || contains_ci(&raw_capabilities, "image")
            || contains_ci(&raw_capabilities, "omni")
        {
            TriState::Yes
        } else {
            TriState::No
        }
    } else {
        TriState::Unknown
    };

    let speech_out = if let Some(ref out_mods) = modalities_out {
        if out_mods
            .iter()
            .any(|m| m == "audio" || m == "speech" || m == "voice")
        {
            TriState::Yes
        } else {
            TriState::No
        }
    } else if !raw_capabilities.is_empty() {
        if contains_ci(&raw_capabilities, "speech")
            || contains_ci(&raw_capabilities, "audio_out")
            || contains_ci(&raw_capabilities, "voice_out")
        {
            TriState::Yes
        } else {
            TriState::Unknown
        }
    } else {
        TriState::Unknown
    };

    let tool_calls = if let Some(val) = obj.get("tool_calls") {
        parse_tri_state(Some(val))
    } else if !raw_capabilities.is_empty() {
        if contains_ci(&raw_capabilities, "tool call")
            || contains_ci(&raw_capabilities, "tools")
            || contains_ci(&raw_capabilities, "tool_calls")
        {
            TriState::Yes
        } else {
            TriState::No
        }
    } else {
        TriState::Unknown
    };

    let thinking_control = if let Some(val) = obj.get("thinking_control") {
        parse_tri_state(Some(val))
    } else if !raw_capabilities.is_empty() {
        if contains_ci(&raw_capabilities, "thinking") || contains_ci(&raw_capabilities, "reasoning")
        {
            TriState::Yes
        } else {
            TriState::Unknown
        }
    } else {
        TriState::Unknown
    };

    let max_image_edge_px = obj
        .get("max_image_edge_px")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);

    let context_tokens = obj.get("context_tokens").and_then(|v| v.as_u64());

    let locality = obj
        .get("locality")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.eq_ignore_ascii_case("local") {
                Locality::Local
            } else if s.eq_ignore_ascii_case("cloud") {
                Locality::Cloud
            } else {
                Locality::Unknown
            }
        })
        .unwrap_or(Locality::Unknown);

    let resident = parse_tri_state(obj.get("resident"));

    let vram_mb_estimate = obj.get("vram_mb_estimate").and_then(|v| v.as_u64());
    let load_ms_estimate = obj.get("load_ms_estimate").and_then(|v| v.as_u64());
    let status = obj
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(ModelCapabilities {
        id,
        modalities_in: modalities_in.unwrap_or_default(),
        modalities_out: modalities_out.unwrap_or_default(),
        audio_in,
        image_in,
        speech_out,
        tool_calls,
        thinking_control,
        max_image_edge_px,
        context_tokens,
        locality,
        resident,
        vram_mb_estimate,
        load_ms_estimate,
        status,
        raw_capabilities,
    })
}

pub fn parse_models_response(body: &str) -> Vec<ModelCapabilities> {
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let data = match parsed.get("data").and_then(|d| d.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    data.iter().filter_map(parse_model_entry).collect()
}

#[derive(Debug)]
pub struct CapabilityCache {
    ttl: Duration,
    entries: Mutex<Option<(Instant, HashMap<String, ModelCapabilities>)>>,
}

impl Default for CapabilityCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_TTL)
    }
}

impl CapabilityCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(None),
        }
    }

    pub fn get(&self) -> Option<HashMap<String, ModelCapabilities>> {
        let guard = self.entries.lock().ok()?;
        if let Some((instant, ref map)) = *guard {
            if instant.elapsed() < self.ttl {
                return Some(map.clone());
            }
        }
        None
    }

    pub fn get_model(&self, model_id: &str) -> Option<ModelCapabilities> {
        self.get().and_then(|map| map.get(model_id).cloned())
    }

    pub fn update(&self, list: Vec<ModelCapabilities>) {
        if let Ok(mut guard) = self.entries.lock() {
            let map = list.into_iter().map(|c| (c.id.clone(), c)).collect();
            *guard = Some((Instant::now(), map));
        }
    }

    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.entries.lock() {
            *guard = None;
        }
    }

    pub async fn fetch_cached(
        &self,
        client: &reqwest::Client,
        base_url: &str,
        api_key: Option<&str>,
    ) -> HashMap<String, ModelCapabilities> {
        if let Some(cached) = self.get() {
            return cached;
        }

        let base = base_url.trim_end_matches('/');
        let mut request = client
            .get(format!("{base}/v1/models"))
            .timeout(CAPABILITIES_TIMEOUT);
        if let Some(key) = api_key {
            request = request.bearer_auth(key);
        }

        match request.send().await {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.unwrap_or_default();
                let list = parse_models_response(&text);
                self.update(list.clone());
                list.into_iter().map(|c| (c.id.clone(), c)).collect()
            }
            _ => self
                .entries
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|(_, m)| m.clone()))
                .unwrap_or_default(),
        }
    }
}

pub type SharedCapabilityCache = Arc<CapabilityCache>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_todays_legacy_shape() {
        let fixture = r#"{
            "data": [
                { "id": "gemma-4-12b-it", "capabilities": ["Chat", "Vision", "Audio", "Tool Call"] },
                { "id": "plain-text", "capabilities": ["Chat"] }
            ]
        }"#;

        let models = parse_models_response(fixture);
        assert_eq!(models.len(), 2);

        let gemma = models.iter().find(|m| m.id == "gemma-4-12b-it").unwrap();
        assert_eq!(gemma.audio_in, TriState::Yes);
        assert_eq!(gemma.image_in, TriState::Yes);
        assert_eq!(gemma.tool_calls, TriState::Yes);
        assert_eq!(gemma.speech_out, TriState::Unknown);
        assert_eq!(gemma.locality, Locality::Unknown);
        assert_eq!(gemma.resident, TriState::Unknown);

        let plain = models.iter().find(|m| m.id == "plain-text").unwrap();
        assert_eq!(plain.audio_in, TriState::No);
        assert_eq!(plain.image_in, TriState::No);
        assert_eq!(plain.tool_calls, TriState::No);
        assert_eq!(plain.locality, Locality::Unknown);
    }

    #[test]
    fn parses_future_structured_shape() {
        let fixture = r#"{
            "data": [
                {
                    "id": "future-omni",
                    "modalities_in": ["text", "image", "audio"],
                    "modalities_out": ["text", "audio"],
                    "tool_calls": true,
                    "thinking_control": true,
                    "max_image_edge_px": 1920,
                    "context_tokens": 262144,
                    "locality": "local",
                    "resident": true,
                    "vram_mb_estimate": 16000,
                    "load_ms_estimate": 8000,
                    "status": "ready",
                    "unknown_future_field": "ignore me without error"
                }
            ]
        }"#;

        let models = parse_models_response(fixture);
        assert_eq!(models.len(), 1);
        let m = &models[0];
        assert_eq!(m.id, "future-omni");
        assert_eq!(m.audio_in, TriState::Yes);
        assert_eq!(m.image_in, TriState::Yes);
        assert_eq!(m.speech_out, TriState::Yes);
        assert_eq!(m.tool_calls, TriState::Yes);
        assert_eq!(m.thinking_control, TriState::Yes);
        assert_eq!(m.max_image_edge_px, Some(1920));
        assert_eq!(m.context_tokens, Some(262144));
        assert_eq!(m.locality, Locality::Local);
        assert_eq!(m.resident, TriState::Yes);
        assert_eq!(m.vram_mb_estimate, Some(16000));
        assert_eq!(m.load_ms_estimate, Some(8000));
        assert_eq!(m.status.as_deref(), Some("ready"));
    }

    #[test]
    fn parses_partial_shape_treating_missing_as_unknown() {
        let fixture = r#"{
            "data": [
                {
                    "id": "partial-model",
                    "locality": "cloud"
                }
            ]
        }"#;

        let models = parse_models_response(fixture);
        assert_eq!(models.len(), 1);
        let m = &models[0];
        assert_eq!(m.id, "partial-model");
        assert_eq!(m.locality, Locality::Cloud);
        assert_eq!(m.audio_in, TriState::Unknown);
        assert_eq!(m.image_in, TriState::Unknown);
        assert_eq!(m.speech_out, TriState::Unknown);
        assert_eq!(m.tool_calls, TriState::Unknown);
        assert_eq!(m.thinking_control, TriState::Unknown);
        assert_eq!(m.resident, TriState::Unknown);
        assert_eq!(m.max_image_edge_px, None);
    }

    #[test]
    fn parses_garbage_gracefully() {
        let fixtures = [
            "<html>502 Bad Gateway</html>",
            r#"{"data": "not an array"}"#,
            r#"{"something": 123}"#,
            r#"{"data": [{"missing_id": true}]}"#,
            "",
            "null",
        ];

        for fixture in fixtures {
            let models = parse_models_response(fixture);
            assert!(models.is_empty(), "expected empty for fixture: {fixture}");
        }
    }

    #[test]
    fn parses_empty_list() {
        let fixture = r#"{ "data": [] }"#;
        let models = parse_models_response(fixture);
        assert!(models.is_empty());
    }

    #[test]
    fn cache_ttl_and_invalidation() {
        let cache = CapabilityCache::new(Duration::from_millis(50));
        assert!(cache.get().is_none());

        cache.update(vec![ModelCapabilities::unknown("m1")]);
        assert!(cache.get().is_some());
        assert!(cache.get_model("m1").is_some());

        cache.invalidate();
        assert!(cache.get().is_none());

        cache.update(vec![ModelCapabilities::unknown("m2")]);
        assert!(cache.get().is_some());
        std::thread::sleep(Duration::from_millis(60));
        assert!(cache.get().is_none(), "cache should expire after TTL");
    }
}
