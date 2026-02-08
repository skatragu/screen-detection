use crate::{
    agent::{
        agent::{ Agent, emit_observed_actions, execute_action },
        agent_model::AgentState,
    },
    browser::playwright::extract_screen,
    canonical::{
        canonical_model::{canonicalize, CanonicalScreenState},
        diff::semantic_diff,
    },
    screen::{classifier::classify, screen_model::DomElement},
    state::{diff::diff, state_builder::build_state},
    trace::logger::TraceLogger,
};

pub mod agent;
pub mod browser;
pub mod canonical;
pub mod screen;
pub mod state;
pub mod trace;

const MAX_ITERATIONS: u32 = 20;

pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("TARGET_URL").unwrap_or_else(|_| "https://google.com".to_string());
    let tracer = TraceLogger::new("agent_trace.jsonl");
    let mut agent = Agent::with_deterministic();

    println!("=== Starting agent loop for: {} ===\n", url);

    // ---- Initial snapshot ----
    let (screen_state, canonical) = snapshot(&url)?;
    let empty_canonical = CanonicalScreenState::empty();
    let mut sem_diff = semantic_diff(&empty_canonical, &canonical, true);

    println!("Initial signals: {:?}", sem_diff.signals);

    let mut prev_canonical = canonical;
    let mut current_screen = screen_state;

    // ---- Agent loop ----
    for iteration in 0..MAX_ITERATIONS {
        println!("\n--- Iteration {} (agent state: {:?}) ---", iteration, agent.state);

        // Feed observed signals back to agent memory
        emit_observed_actions(&sem_diff, &mut agent.memory);

        // Step the agent state machine
        if let Some(action) = agent.step(&current_screen, &sem_diff, &tracer) {
            println!("Agent action: {:?}", action);

            match execute_action(&action, &current_screen) {
                Ok(()) => println!("Action executed successfully"),
                Err(e) => println!("Action failed: {}", e),
            }
        }

        // Check for terminal state
        if matches!(agent.state, AgentState::Stop) {
            println!("\nAgent reached Stop state.");
            break;
        }

        // Take new snapshot after action
        let (new_screen, new_canonical) = snapshot(&url)?;

        // Compute diff against previous state
        sem_diff = semantic_diff(&prev_canonical, &new_canonical, false);

        if !sem_diff.signals.is_empty() {
            println!("Signals: {:?}", sem_diff.signals);
        }

        prev_canonical = new_canonical;
        current_screen = new_screen;
    }

    println!("\n=== Agent loop complete ===");
    Ok(())
}

/// Take a snapshot: extract DOM, classify, build state, canonicalize.
fn snapshot(url: &str) -> Result<(crate::state::state_model::ScreenState, CanonicalScreenState), Box<dyn std::error::Error>> {
    let raw = extract_screen(url)?;
    let dom = raw["dom"]
        .as_array()
        .ok_or("DOM extraction returned no 'dom' array")?;
    let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into())?;

    let semantics = classify(&elements);
    let screen_state = build_state(Some(url), raw["title"].as_str().unwrap_or(""), semantics);
    let identity_diff = diff(&screen_state, &screen_state); // self-diff for canonicalization
    let canonical = canonicalize(&screen_state, Some(&identity_diff));

    Ok((screen_state, canonical))
}
