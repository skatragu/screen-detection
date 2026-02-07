use screen_detection::{
    agent::{
        agent::{Agent, execute_action, gate_decision},
        agent_model::{AgentAction, AgentMemory, DecisionType, MAX_LOOP_REPEATS, ModelDecision},
    },
    browser::playwright::extract_screen,
    canonical::{
        canonical_model::{CanonicalScreenState, canonicalize},
        diff::{SemanticSignal, SemanticStateDiff, semantic_diff},
    },
    screen::{classifier::classify, screen_model::DomElement},
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
