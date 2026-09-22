use crate::inference::engines::ENGINE_PROVIDERS;
use crate::inference::traits::InferenceEngine;
use crate::inference::types::InferenceTaskRequest;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ModelRegistry {
    engines: HashMap<String, Arc<dyn InferenceEngine>>,
    default_tool_engine: Option<String>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
            default_tool_engine: None,
        }
    }

    /// Automatically scans all registered engine providers and initializes
    /// whatever model weights are found on disk.
    pub fn auto_discover() -> Self {
        let mut registry = Self::new();
        let mut best_tool_priority = 0;
        let mut best_tool_engine_id = None;

        println!("[ModelRegistry] Probing for available model weights...");

        for loader in ENGINE_PROVIDERS {
            if let Some(discovered) = loader() {
                let id = discovered.engine.id().to_string();
                println!("[ModelRegistry] ✓ Loaded and registered engine: \"{}\"", id);

                if discovered.is_tool_caller && discovered.priority > best_tool_priority {
                    best_tool_priority = discovered.priority;
                    best_tool_engine_id = Some(id.clone());
                }

                registry.engines.insert(id, discovered.engine);
            }
        }

        if let Some(default_id) = best_tool_engine_id {
            println!(
                "[ModelRegistry] → Active primary tool engine set to: \"{}\"",
                default_id
            );
            registry.default_tool_engine = Some(default_id);
        } else {
            println!("[ModelRegistry] ⚠ Warning: No valid model weights found in weights/ folder.");
        }

        registry
    }

    pub fn register(&mut self, engine: Arc<dyn InferenceEngine>, is_default_for_tools: bool) {
        let id = engine.id().to_string();
        if is_default_for_tools {
            self.default_tool_engine = Some(id.clone());
        }
        self.engines.insert(id, engine);
    }

    pub fn resolve_engine(
        &self,
        preferred_id: Option<&str>,
        task: &InferenceTaskRequest,
    ) -> Option<Arc<dyn InferenceEngine>> {
        if let Some(id) = preferred_id {
            return self.engines.get(id).cloned();
        }

        match task {
            InferenceTaskRequest::ToolCall { .. } => {
                let default_id = self.default_tool_engine.as_ref()?;
                self.engines.get(default_id).cloned()
            }
            _ => None,
        }
    }
}
