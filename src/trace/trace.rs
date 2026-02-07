use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    agent::agent_model::{AgentAction, AgentState},
    canonical::diff::SemanticSignal,
};

#[derive(Debug, Serialize)]
pub struct TraceEvent {
    pub timestamp_ms: u128,
    pub step: u64,

    pub agent_state: String,

    pub signals: Vec<String>,

    pub decision: Option<String>,
    pub action: Option<String>,

    pub confidence: Option<f32>,
    pub suppression_reason: Option<String>,
}

impl TraceEvent {
    pub fn now(step: u64, state: &AgentState) -> Self {
        Self {
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            step,
            agent_state: format!("{:?}", state),
            signals: vec![],
            decision: None,
            action: None,
            confidence: None,
            suppression_reason: None,
        }
    }

    pub fn with_signals(mut self, signals: &[SemanticSignal]) -> Self {
        self.signals = signals.iter().map(|s| format!("{:?}", s)).collect();
        self
    }

    pub fn with_decision(mut self, decision: impl ToString) -> Self {
        self.decision = Some(decision.to_string());
        self
    }

    pub fn with_action(mut self, action: &AgentAction) -> Self {
        self.action = Some(format!("{:?}", action));
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_suppression(mut self, reason: impl ToString) -> Self {
        self.suppression_reason = Some(reason.to_string());
        self
    }
}
