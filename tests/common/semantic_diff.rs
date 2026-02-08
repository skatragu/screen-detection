use std::thread::sleep;
use std::time::Duration;

use screen_detection::browser::playwright::extract_screen;
use screen_detection::canonical::diff::SemanticStateDiff;
use screen_detection::canonical::{canonical_model::canonicalize, diff::semantic_diff};
use screen_detection::screen::{classifier::classify, screen_model::DomElement};
use screen_detection::state::{diff::diff, state_builder::build_state};
use serde_json::Value;

fn run_diff_from_raw(raw1: Value, raw2: Value, is_initial: bool) -> SemanticStateDiff {
    let dom1 = raw1["dom"].as_array().unwrap();
    let dom2 = raw2["dom"].as_array().unwrap();

    let elements1: Vec<DomElement> = serde_json::from_value(dom1.clone().into()).unwrap();
    let elements2: Vec<DomElement> = serde_json::from_value(dom2.clone().into()).unwrap();

    let semantics1 = classify(&elements1);
    let semantics2 = classify(&elements2);

    let state1 = build_state(
        Some(raw1["url"].as_str().unwrap_or("test://fixture")),
        raw1["title"].as_str().unwrap_or(""),
        semantics1,
    );

    let state2 = build_state(
        Some(raw1["url"].as_str().unwrap_or("test://fixture")),
        raw2["title"].as_str().unwrap_or(""),
        semantics2,
    );

    let identity_diff = diff(&state1, &state2);
    let canonical1 = canonicalize(&state1, Some(&identity_diff));
    let canonical2 = canonicalize(&state2, Some(&identity_diff));

    semantic_diff(&canonical1, &canonical2, is_initial)
}

pub fn diff_static(url: &str) -> SemanticStateDiff {
    let raw1 = extract_screen(url).unwrap();
    let raw2 = extract_screen(url).unwrap();

    run_diff_from_raw(raw1, raw2, true)
}

pub fn diff_static_with_initial_flag(url: &str, is_initial: bool) -> SemanticStateDiff {
    let raw1 = extract_screen(url).unwrap();
    let raw2 = extract_screen(url).unwrap();

    run_diff_from_raw(raw1, raw2, is_initial)
}

pub fn diff_mutating(url: &str, wait_ms: u64) -> SemanticStateDiff {
    let raw1 = extract_screen(url).unwrap();

    sleep(Duration::from_millis(wait_ms));

    let raw2 = extract_screen(url).unwrap();

    run_diff_from_raw(raw1, raw2, false)
}

pub fn diff_between_pages(url1: &str, url2: &str) -> SemanticStateDiff {
    let raw1 = extract_screen(url1).unwrap();
    let raw2 = extract_screen(url2).unwrap();

    run_diff_from_raw(raw1, raw2, false)
}
