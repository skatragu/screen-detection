use crate::{
    browser::playwright::extract_screen,
    canonical::{canonical_model::canonicalize, diff::semantic_diff},
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

pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let _tracer = TraceLogger::new("agent_trace.jsonl");
    let url = "https://google.com";

    /* ---------- BASELINE STATE (EMPTY) ---------- */

    // ---- First snapshot ----
    let raw1 = extract_screen(url);
    let dom1 = raw1["dom"].as_array().unwrap();
    let elements1: Vec<DomElement> = serde_json::from_value(dom1.clone().into())?;

    let semantics1 = classify(&elements1);
    let state1 = build_state(Some(url), raw1["title"].as_str().unwrap_or(""), semantics1);

    // ---- Second snapshot (simulate after action for now) ----
    let raw2 = extract_screen(url);
    let dom2 = raw2["dom"].as_array().unwrap();
    let elements2: Vec<DomElement> = serde_json::from_value(dom2.clone().into()).unwrap();

    let semantics2 = classify(&elements2);

    println!("=== SCREEN SEMANTICS ===\n");
    let state2 = build_state(Some(url), raw2["title"].as_str().unwrap_or(""), semantics2);

    // ---- Diff ----
    let identity_diff = diff(&state1, &state2);

    /* ---------- CANONICALIZATION (NEW) ---------- */

    //Identity diff is just added, but not used right now, focus right
    // now is mostly on sematic diff
    let canonical1 = canonicalize(&state1, Some(&identity_diff));
    let canonical2 = canonicalize(&state2, Some(&identity_diff));

    /* ---------- SEMANTIC DIFF (NEW) ---------- */

    let semantic_diff = semantic_diff(&canonical1, &canonical2, true);
    println!("=== SEMANTIC STATE DIFF ===\n{:#?}", semantic_diff);

    Ok(())
}
