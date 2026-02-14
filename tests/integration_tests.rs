//! Integration tests that launch a real BrowserSession (browser_server.js + Playwright).
//!
//! These tests require Node.js + Playwright installed. They are marked `#[ignore]`
//! so they don't run during `cargo test`. Run them with:
//!
//! ```bash
//! cargo test -- --ignored          # only integration tests
//! cargo test -- --include-ignored  # all tests (offline + integration)
//! ```

use std::collections::HashMap;

use screen_detection::agent::page_analyzer::MockPageAnalyzer;
use screen_detection::browser::playwright::SelectorHint;
use screen_detection::browser::session::BrowserSession;
use screen_detection::canonical::diff::{semantic_diff, SemanticSignal};
use screen_detection::explorer::app_map::ExplorerConfig;
use screen_detection::explorer::explorer::explore_live;
use screen_detection::explorer::flow_detector::detect_flows;
use screen_detection::snapshot_session;
use screen_detection::spec::runner::TestRunner;
use screen_detection::spec::spec_model::{AssertionSpec, TestSpec, TestStep};

mod common;
use crate::common::utils::page;

// ============================================================================
// Group A: Session Lifecycle (3 tests)
// ============================================================================

#[test]
#[ignore]
fn test_session_launch() {
    let session = BrowserSession::launch();
    assert!(session.is_ok(), "BrowserSession::launch() should succeed");
    // Drop cleans up the browser process
}

#[test]
#[ignore]
fn test_navigate_and_extract() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();
    let data = session.extract().unwrap();
    let dom = data["dom"].as_array().expect("dom should be an array");
    assert!(!dom.is_empty(), "DOM should contain elements");
}

#[test]
#[ignore]
fn test_navigate_current_url() {
    let mut session = BrowserSession::launch().unwrap();
    let url = page("01_search_form.html");
    session.navigate(&url).unwrap();
    let current = session.current_url().unwrap();
    assert!(
        current.contains("01_search_form.html"),
        "current_url should contain fixture name, got: {}",
        current
    );
}

// ============================================================================
// Group B: DOM Extraction Pipeline (3 tests)
// ============================================================================

#[test]
#[ignore]
fn test_snapshot_search_form_has_forms() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();
    let (screen_state, _canonical) = snapshot_session(&mut session).unwrap();
    assert!(
        !screen_state.forms.is_empty(),
        "Should detect at least one form"
    );
    assert_eq!(screen_state.forms[0].id, "search");
    assert!(
        !screen_state.forms[0].inputs.is_empty(),
        "Form should have inputs"
    );
    assert!(
        !screen_state.forms[0].actions.is_empty(),
        "Form should have actions"
    );
}

#[test]
#[ignore]
fn test_snapshot_error_page_has_outputs() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("05_error.html")).unwrap();
    let (screen_state, _canonical) = snapshot_session(&mut session).unwrap();
    assert!(
        !screen_state.outputs.is_empty(),
        "Error page should have outputs"
    );
    let has_error = screen_state.outputs.iter().any(|o| {
        o.label
            .as_ref()
            .map(|l| l.to_lowercase().contains("invalid"))
            .unwrap_or(false)
    });
    assert!(has_error, "Should find output containing 'invalid'");
}

#[test]
#[ignore]
fn test_snapshot_multiple_forms_detected() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("06_multiple_forms.html")).unwrap();
    let (screen_state, _canonical) = snapshot_session(&mut session).unwrap();
    assert_eq!(screen_state.forms.len(), 2, "Should detect 2 forms");
    let mut ids: Vec<String> = screen_state.forms.iter().map(|f| f.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["newsletter", "search"]);
}

// ============================================================================
// Group C: Query Operations (3 tests)
// ============================================================================

#[test]
#[ignore]
fn test_query_count_search_form_elements() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();
    let input_count = session.query_count("input").unwrap();
    assert_eq!(input_count, 1, "Should have 1 input");
    let button_count = session.query_count("button").unwrap();
    assert_eq!(button_count, 1, "Should have 1 button");
}

#[test]
#[ignore]
fn test_query_text_results_page() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("03_search_results.html")).unwrap();
    let text = session.query_text(".result").unwrap();
    assert!(text.is_some(), "Should find .result element");
    assert!(
        text.unwrap().contains("Result A"),
        "First .result should contain 'Result A'"
    );
}

#[test]
#[ignore]
fn test_query_visible_error_element() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("05_error.html")).unwrap();
    let visible = session.query_visible(".error").unwrap();
    assert!(visible, ".error element should be visible");
}

// ============================================================================
// Group D: Form Interaction (3 tests)
// ============================================================================

#[test]
#[ignore]
fn test_fill_input_no_crash() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();
    let selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("query".into()),
        tag: Some("input".into()),
        input_type: None,
        form_id: Some("search".into()),
    };
    session.fill(&selector, "hello world").unwrap();
    // Verify page is still alive
    let data = session.extract().unwrap();
    assert!(data["dom"].is_array());
}

#[test]
#[ignore]
fn test_fill_and_click_search() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();
    let input_selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("query".into()),
        tag: Some("input".into()),
        input_type: None,
        form_id: Some("search".into()),
    };
    session.fill(&input_selector, "test query").unwrap();
    let button_selector = SelectorHint {
        role: Some("button".into()),
        name: Some("Search".into()),
        tag: None,
        input_type: None,
        form_id: None,
    };
    session.click(&button_selector).unwrap();
}

#[test]
#[ignore]
fn test_fill_login_form() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("05_error.html")).unwrap();
    let email_selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("email".into()),
        tag: Some("input".into()),
        input_type: Some("email".into()),
        form_id: Some("login".into()),
    };
    session.fill(&email_selector, "user@test.com").unwrap();
    let password_selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("password".into()),
        tag: Some("input".into()),
        input_type: Some("password".into()),
        form_id: Some("login".into()),
    };
    session.fill(&password_selector, "secret123").unwrap();
    let login_selector = SelectorHint {
        role: Some("button".into()),
        name: Some("Login".into()),
        tag: None,
        input_type: None,
        form_id: None,
    };
    session.click(&login_selector).unwrap();
}

// ============================================================================
// Group E: TestRunner Full Pipeline (3 tests)
// ============================================================================

#[test]
#[ignore]
fn test_runner_search_page_assertions_pass() {
    let spec = TestSpec {
        name: "Search page smoke test".into(),
        start_url: page("01_search_form.html"),
        steps: vec![
            TestStep::Wait { duration_ms: 500 },
            TestStep::Assert {
                assertions: vec![
                    AssertionSpec::TitleContains {
                        expected: "Search".into(),
                    },
                    AssertionSpec::TextPresent {
                        expected: "Query".into(),
                    },
                ],
            },
        ],
    };
    let mut session = BrowserSession::launch().unwrap();
    let result = TestRunner::run(&spec, &mut session);
    assert!(result.passed, "Test should pass: {:?}", result.error);
    assert_eq!(result.steps_run, 2);
    assert_eq!(result.assertion_results.len(), 2);
    assert!(result.assertion_results.iter().all(|r| r.passed));
    assert!(result.error.is_none());
}

#[test]
#[ignore]
fn test_runner_error_page_element_assertions() {
    let spec = TestSpec {
        name: "Error page element test".into(),
        start_url: page("05_error.html"),
        steps: vec![
            TestStep::Wait { duration_ms: 500 },
            TestStep::Assert {
                assertions: vec![
                    AssertionSpec::ElementText {
                        selector: ".error".into(),
                        expected: "Invalid credentials".into(),
                    },
                    AssertionSpec::ElementVisible {
                        selector: ".error".into(),
                    },
                ],
            },
        ],
    };
    let mut session = BrowserSession::launch().unwrap();
    let result = TestRunner::run(&spec, &mut session);
    assert!(result.passed, "Test should pass: {:?}", result.error);
    assert_eq!(result.assertion_results.len(), 2);
    assert!(result.assertion_results.iter().all(|r| r.passed));
}

#[test]
#[ignore]
fn test_runner_fill_and_submit_steps_run() {
    let mut values = HashMap::new();
    values.insert("query".to_string(), "test".to_string());
    let spec = TestSpec {
        name: "Search form fill test".into(),
        start_url: page("01_search_form.html"),
        steps: vec![
            TestStep::FillAndSubmit {
                form: "search".into(),
                values,
                submit_label: Some("Search".into()),
            },
            TestStep::Wait { duration_ms: 500 },
        ],
    };
    let mut session = BrowserSession::launch().unwrap();
    let result = TestRunner::run(&spec, &mut session);
    assert!(
        result.error.is_none(),
        "Should have no error: {:?}",
        result.error
    );
    assert_eq!(result.steps_run, 2, "Both steps should execute");
}

// ============================================================================
// Group F: Navigation Flow (2 tests)
// ============================================================================

#[test]
#[ignore]
fn test_click_navigation_link() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("08_navigation.html")).unwrap();
    let link_selector = SelectorHint {
        role: None,
        name: Some("Go to results".into()),
        tag: Some("a".into()),
        input_type: None,
        form_id: None,
    };
    session.click(&link_selector).unwrap();
    session.wait_idle(1000).unwrap();
    let url = session.current_url().unwrap();
    assert!(
        url.contains("03_search_results"),
        "Should have navigated to results page, got: {}",
        url
    );
}

#[test]
#[ignore]
fn test_navigate_between_fixtures() {
    let mut session = BrowserSession::launch().unwrap();

    // First page
    session.navigate(&page("01_search_form.html")).unwrap();
    let data1 = session.extract().unwrap();
    let title1 = data1["title"].as_str().unwrap_or("");
    assert_eq!(title1, "Search");

    // Second page
    session.navigate(&page("03_search_results.html")).unwrap();
    let data2 = session.extract().unwrap();
    let title2 = data2["title"].as_str().unwrap_or("");
    assert_eq!(title2, "Results");
}

// ============================================================================
// Group G: Snapshot + Canonical Pipeline (2 tests)
// ============================================================================

#[test]
#[ignore]
fn test_snapshot_canonical_has_entries() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();
    let (_screen_state, canonical) = snapshot_session(&mut session).unwrap();
    assert!(
        !canonical.forms.is_empty(),
        "Canonical should have forms"
    );
    assert!(
        canonical.forms.contains_key("search"),
        "Should have 'search' form"
    );
    assert!(
        !canonical.elements.is_empty(),
        "Canonical should have elements"
    );
    assert_eq!(canonical.title, "Search");
}

#[test]
#[ignore]
fn test_snapshot_diff_detects_signals() {
    let mut session = BrowserSession::launch().unwrap();

    session.navigate(&page("01_search_form.html")).unwrap();
    let (_s1, canonical1) = snapshot_session(&mut session).unwrap();

    session.navigate(&page("03_search_results.html")).unwrap();
    let (_s2, canonical2) = snapshot_session(&mut session).unwrap();

    let diff = semantic_diff(&canonical1, &canonical2, false);
    assert!(
        !diff.signals.is_empty(),
        "Should detect signals between different pages"
    );

    // The search form disappeared and results appeared
    let has_form_submitted = diff.signals.iter().any(|s| {
        matches!(s, SemanticSignal::FormSubmitted { form_id } if form_id == "search")
    });
    let has_results = diff
        .signals
        .iter()
        .any(|s| matches!(s, SemanticSignal::ResultsAppeared));
    assert!(
        has_form_submitted || has_results,
        "Should detect form-submitted or results-appeared, got: {:?}",
        diff.signals
    );
}

// ============================================================================
// Group H: Form-Aware Exploration (4 tests)
// ============================================================================

#[test]
#[ignore]
fn test_form_submit_and_snapshot() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("01_search_form.html")).unwrap();

    // Fill the search form
    let input_selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("query".into()),
        tag: Some("input".into()),
        input_type: None,
        form_id: Some("search".into()),
    };
    session.fill(&input_selector, "test query").unwrap();

    // Click search button
    let button_selector = SelectorHint {
        role: Some("button".into()),
        name: Some("Search".into()),
        tag: None,
        input_type: None,
        form_id: None,
    };
    session.click(&button_selector).unwrap();
    session.wait_idle(500).unwrap();

    // Snapshot the result page
    let result = snapshot_session(&mut session);
    assert!(
        result.is_ok(),
        "snapshot after form submit should succeed: {:?}",
        result.err()
    );
    let (screen_state, _canonical) = result.unwrap();
    assert!(
        !screen_state.title.is_empty(),
        "Result page should have a title"
    );
}

#[test]
#[ignore]
fn test_form_submit_login_and_snapshot() {
    let mut session = BrowserSession::launch().unwrap();
    session.navigate(&page("05_error.html")).unwrap();

    // Fill the login form
    let email_selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("email".into()),
        tag: Some("input".into()),
        input_type: Some("email".into()),
        form_id: Some("login".into()),
    };
    session.fill(&email_selector, "user@test.com").unwrap();

    let password_selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("password".into()),
        tag: Some("input".into()),
        input_type: Some("password".into()),
        form_id: Some("login".into()),
    };
    session.fill(&password_selector, "secret123").unwrap();

    let login_selector = SelectorHint {
        role: Some("button".into()),
        name: Some("Login".into()),
        tag: None,
        input_type: None,
        form_id: None,
    };
    session.click(&login_selector).unwrap();
    session.wait_idle(500).unwrap();

    // Snapshot after submit
    let (screen_state, _canonical) = snapshot_session(&mut session).unwrap();
    // The page may show error outputs (static HTML doesn't really submit)
    assert!(
        !screen_state.title.is_empty(),
        "Result page should have a title"
    );
}

#[test]
#[ignore]
fn test_explore_live_with_forms() {
    let mut session = BrowserSession::launch().unwrap();
    let config = ExplorerConfig {
        start_url: page("01_search_form.html"),
        max_pages: 5,
        max_depth: 2,
        same_origin_only: false,
        explore_forms: true,
        max_forms_per_page: 3,
    };
    let map = explore_live(&config, &mut session, &MockPageAnalyzer).unwrap();

    assert!(
        map.page_count() >= 1,
        "Should discover at least the start page"
    );

    // Check if we have any transitions (either link or form)
    // On static HTML the form submit may stay on the same page, so
    // we check that exploration completed without error
    assert!(
        map.pages.contains_key(&page("01_search_form.html")),
        "Start page should be in the map"
    );
}

#[test]
#[ignore]
fn test_detect_flows_from_live_exploration() {
    let mut session = BrowserSession::launch().unwrap();
    let config = ExplorerConfig {
        start_url: page("01_search_form.html"),
        max_pages: 5,
        max_depth: 2,
        same_origin_only: false,
        explore_forms: true,
        max_forms_per_page: 3,
    };
    let map = explore_live(&config, &mut session, &MockPageAnalyzer).unwrap();

    // detect_flows should work without crashing regardless of what was found
    let flows = detect_flows(&map);
    // On static HTML fixtures, form submit stays on same page,
    // so flows may or may not be detected. Verify no crash and valid output.
    for flow in &flows {
        assert!(!flow.name.is_empty(), "Flow should have a name");
        assert!(!flow.steps.is_empty(), "Flow should have steps");
    }
}
