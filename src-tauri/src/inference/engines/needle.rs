use crate::inference::traits::InferenceEngine;
use crate::inference::types::{InferenceTaskRequest, InferenceTaskResponse};
use needle_infer::v3_engine::V3Engine;
use std::error::Error;
use std::path::Path;
use std::sync::Mutex;

pub struct NeedleEngine {
    inner: Mutex<V3Engine>,
}

impl NeedleEngine {
    pub fn load_from_candidates() -> Result<Self, Box<dyn Error>> {
        let candidate_paths = [
            "weights/needle3.cact",
            "src-tauri/weights/needle3.cact",
            "../weights/needle3.cact",
        ];

        for path in candidate_paths {
            if Path::new(path).exists() {
                println!("[NeedleEngine] Found weights at: {}", path);
                let engine = V3Engine::load(path)?;
                return Ok(Self {
                    inner: Mutex::new(engine),
                });
            }
        }

        let engine = V3Engine::load("weights/needle3.cact")?;
        Ok(Self {
            inner: Mutex::new(engine),
        })
    }

    fn extract_tool_call_json(raw: &str) -> String {
        if let Some(start) = raw.find("<tool_call>") {
            let content_start = start + "<tool_call>".len();
            if let Some(end) = raw[content_start..].find("</tool_call>") {
                return raw[content_start..content_start + end].trim().to_string();
            }
            return raw[content_start..].trim().to_string();
        }

        let mut cleaned = raw;
        if let Some(think_end) = raw.find("</think>") {
            cleaned = &raw[think_end + "</think>".len()..];
        }

        if let (Some(first_b), Some(last_b)) = (cleaned.find('['), cleaned.rfind(']')) {
            if first_b < last_b {
                return cleaned[first_b..=last_b].trim().to_string();
            }
        }
        if let (Some(first_b), Some(last_b)) = (cleaned.find('{'), cleaned.rfind('}')) {
            if first_b < last_b {
                return cleaned[first_b..=last_b].trim().to_string();
            }
        }

        cleaned.trim().to_string()
    }
}

impl InferenceEngine for NeedleEngine {
    fn id(&self) -> &'static str {
        "cactus-needle-3"
    }

    fn execute(
        &self,
        task: &InferenceTaskRequest,
    ) -> Result<InferenceTaskResponse, Box<dyn Error>> {
        match task {
            InferenceTaskRequest::ToolCall { prompt, schema } => {
                let engine = self
                    .inner
                    .lock()
                    .map_err(|e| format!("Lock error: {}", e))?;
                let schema_str = schema.to_string();

                println!("[NeedleEngine] Prompt: \"{}\" \"{}\"", prompt, &schema_str);
                let raw_output = engine.run(prompt, &schema_str);
                println!("[NeedleEngine] Raw output:\n{}", raw_output);

                let clean_json = Self::extract_tool_call_json(&raw_output);
                println!("[NeedleEngine] Clean JSON:\n{}", clean_json);

                let parsed: serde_json::Value = serde_json::from_str(&clean_json)?;
                Ok(InferenceTaskResponse::ToolCall(parsed))
            }
            _ => Err("NeedleEngine only supports ToolCall tasks".into()),
        }
    }
}
