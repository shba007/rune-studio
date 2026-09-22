use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InferenceTaskRequest {
    /// Structured tool execution (Needle 3)
    ToolCall {
        prompt: String,
        schema: serde_json::Value,
    },
    /// Text chat / completion (LLMs like Llama, Mistral, Qwen)
    Chat {
        messages: Vec<ChatMessage>,
    },
    /// Image synthesis (Stable Diffusion, Flux)
    GenerateImage {
        prompt: String,
        width: Option<u32>,
        height: Option<u32>,
    },
    /// Audio / Speech synthesis (TTS engines)
    GenerateAudio {
        text: String,
        voice_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum InferenceTaskResponse {
    ToolCall(serde_json::Value),
    Text(String),
    ImageBase64(String),
    AudioBuffer(Vec<u8>),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}