use std::collections::HashMap;

use crate::agent::app_context::AppContext;
use crate::agent::page_model::{FormModel, PageModel, SuggestedAssertion};
use crate::cli::config::ValueConfig;
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
/// 2. A **form test** per form — fill fields with suggested/overridden values, submit
///
/// Priority order for field values:
/// 1. `value_overrides` — explicit user-provided values (highest priority)
/// 2. `app_context.recall()` — values entered on previous pages during exploration
/// 3. `field.suggested_test_value` — AI-suggested or deterministic value (fallback)
///
/// `app_context` is optional for backward compatibility — existing callers pass `None`.
///
/// Returns a `Vec<TestSpec>` ready for serialization to YAML for human review.
pub fn generate_test_plan(
    app_map: &AppMap,
    value_overrides: Option<&ValueConfig>,
    app_context: Option<&AppContext>,
) -> Vec<TestSpec> {
    let mut specs = Vec::new();

    // Sort by URL for deterministic output order
    let mut urls: Vec<&String> = app_map.pages.keys().collect();
    urls.sort();

    for url in urls {
        let node = &app_map.pages[url];
        specs.push(generate_smoke_test(url, &node.page_model));

        for form in &node.page_model.forms {
            specs.push(generate_form_test_with_overrides(url, form, &node.page_model, value_overrides, app_context));
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
///
/// Also adds a URL path assertion if the URL has a meaningful path component.
pub fn generate_smoke_test(url: &str, model: &PageModel) -> TestSpec {
    let mut steps = vec![TestStep::Wait { duration_ms: 1000 }];

    let mut assertions: Vec<AssertionSpec> = model
        .suggested_assertions
        .iter()
        .filter_map(|sa| map_suggested_assertion(sa))
        .collect();

    // Add URL path assertion if there's a meaningful path and no URL assertion yet
    if let Some(path) = extract_url_path(url) {
        if !assertions.iter().any(|a| matches!(a, AssertionSpec::UrlContains { .. })) {
            assertions.push(AssertionSpec::UrlContains { expected: path });
        }
    }

    if !assertions.is_empty() {
        steps.push(TestStep::Assert { assertions });
    }

    TestSpec {
        name: format!("Smoke: {}", model.purpose),
        start_url: url.to_string(),
        steps,
    }
}

/// Generate a form fill-and-submit test with optional value overrides and context recall.
///
/// Priority: value_overrides > app_context recall > suggested_test_value.
/// The page's free-form `domain` string is used for domain-scoped overrides.
fn generate_form_test_with_overrides(
    url: &str,
    form: &FormModel,
    model: &PageModel,
    value_overrides: Option<&ValueConfig>,
    app_context: Option<&AppContext>,
) -> TestSpec {
    let values: HashMap<String, String> = form
        .fields
        .iter()
        .map(|f| {
            let value = value_overrides
                .and_then(|v| v.resolve(&f.label, Some(&model.domain)))
                .cloned()
                // Cross-page recall: if this label was filled on a previous page, reuse it
                .or_else(|| {
                    app_context
                        .and_then(|ctx| ctx.recall(&f.label))
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| f.suggested_test_value.clone());
            (f.label.clone(), value)
        })
        .collect();

    build_form_test_spec(url, form, model, values)
}

/// Generate a form fill-and-submit test.
///
/// Uses `suggested_test_value` from each `FieldModel` (populated by
/// `MockPageAnalyzer` via `guess_value()` or by `LlmPageAnalyzer` via AI).
///
/// Adds post-submit assertions based on `form.expected_outcome`:
/// - `UrlContains` / `UrlNotContains` for URL patterns
/// - `TextPresent` for first success indicator
/// - `TextAbsent` for all error indicators
pub fn generate_form_test(url: &str, form: &FormModel, model: &PageModel) -> TestSpec {
    let values: HashMap<String, String> = form
        .fields
        .iter()
        .map(|f| (f.label.clone(), f.suggested_test_value.clone()))
        .collect();

    build_form_test_spec(url, form, model, values)
}

/// Build a form TestSpec from pre-resolved values + expected_outcome assertions.
fn build_form_test_spec(url: &str, form: &FormModel, model: &PageModel, values: HashMap<String, String>) -> TestSpec {

    let fill_step = TestStep::FillAndSubmit {
        form: form.form_id.clone(),
        values,
        submit_label: form.submit_label.clone(),
    };

    let mut steps = vec![fill_step, TestStep::Wait { duration_ms: 1000 }];

    // Build post-submit assertions from expected_outcome
    let outcome = &form.expected_outcome;
    let mut assertions = Vec::new();

    if let Some(url_pat) = &outcome.url_contains {
        assertions.push(AssertionSpec::UrlContains { expected: url_pat.clone() });
    }
    if let Some(url_not) = &outcome.url_not_contains {
        assertions.push(AssertionSpec::UrlNotContains { expected: url_not.clone() });
    }
    // Add first success indicator (avoid over-asserting on text)
    if let Some(text) = outcome.success_text.first() {
        assertions.push(AssertionSpec::TextPresent { expected: text.clone() });
    }
    // All error indicators should be absent after success
    for text in &outcome.error_indicators {
        assertions.push(AssertionSpec::TextAbsent { expected: text.clone() });
    }

    if !assertions.is_empty() {
        steps.push(TestStep::Assert { assertions });
    }

    TestSpec {
        name: format!("Form: {} on {}", form.purpose, model.purpose),
        start_url: url.to_string(),
        steps,
    }
}

/// Extract the path component from a URL (e.g. `"/login"` from `"https://example.com/login"`).
fn extract_url_path(url: &str) -> Option<String> {
    let after_scheme = url.find("://").map(|i| i + 3)?;
    let slash_pos = url[after_scheme..].find('/').map(|i| after_scheme + i)?;
    let path = &url[slash_pos..];
    // Strip query and fragment
    let path = path.split('?').next().unwrap_or(path);
    let path = path.split('#').next().unwrap_or(path);
    if path == "/" || path.is_empty() {
        None
    } else {
        Some(path.to_string())
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
        "url_not_contains" => Some(AssertionSpec::UrlNotContains {
            expected: sa.expected.clone(),
        }),
        "element_visible" => Some(AssertionSpec::ElementVisible {
            selector: sa.expected.clone(),
        }),
        _ => None,
    }
}
