use crate::{
    agent::{
        agent::{ Agent, emit_observed_actions, execute_action, execute_action_session },
        agent_model::AgentState,
    },
    browser::{
        playwright::extract_screen,
        session::BrowserSession,
    },
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
pub mod spec;
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

// =========================================================================
// Session-based variants (persistent browser, multi-page flows)
// =========================================================================

/// Take a snapshot using a persistent BrowserSession.
pub fn snapshot_session(
    session: &mut BrowserSession,
) -> Result<(crate::state::state_model::ScreenState, CanonicalScreenState), Box<dyn std::error::Error>> {
    let raw = session.extract()?;
    let dom = raw["dom"]
        .as_array()
        .ok_or("DOM extraction returned no 'dom' array")?;
    let elements: Vec<DomElement> = serde_json::from_value(dom.clone().into())?;

    let url_str = raw["url"].as_str().unwrap_or("unknown");
    let title_str = raw["title"].as_str().unwrap_or("");

    let semantics = classify(&elements);
    let screen_state = build_state(Some(url_str), title_str, semantics);
    let identity_diff = diff(&screen_state, &screen_state);
    let canonical = canonicalize(&screen_state, Some(&identity_diff));

    Ok((screen_state, canonical))
}

/// Run the agent loop using a persistent BrowserSession (multi-page capable).
pub fn run_app_session() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::var("TARGET_URL").unwrap_or_else(|_| "https://google.com".to_string());
    let tracer = TraceLogger::new("agent_trace.jsonl");
    let mut agent = Agent::with_deterministic();
    let mut session = BrowserSession::launch()?;

    println!("=== Starting session agent loop for: {} ===\n", url);

    // Navigate to initial URL
    session.navigate(&url)?;

    // ---- Initial snapshot ----
    let (screen_state, canonical) = snapshot_session(&mut session)?;
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

            match execute_action_session(&action, &current_screen, &mut session) {
                Ok(()) => println!("Action executed successfully"),
                Err(e) => println!("Action failed: {}", e),
            }
        }

        // Check for terminal state
        if matches!(agent.state, AgentState::Stop) {
            println!("\nAgent reached Stop state.");
            break;
        }

        // Take new snapshot after action (using session, no re-navigation needed)
        let (new_screen, new_canonical) = snapshot_session(&mut session)?;

        // Compute diff against previous state
        sem_diff = semantic_diff(&prev_canonical, &new_canonical, false);

        if !sem_diff.signals.is_empty() {
            println!("Signals: {:?}", sem_diff.signals);
        }

        prev_canonical = new_canonical;
        current_screen = new_screen;
    }

    println!("\n=== Session agent loop complete ===");
    session.quit()?;
    Ok(())
}
