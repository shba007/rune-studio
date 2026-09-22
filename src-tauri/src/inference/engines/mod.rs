pub mod needle;
pub mod qwen;

use crate::inference::traits::InferenceEngine;
use std::sync::Arc;

pub struct DiscoveredEngine {
    pub engine: Arc<dyn InferenceEngine>,
    pub priority: u32, // Higher priority = chosen as default for its task type
    pub is_tool_caller: bool,
}

pub type EngineLoader = fn() -> Option<DiscoveredEngine>;

/// Master catalog of all engine loaders.
/// Add new model providers here without touching lib.rs.
pub const ENGINE_PROVIDERS: &[EngineLoader] = &[
    // 1. Qwen3 / GGUF Models (High priority for tool calling)
    || {
        qwen::QwenEngine::load_from_candidates()
            .ok()
            .map(|e| DiscoveredEngine {
                engine: Arc::new(e),
                priority: 100, // Highest priority: chosen first if weights exist
                is_tool_caller: true,
            })
    },
    // 2. Needle 3 / .cact Models (Compact fallback for tool calling)
    || {
        needle::NeedleEngine::load_from_candidates()
            .ok()
            .map(|e| DiscoveredEngine {
                engine: Arc::new(e),
                priority: 50, // Lower priority: acts as fallback if Qwen is missing
                is_tool_caller: true,
            })
    },
    // Future engines (e.g., Diffusion, TTS, LLM Chat) are plugged in here:
    // || diffusion::DiffusionEngine::load().ok().map(...),
    // || whisper::WhisperEngine::load().ok().map(...),
];
