use std::collections::HashMap;

use screen_detection::{
    agent::{
        agent::{Agent, execute_action, gate_decision},
        agent_model::{AgentAction, AgentMemory, DecisionType, MAX_LOOP_REPEATS, ModelDecision, Policy},
        ai_model::DeterministicPolicy,
    },
    browser::playwright::{SelectorHint, extract_screen},
    canonical::{
        canonical_model::{CanonicalScreenState, canonicalize},
        diff::{ActionDiff, FormDiff, OutputDiff, SemanticSignal, SemanticStateDiff, semantic_diff},
    },
    screen::{classifier::classify, screen_model::{DomElement, ElementKind, Form, ScreenElement}},
    state::{state_builder::build_state, state_model::ScreenState},
    trace::logger::TraceLogger,
};

use crate::common::{
    semantic_diff::{
        diff_between_pages, diff_mutating, diff_static, diff_static_with_initial_flag,
    },
    utils::{is_terminal_success, page},
};

mod common;

#[test]
fn initial_page_produces_screen_loaded() {
    let diff = diff_static(&page("01_search_form.html"));

    assert!(
        diff.signals.contains(&SemanticSignal::ScreenLoaded),
        "Expected ScreenLoaded for initial page"
    );

    assert!(
        !diff.signals.contains(&SemanticSignal::NoOp),
        "Initial page must not be NoOp"
    );
}

#[test]
fn form_submission_emits_signal() {
    let diff = diff_between_pages(
        &&page("01_search_form.html"),      // BEFORE
        &&page("02_search_submitted.html"), // AFTER
    );
    println!("diff={:?}", diff);
    assert!(diff.signals.contains(&SemanticSignal::FormSubmitted {
        form_id: "search".into()
    }));
}

#[test]
fn results_appearing_is_detected() {
    let diff = diff_between_pages(
        &&page("02_search_submitted.html"), // BEFORE
        &&page("03_search_results.html"),   // AFTER
    );

    println!("Diff = {:?}", diff);
    assert!(
        diff.signals.contains(&SemanticSignal::ResultsAppeared),
        "Expected ResultsAppeared signal"
    );
}

#[test]
fn direct_form_to_results_is_submission_not_navigation() {
    let diff = diff_between_pages(
        &page("01_search_form.html"),
        &page("03_search_results.html"),
    );

    assert!(
        diff.signals.iter().any(|s| matches!(
            s,
            SemanticSignal::FormSubmitted { form_id } if form_id == "search"
        )),
        "Expected form submission"
    );

    assert!(
        diff.signals.contains(&SemanticSignal::ResultsAppeared),
        "Expected results to appear"
    );

    assert!(
        !diff.signals.contains(&SemanticSignal::NavigationOccurred),
        "Submission with results must not be classified as navigation"
    );
}

#[test]
fn error_appearing_is_detected() {
    let diff = diff_between_pages(&page("02_search_submitted.html"), &page("05_error.html"));

    assert!(
        diff.signals.contains(&SemanticSignal::ErrorAppeared),
        "Expected ErrorAppeared when error first appears"
    );
}

#[test]
fn form_submission_is_detected() {
    let diff = diff_between_pages(
        &page("01_search_form.html"),
        &page("02_search_submitted.html"),
    );

    assert!(
        diff.signals
            .iter()
            .any(|s| matches!(s, SemanticSignal::FormSubmitted { .. })),
        "Expected FormSubmitted signal"
    );
}

#[test]
fn no_false_positive_signals() {
    let diff = diff_mutating(&page("01_search_form.html"), 100);

    assert_eq!(
        diff.signals,
        vec![SemanticSignal::NoOp],
        "Static diff should ONLY produce NoOp"
    );
}

#[test]
fn error_then_results_is_success() {
    let diff = diff_between_pages(&page("05_error.html"), &page("03_search_results.html"));
    println!("Diff = {:?}", diff);

    assert!(
        is_terminal_success(&diff),
        "Transition must be considered success"
    );

    assert!(
        !diff.signals.contains(&SemanticSignal::ErrorAppeared),
        "Old error must not persist"
    );
}

#[test]
fn form_disappears_with_content_is_submission() {
    let diff = diff_between_pages(&page("01_search_form.html"), &page("08_navigation.html"));

    assert!(
        diff.signals
            .iter()
            .any(|s| matches!(s, SemanticSignal::FormSubmitted { .. })),
        "Form disappearance with content is a submission"
    );
}

#[test]
fn repeated_results_page_is_noop() {
    let diff = diff_static_with_initial_flag(&page("03_search_results.html"), false);

    assert_eq!(
        diff.signals,
        vec![SemanticSignal::NoOp],
        "Repeated results page should be NoOp"
    );
}

#[test]
fn repeated_error_page_is_noop() {
    let diff = diff_static_with_initial_flag(&page("05_error.html"), false);

    assert_eq!(
        diff.signals,
        vec![SemanticSignal::NoOp],
        "Repeated error page should be NoOp"
    );
}

#[test]
fn correct_form_is_marked_submitted() {
    let diff = diff_between_pages(
        &page("06_multiple_forms.html"),
        &page("02_search_submitted.html"),
    );

    println!("Diff = {:?}", diff);

    assert!(
        diff.signals.iter().any(|s| matches!(
            s,
            SemanticSignal::FormSubmitted { form_id } if form_id == "search"
        )),
        "Correct form must be marked submitted"
    );
}

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
        let raw = extract_screen(&page(p));
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

#[test]
fn agent_does_not_act_on_noop() {
    let tracer = TraceLogger::new("noop_trace.jsonl");
    let mut agent = Agent::new();

    let diff = diff_static(&page("01_search_form.html"));

    let raw = extract_screen(&page("01_search_form.html"));
    let dom = raw["dom"].as_array().unwrap();
    let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

    let semantics = classify(&elements);
    let state = build_state(
        Some(raw["url"].as_str().unwrap_or("test://fixture")),
        raw["title"].as_str().unwrap_or(""),
        semantics,
    );

    // let action = agent.step(&state, &diff, &tracer);
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
        let raw = extract_screen(&page(p));
        let dom = raw["dom"].as_array().unwrap();
        let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into()).unwrap();

        let semantics = classify(&elements);
        let state = build_state(
            Some(raw["url"].as_str().unwrap_or("test://fixture")),
            raw["title"].as_str().unwrap_or(""),
            semantics,
        );

        // let state = load_state(p);
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
        let raw = extract_screen(&page(page_file));
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
// New tests: classifier field population, selector hints, serialization,
// and error paths for execute_action
// =========================================================================

#[test]
fn classifier_populates_screen_element_fields() {
    let elements = vec![
        DomElement {
            tag: "input".into(),
            text: None,
            role: Some("textbox".into()),
            r#type: Some("text".into()),
            aria_label: Some("Search query".into()),
            disabled: false,
            required: false,
            form_id: Some("search".into()),
        },
        DomElement {
            tag: "button".into(),
            text: Some("Submit".into()),
            role: Some("button".into()),
            r#type: Some("submit".into()),
            aria_label: None,
            disabled: false,
            required: false,
            form_id: Some("search".into()),
        },
        DomElement {
            tag: "div".into(),
            text: Some("Results will appear here".into()),
            role: None,
            r#type: None,
            aria_label: None,
            disabled: false,
            required: false,
            form_id: None,
        },
    ];

    let semantics = classify(&elements);

    // Check the input in the form
    let form = semantics.forms.iter().find(|f| f.id == "search").unwrap();
    let input = &form.inputs[0];
    assert_eq!(input.tag, Some("input".into()));
    assert_eq!(input.role, Some("textbox".into()));
    assert_eq!(input.input_type, Some("text".into()));
    assert_eq!(input.label, Some("Search query".into()));

    // Check the action in the form
    let action = &form.actions[0];
    assert_eq!(action.tag, Some("button".into()));
    assert_eq!(action.role, Some("button".into()));
    assert_eq!(action.input_type, Some("submit".into()));

    // Check the output
    let output = &semantics.outputs[0];
    assert_eq!(output.tag, Some("div".into()));
    assert_eq!(output.role, None);
    assert_eq!(output.input_type, None);
}

#[test]
fn selector_hint_serializes_with_correct_field_names() {
    let hint = SelectorHint {
        role: Some("textbox".into()),
        name: Some("Email".into()),
        tag: Some("input".into()),
        input_type: Some("email".into()),
        form_id: Some("login".into()),
    };

    let json: serde_json::Value = serde_json::to_value(&hint).unwrap();

    // Verify serde renames: input_type -> "type", form_id -> "formId"
    assert_eq!(json["role"], "textbox");
    assert_eq!(json["name"], "Email");
    assert_eq!(json["tag"], "input");
    assert_eq!(json["type"], "email", "input_type must serialize as 'type'");
    assert_eq!(json["formId"], "login", "form_id must serialize as 'formId'");

    // Verify the Rust field names are NOT in the JSON
    assert!(json.get("input_type").is_none(), "Must not contain 'input_type' key");
    assert!(json.get("form_id").is_none(), "Must not contain 'form_id' key");
}

#[test]
fn selector_hint_skips_none_fields() {
    let hint = SelectorHint {
        role: None,
        name: Some("Click me".into()),
        tag: Some("a".into()),
        input_type: None,
        form_id: None,
    };

    let json_str = serde_json::to_string(&hint).unwrap();

    assert!(!json_str.contains("role"), "None fields must be skipped");
    assert!(!json_str.contains("type"), "None fields must be skipped");
    assert!(!json_str.contains("formId"), "None fields must be skipped");
    assert!(json_str.contains("name"), "Present fields must appear");
    assert!(json_str.contains("tag"), "Present fields must appear");
}

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
        result.unwrap_err().contains("No URL"),
        "Error message should mention missing URL"
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

    // ClickAction with unknown identity
    let click = execute_action(
        &AgentAction::ClickAction {
            label: "nonexistent".into(),
            identity: Some("bogus_id".into()),
        },
        &state,
    );
    assert!(click.is_err(), "ClickAction must fail for missing element");
}

// =========================================================================
// Policy tests: DeterministicPolicy behavior and Agent integration
// =========================================================================

/// Helper: build a minimal ScreenState with one form containing one input and one action.
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

/// Helper: build a SemanticStateDiff with a single signal.
fn diff_with_signal(signal: SemanticSignal) -> SemanticStateDiff {
    SemanticStateDiff {
        forms: FormDiff { added: vec![], removed: vec![], changed: vec![] },
        standalone_actions: ActionDiff { added: vec![], removed: vec![] },
        outputs: OutputDiff { added: vec![], removed: vec![] },
        signals: vec![signal],
    }
}

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
        Some(AgentAction::FillInput { ref form_id, .. }) if form_id == "login"
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
        let raw = extract_screen(&page(p));
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
