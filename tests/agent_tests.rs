use std::collections::HashMap;
use std::error::Error as StdError;

use screen_detection::{
    agent::{
        agent::{Agent, execute_action, gate_decision},
        agent_model::{AgentAction, AgentMemory, DecisionType, MAX_LOOP_REPEATS, ModelDecision},
        ai_model::{DeterministicPolicy, guess_value},
        budget::{BudgetDecision, check_budgets},
        error::AgentError,
    },
    browser::playwright::extract_screen,
    canonical::{
        canonical_model::{CanonicalScreenState, canonicalize},
        diff::{ActionDiff, FormDiff, OutputDiff, SemanticSignal, SemanticStateDiff, semantic_diff},
    },
    screen::{
        classifier::classify,
        screen_model::{DomElement, ElementKind, Form, ScreenElement},
    },
    state::{
        state_builder::build_state,
        state_model::ScreenState,
    },
    trace::logger::TraceLogger,
};

use screen_detection::agent::agent_model::Policy;

use crate::common::{
    semantic_diff::diff_static,
    utils::page,
};

mod common;

// =========================================================================
// Helpers
// =========================================================================

fn run_agent_until_action(
    agent: &mut Agent,
    state: &ScreenState,
    diff: &SemanticStateDiff,
    tracer: &TraceLogger,
    max_steps: usize,
) -> Option<AgentAction> {
    for _ in 0..max_steps {
        if let Some(action) = agent.step(state, diff, tracer) {
            return Some(action);
        }
    }
    None
}

fn mock_screen_with_form() -> ScreenState {
    ScreenState {
        url: Some("https://example.com".into()),
        title: "Test Page".into(),
        forms: vec![Form {
            id: "login".into(),
            inputs: vec![ScreenElement {
                label: Some("Username".into()),
                kind: ElementKind::Input,
                tag: Some("input".into()),
                role: Some("textbox".into()),
                input_type: Some("text".into()),
            }],
            actions: vec![ScreenElement {
                label: Some("Sign In".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: Some("submit".into()),
            }],
            primary_action: None,
            intent: None,
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    }
}

fn diff_with_signal(signal: SemanticSignal) -> SemanticStateDiff {
    SemanticStateDiff {
        forms: FormDiff { added: vec![], removed: vec![], changed: vec![] },
        standalone_actions: ActionDiff { added: vec![], removed: vec![] },
        outputs: OutputDiff { added: vec![], removed: vec![] },
        signals: vec![signal],
    }
}

// =========================================================================
// Agent flow tests
// =========================================================================

#[test]
fn agent_completes_search_flow() {
    let tracer = TraceLogger::new("agent_trace.jsonl");
    let mut agent = Agent::new();

    let pages = vec![
        "01_search_form.html",
        "02_search_submitted.html",
        "03_search_results.html",
    ];

    let mut prev_state: Option<CanonicalScreenState> = None;

    for p in pages {
        let raw = extract_screen(&page(p)).unwrap();
        let dom = raw["dom"].as_array().unwrap();
        let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

        let semantics = classify(&elements);
        let state = build_state(
            Some(raw["url"].as_str().unwrap_or("test://fixture")),
            raw["title"].as_str().unwrap_or(""),
            semantics,
        );

        let canonical = canonicalize(&state, None);

        let diff = match &prev_state {
            Some(prev) => semantic_diff(prev, &canonical, false),
            None => semantic_diff(&CanonicalScreenState::empty(), &canonical, true),
        };

        if let Some(action) = agent.step(&state, &diff, &tracer) {
            let _ = execute_action(&action, &state);
        }

        prev_state = Some(canonical);
    }

    assert_eq!(
        agent.memory.last_signal,
        Some(SemanticSignal::ResultsAppeared)
    );
}

#[test]
fn agent_does_not_act_on_noop() {
    let tracer = TraceLogger::new("noop_trace.jsonl");
    let mut agent = Agent::new();

    let diff = diff_static(&page("01_search_form.html"));

    let raw = extract_screen(&page("01_search_form.html")).unwrap();
    let dom = raw["dom"].as_array().unwrap();
    let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

    let semantics = classify(&elements);
    let state = build_state(
        Some(raw["url"].as_str().unwrap_or("test://fixture")),
        raw["title"].as_str().unwrap_or(""),
        semantics,
    );

    let action = run_agent_until_action(&mut agent, &state, &diff, &tracer, 5);

    assert!(action.is_none(), "Agent must not act on NoOp");
}

#[test]
fn agent_stops_after_repeated_error() {
    let tracer = TraceLogger::new("error_loop.jsonl");
    let mut agent = Agent::new();

    let pages = vec!["05_error.html", "05_error.html", "05_error.html"];

    let mut prev: Option<CanonicalScreenState> = None;

    let mut last_action = None;

    for p in pages {
        let raw = extract_screen(&page(p)).unwrap();
        let dom = raw["dom"].as_array().unwrap();
        let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

        let semantics = classify(&elements);
        let state = build_state(
            Some(raw["url"].as_str().unwrap_or("test://fixture")),
            raw["title"].as_str().unwrap_or(""),
            semantics,
        );

        let canonical = canonicalize(&state, None);

        let diff = match &prev {
            Some(prev) => semantic_diff(prev, &canonical, false),
            None => semantic_diff(&CanonicalScreenState::empty(), &canonical, true),
        };

        last_action = agent.step(&state, &diff, &tracer);
        prev = Some(canonical);
    }

    assert!(
        last_action.is_none(),
        "Agent should stop proposing actions after repeated errors"
    );

    assert!(
        matches!(agent.memory.last_signal, Some(SemanticSignal::NoOp)),
        "Agent should settle into NoOp after error loop"
    );
}

#[test]
fn suppresses_looping_action() {
    let mut memory = AgentMemory::default();

    let action = AgentAction::ClickAction {
        label: "Search".into(),
        identity: Some("btn1".into()),
    };

    let decision = ModelDecision {
        decision: DecisionType::Act,
        next_action: Some(action.clone()),
        confidence: 0.9,
    };

    // Allow up to MAX_LOOP_REPEATS
    for _ in 0..MAX_LOOP_REPEATS {
        let allowed = gate_decision(decision.clone(), &mut memory);
        assert!(
            allowed.is_some(),
            "Action should be allowed before loop exhaustion"
        );
    }

    // Now it must be blocked
    let blocked = gate_decision(decision, &mut memory);
    assert!(blocked.is_none(), "Looping action should be suppressed");
}

/// Full agent loop test using MockBackend (no Ollama required)
#[test]
fn agent_loop_with_mock_backend() {
    let tracer = TraceLogger::new("mock_agent_trace.jsonl");
    let mut agent = Agent::with_mock(); // Uses MockBackend

    let pages = vec![
        ("01_search_form.html", "Initial form load"),
        ("02_search_submitted.html", "After form submission"),
        ("03_search_results.html", "Results page"),
    ];

    let mut prev_state: Option<CanonicalScreenState> = None;
    let mut actions_taken: Vec<String> = vec![];

    println!("\n=== AGENT LOOP TEST ===\n");

    for (page_file, description) in pages {
        println!("--- {} ({}) ---", description, page_file);

        // Load page
        let raw = extract_screen(&page(page_file)).unwrap();
        let dom = raw["dom"].as_array().unwrap();
        let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

        let semantics = classify(&elements);
        let state = build_state(
            Some(raw["url"].as_str().unwrap_or("test://fixture")),
            raw["title"].as_str().unwrap_or(""),
            semantics,
        );

        let canonical = canonicalize(&state, None);

        // Compute diff
        let diff = match &prev_state {
            Some(prev) => semantic_diff(prev, &canonical, false),
            None => semantic_diff(&CanonicalScreenState::empty(), &canonical, true),
        };

        println!("  Signals: {:?}", diff.signals);

        // Run agent step (may need multiple steps to reach Act state)
        for step in 0..5 {
            if let Some(action) = agent.step(&state, &diff, &tracer) {
                let action_desc = format!("{:?}", action);
                println!("  Step {}: Action = {}", step, action_desc);
                actions_taken.push(action_desc);

                // Execute the action (logs only, no real browser)
                let _ = execute_action(&action, &state);
                break;
            }
        }

        println!("  Agent state: {:?}", agent.state);
        println!("  Last signal: {:?}", agent.memory.last_signal);
        println!();

        prev_state = Some(canonical);
    }

    println!("=== SUMMARY ===");
    println!("Actions taken: {}", actions_taken.len());
    for (i, action) in actions_taken.iter().enumerate() {
        println!("  {}: {}", i + 1, action);
    }

    // Verify agent processed the flow
    assert!(
        !actions_taken.is_empty(),
        "Agent should have taken at least one action"
    );
    assert_eq!(
        agent.memory.last_signal,
        Some(SemanticSignal::ResultsAppeared),
        "Agent should end with ResultsAppeared signal"
    );
}

// =========================================================================
// DeterministicPolicy tests
// =========================================================================

#[test]
fn deterministic_policy_fills_input_on_screen_loaded() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form();
    let diff = diff_with_signal(SemanticSignal::ScreenLoaded);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory);
    assert!(decision.is_some(), "Should return a decision for ScreenLoaded");

    let decision = decision.unwrap();
    assert!(matches!(decision.decision, DecisionType::Act));
    assert!(matches!(
        decision.next_action,
        Some(AgentAction::FillAndSubmitForm { ref form_id, .. }) if form_id == "login"
    ));
    assert!(decision.confidence >= 0.65, "Confidence must pass gating threshold");
}

#[test]
fn deterministic_policy_waits_on_form_submitted() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form();
    let diff = diff_with_signal(SemanticSignal::FormSubmitted {
        form_id: "login".into(),
    });
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();
    assert!(matches!(decision.decision, DecisionType::Wait));
    assert!(matches!(decision.next_action, Some(AgentAction::Wait { .. })));
}

#[test]
fn deterministic_policy_returns_none_on_noop() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form();
    let diff = diff_with_signal(SemanticSignal::NoOp);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory);
    assert!(decision.is_none(), "DeterministicPolicy must return None on NoOp");
}

#[test]
fn agent_with_deterministic_policy() {
    let tracer = TraceLogger::new("deterministic_trace.jsonl");
    let mut agent = Agent::with_deterministic();

    let pages = vec![
        "01_search_form.html",
        "02_search_submitted.html",
        "03_search_results.html",
    ];

    let mut prev_state: Option<CanonicalScreenState> = None;
    let mut actions_taken = 0;

    for p in pages {
        let raw = extract_screen(&page(p)).unwrap();
        let dom = raw["dom"].as_array().unwrap();
        let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

        let semantics = classify(&elements);
        let state = build_state(
            Some(raw["url"].as_str().unwrap_or("test://fixture")),
            raw["title"].as_str().unwrap_or(""),
            semantics,
        );

        let canonical = canonicalize(&state, None);

        let diff = match &prev_state {
            Some(prev) => semantic_diff(prev, &canonical, false),
            None => semantic_diff(&CanonicalScreenState::empty(), &canonical, true),
        };

        for _ in 0..5 {
            if let Some(_action) = agent.step(&state, &diff, &tracer) {
                actions_taken += 1;
                break;
            }
        }

        prev_state = Some(canonical);
    }

    assert!(actions_taken > 0, "Deterministic agent should take at least one action");
    assert_eq!(
        agent.memory.last_signal,
        Some(SemanticSignal::ResultsAppeared),
        "Agent should end with ResultsAppeared"
    );
}

// =========================================================================
// guess_value and FillAndSubmitForm tests
// =========================================================================

#[test]
fn guess_value_uses_label_heuristics() {
    assert_eq!(guess_value("email", None), "user@example.com");
    assert_eq!(guess_value("Email Address", None), "user@example.com");
    assert_eq!(guess_value("password", None), "TestPass123!");
    assert_eq!(guess_value("Phone Number", None), "555-0100");
    assert_eq!(guess_value("search", None), "test query");
    assert_eq!(guess_value("query", None), "test query");
    assert_eq!(guess_value("username", None), "testuser");
    assert_eq!(guess_value("Full Name", None), "Jane Doe");
    assert_eq!(guess_value("zip code", None), "90210");
    assert_eq!(guess_value("some random field", None), "test");
}

#[test]
fn guess_value_falls_back_to_input_type() {
    // Generic label, but specific input type
    assert_eq!(guess_value("enter value", Some("email")), "user@example.com");
    assert_eq!(guess_value("enter value", Some("password")), "TestPass123!");
    assert_eq!(guess_value("enter value", Some("tel")), "555-0100");
    assert_eq!(guess_value("enter value", Some("number")), "42");
    assert_eq!(guess_value("enter value", Some("date")), "2025-01-15");
    // Unknown type falls to final fallback
    assert_eq!(guess_value("enter value", Some("text")), "test");
}

#[test]
fn deterministic_policy_returns_fill_and_submit() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form(); // has "Username" input + "Sign In" action
    let diff = diff_with_signal(SemanticSignal::ScreenLoaded);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();
    assert!(matches!(decision.decision, DecisionType::Act));

    match &decision.next_action {
        Some(AgentAction::FillAndSubmitForm { form_id, values, submit_label }) => {
            assert_eq!(form_id, "login");
            assert_eq!(values.len(), 1);
            assert_eq!(values[0].0, "Username");
            assert_eq!(values[0].1, "testuser"); // "Username" label → "testuser"
            assert_eq!(submit_label.as_deref(), Some("Sign In"));
        }
        other => panic!("Expected FillAndSubmitForm, got {:?}", other),
    }
}

#[test]
fn deterministic_policy_multi_input_form() {
    let policy = DeterministicPolicy;
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Signup".into(),
        forms: vec![Form {
            id: "signup".into(),
            inputs: vec![
                ScreenElement {
                    label: Some("Email".into()),
                    kind: ElementKind::Input,
                    tag: Some("input".into()),
                    role: Some("textbox".into()),
                    input_type: Some("email".into()),
                },
                ScreenElement {
                    label: Some("Password".into()),
                    kind: ElementKind::Input,
                    tag: Some("input".into()),
                    role: None,
                    input_type: Some("password".into()),
                },
                ScreenElement {
                    label: Some("Phone".into()),
                    kind: ElementKind::Input,
                    tag: Some("input".into()),
                    role: None,
                    input_type: Some("tel".into()),
                },
            ],
            actions: vec![ScreenElement {
                label: Some("Register".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: Some("submit".into()),
            }],
            primary_action: None,
            intent: None,
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };

    let diff = diff_with_signal(SemanticSignal::ScreenLoaded);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();

    match &decision.next_action {
        Some(AgentAction::FillAndSubmitForm { form_id, values, submit_label }) => {
            assert_eq!(form_id, "signup");
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], ("Email".to_string(), "user@example.com".to_string()));
            assert_eq!(values[1], ("Password".to_string(), "TestPass123!".to_string()));
            assert_eq!(values[2], ("Phone".to_string(), "555-0100".to_string()));
            assert_eq!(submit_label.as_deref(), Some("Register"));
        }
        other => panic!("Expected FillAndSubmitForm, got {:?}", other),
    }
}

// =========================================================================
// AgentError tests
// =========================================================================

#[test]
fn agent_error_display_messages() {
    let missing = AgentError::MissingState("no url".into());
    assert!(missing.to_string().contains("Missing state"), "MissingState display");
    assert!(missing.to_string().contains("no url"), "MissingState contains detail");

    let browser = AgentError::BrowserAction("timeout".into());
    assert!(browser.to_string().contains("Browser action failed"), "BrowserAction display");
    assert!(browser.to_string().contains("timeout"), "BrowserAction contains detail");

    let dom = AgentError::DomStructure("no dom array".into());
    assert!(dom.to_string().contains("Unexpected DOM"), "DomStructure display");
    assert!(dom.to_string().contains("no dom array"), "DomStructure contains detail");

    let not_found = AgentError::ElementNotFound {
        element: "Submit".into(),
        context: "form 'login'".into(),
    };
    assert!(not_found.to_string().contains("Submit"), "ElementNotFound contains element");
    assert!(not_found.to_string().contains("form 'login'"), "ElementNotFound contains context");
}

#[test]
fn agent_error_source_chain() {
    // JsonParse wraps serde_json::Error → source() is Some
    let json_err: serde_json::Error = serde_json::from_str::<String>("not json").unwrap_err();
    let parse = AgentError::JsonParse {
        context: "test".into(),
        source: json_err,
    };
    assert!(parse.source().is_some(), "JsonParse should have a source");

    // BrowserAction → no source
    let browser = AgentError::BrowserAction("fail".into());
    assert!(browser.source().is_none(), "BrowserAction should have no source");

    // MissingState → no source
    let missing = AgentError::MissingState("x".into());
    assert!(missing.source().is_none(), "MissingState should have no source");

    // ElementNotFound → no source
    let not_found = AgentError::ElementNotFound {
        element: "x".into(),
        context: "y".into(),
    };
    assert!(not_found.source().is_none(), "ElementNotFound should have no source");
}

#[test]
fn agent_error_session_io_display() {
    let err = AgentError::SessionIO("stdin broken".into());
    assert!(err.to_string().contains("Session I/O error"), "SessionIO display");
    assert!(err.to_string().contains("stdin broken"), "SessionIO contains detail");
    assert!(err.source().is_none(), "SessionIO has no source");
}

#[test]
fn agent_error_session_protocol_display() {
    let err = AgentError::SessionProtocol {
        command: "navigate".into(),
        error: "timeout".into(),
    };
    assert!(err.to_string().contains("Session protocol error"), "SessionProtocol display");
    assert!(err.to_string().contains("navigate"), "SessionProtocol contains command");
    assert!(err.to_string().contains("timeout"), "SessionProtocol contains error");
    assert!(err.source().is_none(), "SessionProtocol has no source");
}

// =========================================================================
// DeterministicPolicy — signal branches
// =========================================================================

#[test]
fn deterministic_policy_waits_on_results_appeared() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form();
    let diff = diff_with_signal(SemanticSignal::ResultsAppeared);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();
    assert!(matches!(decision.decision, DecisionType::Wait));
    match &decision.next_action {
        Some(AgentAction::Wait { reason }) => {
            assert!(reason.contains("Results appeared"), "Reason: {}", reason);
        }
        other => panic!("Expected Wait action, got {:?}", other),
    }
}

#[test]
fn deterministic_policy_waits_on_navigation_occurred() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form();
    let diff = diff_with_signal(SemanticSignal::NavigationOccurred);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();
    assert!(matches!(decision.decision, DecisionType::Wait));
    match &decision.next_action {
        Some(AgentAction::Wait { reason }) => {
            assert!(reason.contains("Navigation occurred"), "Reason: {}", reason);
        }
        other => panic!("Expected Wait action, got {:?}", other),
    }
}

#[test]
fn deterministic_policy_waits_on_error_appeared() {
    let policy = DeterministicPolicy;
    let screen = mock_screen_with_form();
    let diff = diff_with_signal(SemanticSignal::ErrorAppeared);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();
    assert!(matches!(decision.decision, DecisionType::Wait));
    match &decision.next_action {
        Some(AgentAction::Wait { reason }) => {
            assert!(reason.contains("Error appeared"), "Reason: {}", reason);
        }
        other => panic!("Expected Wait action, got {:?}", other),
    }
}

// =========================================================================
// DeterministicPolicy — edge cases
// =========================================================================

#[test]
fn deterministic_policy_no_forms_returns_none() {
    let policy = DeterministicPolicy;
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Empty Page".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };
    let diff = diff_with_signal(SemanticSignal::ScreenLoaded);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory);
    assert!(decision.is_none(), "ScreenLoaded with no forms should return None");
}

#[test]
fn deterministic_policy_form_no_inputs_still_submits() {
    let policy = DeterministicPolicy;
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Action Only".into(),
        forms: vec![Form {
            id: "confirm".into(),
            inputs: vec![],
            actions: vec![ScreenElement {
                label: Some("Confirm".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: Some("submit".into()),
            }],
            primary_action: None,
            intent: None,
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };
    let diff = diff_with_signal(SemanticSignal::ScreenLoaded);
    let memory = AgentMemory::default();

    let decision = policy.decide(&screen, &diff, &memory).unwrap();
    assert!(matches!(decision.decision, DecisionType::Act));

    match &decision.next_action {
        Some(AgentAction::FillAndSubmitForm { form_id, values, submit_label }) => {
            assert_eq!(form_id, "confirm");
            assert!(values.is_empty(), "No inputs means empty values vec");
            assert_eq!(submit_label.as_deref(), Some("Confirm"));
        }
        other => panic!("Expected FillAndSubmitForm, got {:?}", other),
    }
}

// =========================================================================
// Budget gating tests
// =========================================================================

#[test]
fn budget_blocks_when_think_exhausted() {
    let mut memory = AgentMemory::default();
    memory.think_budget_remaining = 0;

    let action = AgentAction::Wait { reason: "test".into() };
    let result = check_budgets(&memory, &action);
    assert!(
        matches!(result, BudgetDecision::Block("think_budget_exhausted")),
        "Expected think_budget_exhausted, got {:?}", result
    );
}

#[test]
fn budget_blocks_when_retry_exhausted() {
    let mut memory = AgentMemory::default();
    let action = AgentAction::Wait { reason: "same".into() };
    memory.last_action = Some(action.clone());
    memory.retry_budget_remaining = 0;

    let result = check_budgets(&memory, &action);
    assert!(
        matches!(result, BudgetDecision::Block("retry_budget_exhausted")),
        "Expected retry_budget_exhausted, got {:?}", result
    );
}

#[test]
fn budget_blocks_when_loop_exhausted() {
    let mut memory = AgentMemory::default();
    memory.loop_budget_remaining = 0;

    let action = AgentAction::Wait { reason: "test".into() };
    let result = check_budgets(&memory, &action);
    assert!(
        matches!(result, BudgetDecision::Block("loop_budget_exhausted")),
        "Expected loop_budget_exhausted, got {:?}", result
    );
}

#[test]
fn gate_decision_rejects_low_confidence() {
    let mut memory = AgentMemory::default();
    let decision = ModelDecision {
        decision: DecisionType::Act,
        next_action: Some(AgentAction::Wait { reason: "test".into() }),
        confidence: 0.3,
    };

    let result = gate_decision(decision, &mut memory);
    assert!(result.is_none(), "Low confidence should be gated");
}

// =========================================================================
// execute_action error paths
// =========================================================================

#[test]
fn execute_action_errors_on_missing_url() {
    let state = ScreenState {
        url: None,
        title: "Test".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };

    let action = AgentAction::FillInput {
        form_id: "search".into(),
        input_label: "Query".into(),
        value: "test".into(),
        identity: None,
    };

    let result = execute_action(&action, &state);
    assert!(result.is_err(), "Must fail when URL is missing");
    assert!(
        matches!(result.unwrap_err(), AgentError::MissingState(_)),
        "Error should be MissingState variant"
    );
}

#[test]
fn execute_action_errors_on_missing_element() {
    let state = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };

    // FillInput with unknown identity and no matching label
    let fill = execute_action(
        &AgentAction::FillInput {
            form_id: "search".into(),
            input_label: "nonexistent".into(),
            value: "test".into(),
            identity: Some("bogus_id".into()),
        },
        &state,
    );
    assert!(fill.is_err(), "FillInput must fail for missing element");
    assert!(
        matches!(fill.unwrap_err(), AgentError::ElementNotFound { .. }),
        "FillInput error should be ElementNotFound"
    );

    // SubmitForm with unknown identity
    let submit = execute_action(
        &AgentAction::SubmitForm {
            form_id: "search".into(),
            action_label: "nonexistent".into(),
            identity: Some("bogus_id".into()),
        },
        &state,
    );
    assert!(submit.is_err(), "SubmitForm must fail for missing element");
    assert!(
        matches!(submit.unwrap_err(), AgentError::ElementNotFound { .. }),
        "SubmitForm error should be ElementNotFound"
    );

    // ClickAction with unknown identity
    let click = execute_action(
        &AgentAction::ClickAction {
            label: "nonexistent".into(),
            identity: Some("bogus_id".into()),
        },
        &state,
    );
    assert!(click.is_err(), "ClickAction must fail for missing element");
    assert!(
        matches!(click.unwrap_err(), AgentError::ElementNotFound { .. }),
        "ClickAction error should be ElementNotFound"
    );
}

#[test]
fn execute_action_navigate_to_is_noop_in_subprocess_mode() {
    let state = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };

    let action = AgentAction::NavigateTo {
        url: "https://other.com".into(),
        reason: "testing".into(),
    };

    // Should succeed (no-op in subprocess mode)
    let result = execute_action(&action, &state);
    assert!(result.is_ok(), "NavigateTo should be ok in subprocess mode");
}

#[test]
fn execute_action_session_errors_on_missing_element() {
    let state = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    };

    let action = AgentAction::FillInput {
        form_id: "nonexistent".into(),
        input_label: "missing".into(),
        value: "test".into(),
        identity: None,
    };

    // Test via the original execute_action (same error behavior)
    let result = execute_action(&action, &state);
    assert!(matches!(result.unwrap_err(), AgentError::ElementNotFound { .. }));
}
