pub mod audit;
pub mod foreground;
pub mod gate;
pub mod rules;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    #[default]
    Low,
    Medium,
    High,
}

impl Risk {
    pub fn label(self) -> &'static str {
        match self {
            Risk::Low => "low",
            Risk::Medium => "medium",
            Risk::High => "high",
        }
    }

    pub fn parse(text: &str) -> Option<Risk> {
        match text.trim().to_ascii_lowercase().as_str() {
            "low" => Some(Risk::Low),
            "medium" => Some(Risk::Medium),
            "high" => Some(Risk::High),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub risk: Risk,
    pub reasons: Vec<String>,
}

impl Assessment {
    pub fn low() -> Self {
        Self {
            risk: Risk::Low,
            reasons: Vec::new(),
        }
    }

    pub fn raise(&mut self, risk: Risk, reason: impl Into<String>) {
        self.risk = self.risk.max(risk);
        let reason = reason.into();
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }
}

pub fn needs_confirmation(risk: Risk, ask_every_action: bool, harmless: bool) -> bool {
    if harmless {
        return false;
    }
    ask_every_action || risk == Risk::High
}
