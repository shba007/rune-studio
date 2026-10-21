use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelRole {
    Agent,
    Verifier,
    Analysis,
    Voice,
    Decision,
    Advisor,
}

impl ModelRole {
    pub fn all() -> &'static [ModelRole] {
        &[
            ModelRole::Agent,
            ModelRole::Verifier,
            ModelRole::Analysis,
            ModelRole::Voice,
            ModelRole::Decision,
            ModelRole::Advisor,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ModelRole::Agent => "agent",
            ModelRole::Verifier => "verifier",
            ModelRole::Analysis => "analysis",
            ModelRole::Voice => "voice",
            ModelRole::Decision => "decision",
            ModelRole::Advisor => "advisor",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelRole::Agent => "Agent",
            ModelRole::Verifier => "Verifier",
            ModelRole::Analysis => "Screen Analysis",
            ModelRole::Voice => "Voice",
            ModelRole::Decision => "Decision",
            ModelRole::Advisor => "Advisor",
        }
    }
}
