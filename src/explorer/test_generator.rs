use std::collections::HashMap;

use crate::agent::page_model::{FormModel, PageModel, SuggestedAssertion};
use crate::spec::spec_model::{AssertionSpec, TestSpec, TestStep};

use super::app_map::{AppMap, Flow, FlowStep};
use super::flow_detector::detect_flows;

// ============================================================================
// Test plan generation from AppMap
// ============================================================================

/// Generate test specs from a completed AppMap.
///
/// For each discovered page, generates:
/// 1. A **smoke test** — page loads, wait, verify suggested assertions
/// 2. A **form test** per form — fill fields with suggested values, submit
///
/// Returns a `Vec<TestSpec>` ready for serialization to YAML for human review.
pub fn generate_test_plan(app_map: &AppMap) -> Vec<TestSpec> {
    let mut specs = Vec::new();

    // Sort by URL for deterministic output order
    let mut urls: Vec<&String> = app_map.pages.keys().collect();
    urls.sort();

    for url in urls {
        let node = &app_map.pages[url];
        specs.push(generate_smoke_test(url, &node.page_model));

        for form in &node.page_model.forms {
            specs.push(generate_form_test(url, form, &node.page_model));
        }
    }

    // Flow tests from detected multi-step journeys
    let flows = detect_flows(app_map);
    specs.extend(generate_flow_tests(&flows));

    specs
}

// ============================================================================
// Individual test generators
// ============================================================================

/// Generate a smoke test: navigate to page, wait, assert suggested assertions.
pub fn generate_smoke_test(url: &str, model: &PageModel) -> TestSpec {
    let mut steps = vec![TestStep::Wait { duration_ms: 1000 }];

    let assertions: Vec<AssertionSpec> = model
        .suggested_assertions
        .iter()
        .filter_map(|sa| map_suggested_assertion(sa))
        .collect();

    if !assertions.is_empty() {
        steps.push(TestStep::Assert { assertions });
    }

    TestSpec {
        name: format!("Smoke: {}", model.purpose),
        start_url: url.to_string(),
        steps,
    }
}

/// Generate a form fill-and-submit test.
///
/// Uses `suggested_test_value` from each `FieldModel` (populated by
/// `MockPageAnalyzer` via `guess_value()` or by `LlmPageAnalyzer` via AI).
pub fn generate_form_test(url: &str, form: &FormModel, model: &PageModel) -> TestSpec {
    let values: HashMap<String, String> = form
        .fields
        .iter()
        .map(|f| (f.label.clone(), f.suggested_test_value.clone()))
        .collect();

    let fill_step = TestStep::FillAndSubmit {
        form: form.form_id.clone(),
        values,
        submit_label: form.submit_label.clone(),
    };

    TestSpec {
        name: format!("Form: {} on {}", form.purpose, model.purpose),
        start_url: url.to_string(),
        steps: vec![fill_step],
    }
}

// ============================================================================
// Assertion mapping
// ============================================================================

// ============================================================================
// Flow-based test generation
// ============================================================================

/// Generate multi-step flow tests from detected flows.
///
/// Each flow becomes a single `TestSpec` with Navigate and FillAndSubmit steps
/// that replay the discovered multi-page journey.
pub fn generate_flow_tests(flows: &[Flow]) -> Vec<TestSpec> {
    flows
        .iter()
        .map(|flow| {
            let start_url = flow
                .steps
                .iter()
                .find_map(|s| match s {
                    FlowStep::Navigate { url } => Some(url.clone()),
                    FlowStep::FillAndSubmit { url, .. } => Some(url.clone()),
                })
                .unwrap_or_default();

            let steps: Vec<TestStep> = flow
                .steps
                .iter()
                .map(|s| match s {
                    FlowStep::Navigate { url } => TestStep::Navigate { url: url.clone() },
                    FlowStep::FillAndSubmit {
                        form_id,
                        values,
                        submit_label,
                        ..
                    } => TestStep::FillAndSubmit {
                        form: form_id.clone(),
                        values: values.clone(),
                        submit_label: submit_label.clone(),
                    },
                })
                .collect();

            TestSpec {
                name: flow.name.clone(),
                start_url,
                steps,
            }
        })
        .collect()
}

// ============================================================================
// Assertion mapping
// ============================================================================

/// Map a `SuggestedAssertion` (from `PageModel`) to an `AssertionSpec` (for `TestSpec`).
///
/// Returns `None` for unrecognized assertion types, which are silently skipped
/// during test generation.
pub fn map_suggested_assertion(sa: &SuggestedAssertion) -> Option<AssertionSpec> {
    match sa.assertion_type.as_str() {
        "title_contains" => Some(AssertionSpec::TitleContains {
            expected: sa.expected.clone(),
        }),
        "text_present" => Some(AssertionSpec::TextPresent {
            expected: sa.expected.clone(),
        }),
        "text_absent" => Some(AssertionSpec::TextAbsent {
            expected: sa.expected.clone(),
        }),
        "url_contains" => Some(AssertionSpec::UrlContains {
            expected: sa.expected.clone(),
        }),
        "url_equals" => Some(AssertionSpec::UrlEquals {
            expected: sa.expected.clone(),
        }),
        "element_visible" => Some(AssertionSpec::ElementVisible {
            selector: sa.expected.clone(),
        }),
        _ => None,
    }
}
