use std::thread;
use std::time::Duration;

use crate::{
    agent::{
        agent_model::{
            AgentAction, AgentMemory, AgentState, DecisionType, MAX_LOOP_REPEATS, MAX_RETRIES,
            MIN_CONFIDENCE, ModelDecision,
        },
        budget::{BudgetDecision, check_budgets},
        ai_model::{ModelBackend, MockBackend, OllamaBackend},
    },
    browser::playwright::{BrowserCommand, SelectorHint, execute_browser_action},
    canonical::diff::{SemanticSignal, SemanticStateDiff},
    screen::screen_model::ElementKind,
    state::{identity::IdentifiedElement, state_model::ScreenState},
    trace::{logger::TraceLogger, trace::TraceEvent},
};

pub struct Agent {
    pub state: AgentState,
    pub memory: AgentMemory,
    pub step: u64,
    backend: Box<dyn ModelBackend>,
}

impl Agent {
    /// Create agent with default Ollama backend (requires Ollama running)
    pub fn new() -> Agent {
        Agent {
            state: AgentState::Observe,
            memory: AgentMemory::default(),
            step: 0,
            backend: Box::new(OllamaBackend::default()),
        }
    }

    /// Create agent with mock backend (for testing without Ollama)
    pub fn with_mock() -> Agent {
        Agent {
            state: AgentState::Observe,
            memory: AgentMemory::default(),
            step: 0,
            backend: Box::new(MockBackend),
        }
    }

    /// Create agent with custom Ollama config
    pub fn with_ollama(endpoint: &str, model: &str) -> Agent {
        Agent {
            state: AgentState::Observe,
            memory: AgentMemory::default(),
            step: 0,
            backend: Box::new(OllamaBackend::new(endpoint, model)),
        }
    }

    pub fn step(
        &mut self,
        screen: &ScreenState,
        diff: &SemanticStateDiff,
        tracer: &TraceLogger,
    ) -> Option<AgentAction> {
        let mut trace = TraceEvent::now(self.step as u64, &self.state).with_signals(&diff.signals);
        self.step += 1;

        match self.state {
            AgentState::Observe => {
                self.state = AgentState::Evaluate;
                tracer.log(&trace.with_decision("observe"));
                None
            }

            AgentState::Evaluate => {
                if diff.signals.is_empty() {
                    self.memory.loop_count += 1;
                    self.memory.loop_budget_remaining =
                        self.memory.loop_budget_remaining.saturating_sub(1);

                    tracer.log(
                        &trace
                            .with_decision("no_signals")
                            .with_suppression("no_progress"),
                    );

                    self.state = AgentState::Observe;
                    return None;
                }

                self.memory.loop_count = 0;
                self.memory.loop_budget_remaining = 5;

                self.memory.last_signal = diff.signals.last().cloned();
                self.state = AgentState::Think;
                tracer.log(&trace.with_decision("signals_detected"));
                None
            }

            AgentState::Think => {
                let decision = self.backend.infer(screen, diff, &self.memory)
                    .unwrap_or(ModelDecision {
                        decision: DecisionType::Wait,
                        next_action: None,
                        confidence: 0.0,
                    });

                trace = trace
                    .with_decision(format!("{:?}", decision.decision))
                    .with_confidence(decision.confidence);
                if let Some(action) = gate_decision(decision, &mut self.memory) {
                    self.state = AgentState::Act;
                    return Some(action);
                }

                tracer.log(&trace.with_suppression("gated"));
                self.state = AgentState::Observe;
                None
            }

            AgentState::Act => {
                // Action is executed outside
                tracer.log(&trace.with_decision("executed"));
                self.state = AgentState::Observe;
                None
            }

            AgentState::Stop => {
                tracer.log(&trace.with_decision("stop"));
                None
            }
        }
    }
}

pub fn emit_observed_actions(diff: &SemanticStateDiff, memory: &mut AgentMemory) {
    for signal in &diff.signals {
        if let SemanticSignal::FormSubmitted { form_id } = signal {
            let confirmed = AgentAction::FormSubmitted {
                form_id: form_id.clone(),
            };

            memory.last_confirmed_action = Some(confirmed);
            memory.attempt_count = 0; // reset retries on success
            memory.loop_count = 0;
        }
    }
}

fn _call_model(
    _screen: &ScreenState,
    _diff: &SemanticStateDiff,
    _memory: &AgentMemory,
) -> ModelDecision {
    // Placeholder: rule-based or mock
    ModelDecision {
        decision: DecisionType::Wait,
        next_action: None,
        confidence: 0.0,
    }
}

/// Build a SelectorHint from an IdentifiedElement's semantic info.
fn selector_from_target(target: &IdentifiedElement, form_id: Option<&str>) -> SelectorHint {
    SelectorHint {
        role: target.element.role.clone(),
        name: target.element.label.clone(),
        tag: target.element.tag.clone(),
        input_type: target.element.input_type.clone(),
        form_id: form_id.map(|s| s.to_string()),
    }
}

pub fn execute_action(action: &AgentAction, state: &ScreenState) -> Result<(), String> {
    let url = state
        .url
        .as_deref()
        .ok_or_else(|| "No URL in screen state".to_string())?;

    match action {
        AgentAction::FillInput {
            form_id,
            input_label,
            value,
            identity,
        } => {
            let target = resolve_target(identity, state)
                .or_else(|| find_input_by_label(form_id, input_label, state))
                .ok_or_else(|| {
                    format!("Input '{}' not found in form '{}'", input_label, form_id)
                })?;

            println!(
                "Filling input [{}] '{}' with '{}'",
                target.id, input_label, value
            );

            let command = BrowserCommand {
                action: "fill".into(),
                url: url.to_string(),
                value: Some(value.clone()),
                selector: Some(selector_from_target(target, Some(form_id))),
                duration_ms: None,
            };
            execute_browser_action(&command)
        }

        AgentAction::SubmitForm {
            form_id,
            action_label,
            identity,
        } => {
            let target = resolve_target(identity, state)
                .or_else(|| find_form_action_by_label(form_id, action_label, state))
                .ok_or_else(|| {
                    format!(
                        "Submit action '{}' not found in form '{}'",
                        action_label, form_id
                    )
                })?;

            println!(
                "Submitting form '{}' via action [{}] '{}'",
                form_id, target.id, action_label
            );

            let command = BrowserCommand {
                action: "click".into(),
                url: url.to_string(),
                value: None,
                selector: Some(selector_from_target(target, Some(form_id))),
                duration_ms: None,
            };
            execute_browser_action(&command)
        }

        AgentAction::ClickAction { label, identity } => {
            let target = resolve_target(identity, state)
                .or_else(|| find_standalone_action_by_label(label, state))
                .ok_or_else(|| format!("Standalone action '{}' not found", label))?;

            println!("Clicking standalone action [{}] '{}'", target.id, label);

            let command = BrowserCommand {
                action: "click".into(),
                url: url.to_string(),
                value: None,
                selector: Some(selector_from_target(target, None)),
                duration_ms: None,
            };
            execute_browser_action(&command)
        }

        AgentAction::Wait { reason } => {
            println!("Waiting: {}", reason);
            thread::sleep(Duration::from_secs(2));
            Ok(())
        }

        AgentAction::FormSubmitted { form_id } => {
            // IMPORTANT: This is an OBSERVED action, not executable
            println!(
                "Form '{}' submission confirmed (no execution required)",
                form_id
            );
            Ok(())
        }
    }
}

fn resolve_target<'a>(
    identity: &Option<String>,
    state: &'a ScreenState,
) -> Option<&'a IdentifiedElement> {
    identity.as_ref().and_then(|id| state.identities.get(id))
}

fn find_input_by_label<'a>(
    form_id: &str,
    label: &str,
    state: &'a ScreenState,
) -> Option<&'a IdentifiedElement> {
    state.identities.values().find(|el| {
        el.scope == format!("form:{}", form_id)
            && el.element.kind == ElementKind::Input
            && el.element.label.as_deref() == Some(label)
    })
}

fn find_form_action_by_label<'a>(
    form_id: &str,
    label: &str,
    state: &'a ScreenState,
) -> Option<&'a IdentifiedElement> {
    state.identities.values().find(|el| {
        el.scope == format!("form:{}", form_id)
            && el.element.kind == ElementKind::Action
            && el.element.label.as_deref() == Some(label)
    })
}

fn find_standalone_action_by_label<'a>(
    label: &str,
    state: &'a ScreenState,
) -> Option<&'a IdentifiedElement> {
    state.identities.values().find(|el| {
        el.scope == "screen"
            && el.element.kind == ElementKind::Action
            && el.element.label.as_deref() == Some(label)
    })
}

fn same_action(a: &AgentAction, b: &AgentAction) -> bool {
    a == b
}

pub fn gate_decision(decision: ModelDecision, memory: &mut AgentMemory) -> Option<AgentAction> {
    // --- Terminal success gate ---
    if let Some(AgentAction::FormSubmitted {
        form_id: submitted_form,
    }) = &memory.last_confirmed_action
    {
        if let Some(next_action) = decision.next_action.as_ref() {
            if let AgentAction::SubmitForm {
                form_id: next_form, ..
            } = next_action
            {
                if submitted_form == next_form {
                    println!(
                        "Form '{}' already submitted successfully — blocking retry",
                        submitted_form
                    );
                    return None;
                }
            }
        }
    }

    let action = decision.next_action?.clone();
    // --- Confidence gate ---
    if decision.confidence < MIN_CONFIDENCE {
        println!("Low confidence: {}", decision.confidence);
        return None;
    }

    // --- Loop detection (authoritative budget) ---
    if let Some(prev) = &memory.last_action {
        if same_action(prev, &action) {
            memory.loop_budget_remaining = memory.loop_budget_remaining.saturating_sub(1);

            if memory.loop_budget_remaining == 0 {
                println!("Loop budget exhausted");
                return None;
            }
        } else {
            memory.loop_budget_remaining = MAX_LOOP_REPEATS;
        }
    }

    // --- Budget gate ---
    match check_budgets(memory, &action) {
        BudgetDecision::Allow => {
            memory.think_budget_remaining = memory.think_budget_remaining.saturating_sub(1);

            if let Some(last) = &memory.last_action {
                if same_action(last, &action) {
                    memory.retry_budget_remaining = memory.retry_budget_remaining.saturating_sub(1);
                } else {
                    memory.retry_budget_remaining = MAX_RETRIES;
                }
            } else {
                memory.retry_budget_remaining = MAX_RETRIES;
            }

            memory.last_action = Some(action.clone());
            Some(action)
        }

        BudgetDecision::Block(reason) => {
            println!("Action gated: {}", reason);
            None
        }
    }
}
