use std::error::Error;
use super::types::{InferenceTaskRequest, InferenceTaskResponse};

pub trait InferenceEngine: Send + Sync {
    /// Unique identifier for this model engine (e.g., "cactus-needle-3")
    fn id(&self) -> &'static str;

    /// Runs the task through the engine
    fn execute(&self, task: &InferenceTaskRequest) -> Result<InferenceTaskResponse, Box<dyn Error>>;
}