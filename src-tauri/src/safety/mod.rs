//! The safety gate: nothing risky happens without the user's yes.
//!
//! Every action, whoever chose it (the built-in model or an outside agent), is assessed here
//! before it runs. The final risk is the higher of what the agent said and what our own rules
//! find. High risk pauses for the user; with "Ask before every action" on, everything does.

pub mod audit;
pub mod foreground;
pub mod gate;
pub mod rules;

use serde::{Deserialize, Serialize};

/// How much harm an action could do. Ordered, so `max` picks the worse of two opinions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    #[default]
    Low,
    Medium,
    High,
}

impl Risk {
    /// The word the record and the model protocol use for it.
    pub fn label(self) -> &'static str {
        match self {
            Risk::Low => "low",
            Risk::Medium => "medium",
            Risk::High => "high",
        }
    }

    /// Reads what an agent wrote in its `risk` argument. Anything unrecognised counts as unstated.
    pub fn parse(text: &str) -> Option<Risk> {
        match text.trim().to_ascii_lowercase().as_str() {
            "low" => Some(Risk::Low),
            "medium" => Some(Risk::Medium),
            "high" => Some(Risk::High),
            _ => None,
        }
    }
}

/// What we found out about one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub risk: Risk,
    /// Short, plain reasons, shown to the user next to the question.
    pub reasons: Vec<String>,
}

impl Assessment {
    pub fn low() -> Self {
        Self {
            risk: Risk::Low,
            reasons: Vec::new(),
        }
    }

    /// Records a finding. The risk only ever goes up.
    pub fn raise(&mut self, risk: Risk, reason: impl Into<String>) {
        self.risk = self.risk.max(risk);
        let reason = reason.into();
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }
}

/// Whether the user must say yes first.
pub fn needs_confirmation(risk: Risk, ask_every_action: bool, harmless: bool) -> bool {
    if harmless {
        return false;
    }
    ask_every_action || risk == Risk::High
}
