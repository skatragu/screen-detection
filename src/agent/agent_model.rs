use std::collections::HashSet;

use crate::{
    canonical::diff::{SemanticSignal, SemanticStateDiff},
    state::state_model::ScreenState,
};

pub const MIN_CONFIDENCE: f32 = 0.65;
pub const MAX_RETRIES: u32 = 3;
pub const MAX_LOOP_REPEATS: u32 = 2;
pub const MAX_THINK_STEPS: u32 = 5;

#[derive(Debug, Clone)]
pub enum AgentState {
    Observe,
    Evaluate,
    Think,
    Act,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    FillInput {
        form_id: String,
        input_label: String,
        value: String,
        identity: Option<String>,
    },
    SubmitForm {
        form_id: String,
        action_label: String,
        identity: Option<String>,
    },
    FormSubmitted {
        form_id: String,
    },
    FillAndSubmitForm {
        form_id: String,
        values: Vec<(String, String)>,     // (input_label, value) pairs
        submit_label: Option<String>,      // label of the action to click after filling
    },
    ClickAction {
        label: String,
        identity: Option<String>,
    },
    Wait {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub struct AgentMemory {
    pub last_action: Option<AgentAction>,
    pub last_confirmed_action: Option<AgentAction>,
    pub attempt_count: u32,
    pub last_signal: Option<SemanticSignal>,
    pub loop_count: u32,

    // ---- Budgets ----
    pub think_budget_remaining: u32, // max model calls
    pub retry_budget_remaining: u32, // retries for same action
    pub loop_budget_remaining: u32,  // allowed repeated no-progress loops

    pub suppressed_identities: std::collections::HashSet<String>,
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self {
            last_action: None,
            last_confirmed_action: None,

            think_budget_remaining: MAX_THINK_STEPS,
            retry_budget_remaining: MAX_RETRIES,
            loop_budget_remaining: MAX_LOOP_REPEATS,

            loop_count: 0,
            attempt_count: 0,
            last_signal: None,
            suppressed_identities: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelDecision {
    pub decision: DecisionType,
    pub next_action: Option<AgentAction>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum DecisionType {
    Act,
    Wait,
    Stop,
}

pub trait Policy {
    fn decide(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        memory: &AgentMemory,
    ) -> Option<ModelDecision>;
}
