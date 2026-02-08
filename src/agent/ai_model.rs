use crate::{
    agent::agent_model::{
        AgentAction, AgentMemory, DecisionType, MIN_CONFIDENCE, ModelDecision, Policy,
    },
    canonical::diff::SemanticSignal,
    canonical::diff::SemanticStateDiff,
    state::state_model::ScreenState,
};
use serde::{Deserialize, Serialize};

pub struct ModelPolicy {
    pub model: Box<dyn ModelBackend>,
}

impl Policy for ModelPolicy {
    fn decide(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        memory: &AgentMemory,
    ) -> Option<ModelDecision> {
        self.model.infer(screen, diff, memory)
    }
}

pub trait ModelBackend {
    fn infer(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        memory: &AgentMemory,
    ) -> Option<ModelDecision>;
}

pub struct DeterministicPolicy;

/// Derive a sensible fill value from the input's label and type.
pub fn guess_value(label: &str, input_type: Option<&str>) -> String {
    let l = label.to_lowercase();

    // Label-based heuristics (checked in order)
    if l.contains("email") {
        return "user@example.com".into();
    }
    if l.contains("password") {
        return "TestPass123!".into();
    }
    if l.contains("phone") || l.contains("tel") {
        return "555-0100".into();
    }
    if l.contains("url") || l.contains("website") {
        return "https://example.com".into();
    }
    if l.contains("zip") || l.contains("postal") {
        return "90210".into();
    }
    if l.contains("username") || l.contains("user") {
        return "testuser".into();
    }
    if l.contains("name") {
        return "Jane Doe".into();
    }
    if l.contains("search") || l.contains("query") {
        return "test query".into();
    }
    if l.contains("date") {
        return "2025-01-15".into();
    }
    if l.contains("number") || l.contains("amount") || l.contains("quantity") {
        return "42".into();
    }

    // Fallback to input_type
    if let Some(t) = input_type {
        match t {
            "email" => return "user@example.com".into(),
            "password" => return "TestPass123!".into(),
            "tel" => return "555-0100".into(),
            "url" => return "https://example.com".into(),
            "number" => return "42".into(),
            "date" => return "2025-01-15".into(),
            _ => {}
        }
    }

    "test".into()
}

impl Policy for DeterministicPolicy {
    fn decide(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        _memory: &AgentMemory,
    ) -> Option<ModelDecision> {
        let signal = diff.signals.last()?;

        let (decision_type, action) = match signal {
            SemanticSignal::ScreenLoaded => {
                let form = screen.forms.first()?;

                let values: Vec<(String, String)> = form
                    .inputs
                    .iter()
                    .map(|input| {
                        let label = input.label.clone().unwrap_or_default();
                        let value = guess_value(&label, input.input_type.as_deref());
                        (label, value)
                    })
                    .collect();

                let submit_label = form
                    .primary_action
                    .as_ref()
                    .or(form.actions.first())
                    .and_then(|a| a.label.clone());

                (
                    DecisionType::Act,
                    Some(AgentAction::FillAndSubmitForm {
                        form_id: form.id.clone(),
                        values,
                        submit_label,
                    }),
                )
            }

            SemanticSignal::FormSubmitted { form_id } => (
                DecisionType::Wait,
                Some(AgentAction::Wait {
                    reason: format!("Waiting after submitting {}", form_id),
                }),
            ),

            SemanticSignal::ResultsAppeared => (
                DecisionType::Wait,
                Some(AgentAction::Wait {
                    reason: "Results appeared".to_string(),
                }),
            ),

            SemanticSignal::NavigationOccurred => (
                DecisionType::Wait,
                Some(AgentAction::Wait {
                    reason: "Navigation occurred".to_string(),
                }),
            ),

            SemanticSignal::ErrorAppeared => (
                DecisionType::Wait,
                Some(AgentAction::Wait {
                    reason: "Error appeared".to_string(),
                }),
            ),

            SemanticSignal::NoOp => return None,
        };

        Some(ModelDecision {
            decision: decision_type,
            next_action: action,
            confidence: 0.9,
        })
    }
}

pub struct HybridPolicy {
    pub deterministic: DeterministicPolicy,
    pub model: ModelPolicy,
}

impl Policy for HybridPolicy {
    fn decide(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        memory: &AgentMemory,
    ) -> Option<ModelDecision> {
        // Try deterministic first
        if let Some(decision) = self.deterministic.decide(screen, diff, memory) {
            if decision.confidence >= MIN_CONFIDENCE {
                return Some(decision);
            }
        }

        // Fall back to model
        self.model.decide(screen, diff, memory)
    }
}

// ============================================================================
// Ollama Backend
// ============================================================================

pub struct OllamaBackend {
    pub endpoint: String,
    pub model: String,
}

impl Default for OllamaBackend {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434/api/generate".to_string(),
            model: "qwen2.5:1.5b".to_string(),
        }
    }
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: &'static str,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize)]
struct ModelActionResponse {
    action: String,
    #[serde(default)]
    form_id: Option<String>,
    #[serde(default)]
    input_label: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    action_label: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

impl OllamaBackend {
    pub fn new(endpoint: &str, model: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            model: model.to_string(),
        }
    }

    fn build_prompt(&self, screen: &ScreenState, diff: &SemanticStateDiff, memory: &AgentMemory) -> String {
        let forms_summary = screen
            .forms
            .iter()
            .map(|f| {
                let inputs: Vec<_> = f.inputs.iter()
                    .filter_map(|i| i.label.clone())
                    .collect();
                let actions: Vec<_> = f.actions.iter()
                    .filter_map(|a| a.label.clone())
                    .collect();
                format!(
                    "  - Form '{}': inputs=[{}], actions=[{}]",
                    f.id,
                    inputs.join(", "),
                    actions.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let outputs_summary = screen
            .outputs
            .iter()
            .filter_map(|o| o.label.clone())
            .take(5)
            .collect::<Vec<_>>()
            .join("; ");

        let signal = diff.signals.last()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "None".to_string());

        let last_action = memory.last_action.as_ref()
            .map(|a| format!("{:?}", a))
            .unwrap_or_else(|| "None".to_string());

        format!(
r#"You are a web automation agent. Decide the next action based on the screen state.

SCREEN STATE:
- URL: {}
- Title: {}
- Forms:
{}
- Outputs: {}
- Last signal: {}
- Last action: {}

AVAILABLE ACTIONS (respond with exactly one as JSON):
1. FillInput: {{"action":"FillInput","form_id":"...","input_label":"...","value":"...","confidence":0.9}}
2. SubmitForm: {{"action":"SubmitForm","form_id":"...","action_label":"...","confidence":0.9}}
3. ClickAction: {{"action":"ClickAction","label":"...","confidence":0.9}}
4. Wait: {{"action":"Wait","reason":"...","confidence":0.9}}

Respond with ONLY valid JSON, no explanation."#,
            screen.url.as_deref().unwrap_or("unknown"),
            screen.title,
            if forms_summary.is_empty() { "  (none)" } else { &forms_summary },
            if outputs_summary.is_empty() { "(none)" } else { &outputs_summary },
            signal,
            last_action
        )
    }

    fn parse_response(&self, response: &str) -> Option<ModelDecision> {
        let parsed: ModelActionResponse = serde_json::from_str(response).ok()?;
        let confidence = parsed.confidence.unwrap_or(0.7);

        let action = match parsed.action.as_str() {
            "FillInput" => AgentAction::FillInput {
                form_id: parsed.form_id?,
                input_label: parsed.input_label?,
                value: parsed.value.unwrap_or_default(),
                identity: None,
            },
            "SubmitForm" => AgentAction::SubmitForm {
                form_id: parsed.form_id?,
                action_label: parsed.action_label.unwrap_or_default(),
                identity: None,
            },
            "ClickAction" => AgentAction::ClickAction {
                label: parsed.label?,
                identity: None,
            },
            "Wait" => AgentAction::Wait {
                reason: parsed.reason.unwrap_or_else(|| "Waiting".to_string()),
            },
            _ => return None,
        };

        let decision_type = match &action {
            AgentAction::Wait { .. } => DecisionType::Wait,
            _ => DecisionType::Act,
        };

        Some(ModelDecision {
            decision: decision_type,
            next_action: Some(action),
            confidence,
        })
    }
}

impl ModelBackend for OllamaBackend {
    fn infer(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        memory: &AgentMemory,
    ) -> Option<ModelDecision> {
        let prompt = self.build_prompt(screen, diff, memory);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            format: "json",
        };

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .ok()?;

        let ollama_response: OllamaResponse = response.json().ok()?;
        self.parse_response(&ollama_response.response)
    }
}

// ============================================================================
// Mock Backend (for testing without Ollama)
// ============================================================================

pub struct MockBackend;

impl ModelBackend for MockBackend {
    fn infer(
        &self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        memory: &AgentMemory,
    ) -> Option<ModelDecision> {
        DeterministicPolicy.decide(screen, diff, memory)
    }
}
