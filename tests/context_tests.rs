use std::collections::HashMap;

use screen_detection::agent::{AppContext, PageSummary};
use screen_detection::agent::page_analyzer::{
    build_rich_page_prompt, LlmPageAnalyzer, MockPageAnalyzer, PageAnalyzer,
};
use screen_detection::agent::page_model::{FieldModel, FieldType, FormModel, PageModel};
use screen_detection::screen::screen_model::{
    ElementKind, Form, HeadingEntry, LandmarkEntry, ScreenElement, StructuralOutline,
};
use screen_detection::state::state_model::ScreenState;

// ============================================================================
// Helpers
// ============================================================================

fn empty_screen() -> ScreenState {
    ScreenState {
        url: Some("http://example.com".to_string()),
        title: "Test Page".to_string(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    }
}

fn page_model_with_purpose(purpose: &str, domain: &str) -> PageModel {
    PageModel {
        purpose: purpose.to_string(),
        domain: domain.to_string(),
        forms: vec![FormModel {
            form_id: "f1".to_string(),
            purpose: "Entry form".to_string(),
            fields: vec![
                FieldModel {
                    label: "MSISDN".to_string(),
                    field_type: FieldType::Text,
                    required: true,
                    suggested_test_value: "447700123456".to_string(),
                },
                FieldModel {
                    label: "ICCID".to_string(),
                    field_type: FieldType::Text,
                    required: true,
                    suggested_test_value: "8944500102198304251".to_string(),
                },
            ],
            submit_label: Some("Next".to_string()),
            expected_outcome: Default::default(),
        }],
        outputs: vec![],
        suggested_assertions: vec![],
        navigation_targets: vec![],
        expected_outcome: Default::default(),
        layout_description: None,
        field_analyses: vec![],
        suggested_test_scenarios: vec![],
    }
}

// ============================================================================
// Phase 15 Steps 3+4: AppContext tests
// ============================================================================

#[test]
fn app_context_record_and_recall() {
    let mut ctx = AppContext::new();
    ctx.record_fill("MSISDN", "447700123456");
    assert_eq!(ctx.recall("MSISDN"), Some("447700123456"));
}

#[test]
fn app_context_recall_case_insensitive() {
    let mut ctx = AppContext::new();
    ctx.record_fill("MSISDN", "447700123456");
    // Recall should work regardless of casing of both stored and query labels
    assert_eq!(ctx.recall("msisdn"), Some("447700123456"));
    assert_eq!(ctx.recall("Msisdn"), Some("447700123456"));
    assert_eq!(ctx.recall("MSISDN"), Some("447700123456"));
}

#[test]
fn app_context_recall_unknown_label_returns_none() {
    let ctx = AppContext::new();
    assert_eq!(ctx.recall("MSISDN"), None);
}

#[test]
fn app_context_build_context_summary_empty() {
    let ctx = AppContext::new();
    // First page: no prior context to include
    assert_eq!(ctx.build_context_summary(), "");
    assert!(ctx.is_empty());
    assert_eq!(ctx.pages_visited(), 0);
}

#[test]
fn app_context_build_context_summary_with_pages() {
    let mut ctx = AppContext::new();
    let model = page_model_with_purpose("SIM card details entry", "telecom SIM provisioning");
    let values = HashMap::from([
        ("MSISDN".to_string(), "447700123456".to_string()),
        ("ICCID".to_string(), "8944500102198304251".to_string()),
    ]);
    ctx.record_page("http://example.com/step1", &model, values);

    let summary = ctx.build_context_summary();
    assert!(summary.contains("step1"), "summary should mention URL");
    assert!(summary.contains("SIM card details entry"), "summary should mention page purpose");
    assert!(summary.contains("msisdn: 447700123456"), "summary should include entered data");
    assert!(summary.contains("iccid: 8944500102198304251"), "summary should include ICCID");
    assert!(
        summary.contains("Pages visited so far"),
        "summary should have 'Pages visited' section"
    );
    assert!(
        summary.contains("Data entered so far"),
        "summary should have 'Data entered' section"
    );
}

#[test]
fn app_context_build_context_summary_with_domain() {
    let mut ctx = AppContext::new();
    ctx.update_domain(Some("telecom SIM provisioning".to_string()));
    let model = page_model_with_purpose("Step 1", "telecom SIM provisioning");
    ctx.record_page("http://example.com/step1", &model, HashMap::new());

    let summary = ctx.build_context_summary();
    assert!(
        summary.contains("Application domain: telecom SIM provisioning"),
        "summary should include domain"
    );
}

#[test]
fn app_context_update_domain() {
    let mut ctx = AppContext::new();
    assert!(ctx.domain.is_none());

    ctx.update_domain(Some("hospital patient intake".to_string()));
    assert_eq!(ctx.domain, Some("hospital patient intake".to_string()));

    // Empty string should not overwrite existing domain
    ctx.update_domain(Some("".to_string()));
    assert_eq!(ctx.domain, Some("hospital patient intake".to_string()));

    // None should not overwrite existing domain
    ctx.update_domain(None);
    assert_eq!(ctx.domain, Some("hospital patient intake".to_string()));
}

#[test]
fn app_context_record_page_updates_entered_values() {
    let mut ctx = AppContext::new();
    let model = page_model_with_purpose("SIM card details", "telecom");
    let values = HashMap::from([
        ("MSISDN".to_string(), "447700123456".to_string()),
        ("Service Plan".to_string(), "Starter 10GB".to_string()),
    ]);
    ctx.record_page("http://example.com/step1", &model, values);

    // Cross-page recall should work immediately after record_page
    assert_eq!(ctx.recall("MSISDN"), Some("447700123456"));
    assert_eq!(ctx.recall("service plan"), Some("Starter 10GB"));
    assert_eq!(ctx.pages_visited(), 1);
    assert!(!ctx.is_empty());
}

#[test]
fn app_context_cross_page_recall_after_multiple_pages() {
    let mut ctx = AppContext::new();

    // Page 1: SIM card details
    let model1 = page_model_with_purpose("SIM card details", "telecom SIM provisioning");
    let values1 = HashMap::from([("MSISDN".to_string(), "447700123456".to_string())]);
    ctx.record_page("http://example.com/step1", &model1, values1);
    ctx.update_domain(Some("telecom SIM provisioning".to_string()));

    // Page 2: address entry (no MSISDN field)
    let model2 = page_model_with_purpose("Address entry", "telecom SIM provisioning");
    let values2 = HashMap::from([
        ("Street".to_string(), "123 Main St".to_string()),
        ("City".to_string(), "London".to_string()),
    ]);
    ctx.record_page("http://example.com/step2", &model2, values2);

    // Page 3: confirmation — MSISDN from page 1 should still be recalled
    assert_eq!(ctx.recall("MSISDN"), Some("447700123456"));
    assert_eq!(ctx.recall("street"), Some("123 Main St"));
    assert_eq!(ctx.pages_visited(), 2);

    let summary = ctx.build_context_summary();
    assert!(summary.contains("step1"));
    assert!(summary.contains("step2"));
    assert!(summary.contains("Application domain: telecom SIM provisioning"));
}

#[test]
fn app_context_serde_roundtrip() {
    let mut ctx = AppContext::new();
    ctx.update_domain(Some("HR leave management".to_string()));
    ctx.inferred_flow = Some("3-step leave request".to_string());
    let model = page_model_with_purpose("Leave request form", "HR leave management");
    let values = HashMap::from([("Employee ID".to_string(), "EMP-42".to_string())]);
    ctx.record_page("http://hr.example.com/leave", &model, values);

    let json = serde_json::to_string(&ctx).unwrap();
    let parsed: AppContext = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.domain, Some("HR leave management".to_string()));
    assert_eq!(parsed.inferred_flow, Some("3-step leave request".to_string()));
    assert_eq!(parsed.recall("employee id"), Some("EMP-42"));
    assert_eq!(parsed.pages_visited(), 1);
    assert_eq!(parsed.page_summaries[0].url, "http://hr.example.com/leave");
    assert_eq!(parsed.page_summaries[0].purpose, "Leave request form");
}

#[test]
fn app_context_page_summary_fields_seen() {
    let mut ctx = AppContext::new();
    let model = page_model_with_purpose("SIM card details", "telecom");
    // model has 2 fields: MSISDN and ICCID
    ctx.record_page("http://example.com/step1", &model, HashMap::new());

    assert_eq!(ctx.page_summaries[0].fields_seen, vec!["MSISDN", "ICCID"]);
}

#[test]
fn rich_prompt_includes_context_summary() {
    let mut ctx = AppContext::new();
    let model = page_model_with_purpose("SIM card details entry", "telecom SIM provisioning");
    let values = HashMap::from([("MSISDN".to_string(), "447700123456".to_string())]);
    ctx.record_page("http://example.com/step1", &model, values);
    ctx.update_domain(Some("telecom SIM provisioning".to_string()));

    let screen = empty_screen();
    let prompt = build_rich_page_prompt(&screen, Some(&ctx));

    assert!(
        prompt.contains("ACCUMULATED CONTEXT FROM PREVIOUS PAGES"),
        "prompt should include accumulated context section"
    );
    assert!(
        prompt.contains("telecom SIM provisioning"),
        "prompt should include domain from context"
    );
    assert!(
        prompt.contains("SIM card details entry"),
        "prompt should include prior page purpose"
    );
    assert!(
        prompt.contains("msisdn: 447700123456"),
        "prompt should include entered data from context"
    );
}

#[test]
fn rich_prompt_no_context_section_when_empty() {
    let ctx = AppContext::new(); // empty — no pages visited
    let screen = empty_screen();
    let prompt = build_rich_page_prompt(&screen, Some(&ctx));

    // First page: context should not appear (it's empty)
    assert!(
        !prompt.contains("ACCUMULATED CONTEXT FROM PREVIOUS PAGES"),
        "first-page prompt should not include accumulated context section"
    );
}

#[test]
fn rich_prompt_no_context_section_when_none() {
    let screen = empty_screen();
    let prompt = build_rich_page_prompt(&screen, None);

    assert!(
        !prompt.contains("ACCUMULATED CONTEXT FROM PREVIOUS PAGES"),
        "prompt with None context should not include context section"
    );
}

#[test]
fn llm_analyzer_with_context_updates_model_domain() {
    let mock_response = r#"{
        "purpose": "Address entry for SIM provisioning",
        "domain": "telecom SIM provisioning wizard step 2",
        "layout_description": "Single column address form",
        "field_values": {},
        "field_analyses": [],
        "success_indicators": [],
        "error_indicators": [],
        "test_scenarios": []
    }"#;

    let mut ctx = AppContext::new();
    // Prior page established context
    let prior_model = page_model_with_purpose("SIM card details", "telecom SIM provisioning");
    ctx.record_page("http://example.com/step1", &prior_model, HashMap::new());
    ctx.update_domain(Some("telecom SIM provisioning".to_string()));

    let screen = empty_screen();
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);
    let model = analyzer.analyze_with_context(&screen, &ctx).unwrap();

    // LLM's domain from the response should be applied to the model
    assert_eq!(model.domain, "telecom SIM provisioning wizard step 2");
    assert_eq!(
        model.layout_description,
        Some("Single column address form".to_string())
    );
}

#[test]
fn llm_analyzer_analyze_delegates_to_analyze_with_context() {
    // analyze() should produce the same domain-aware result as analyze_with_context() with empty ctx
    let mock_response = r#"{
        "purpose": "Login page",
        "domain": "web app authentication",
        "field_values": {},
        "field_analyses": [],
        "test_scenarios": []
    }"#;

    let screen = empty_screen();
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.domain, "web app authentication");
}

#[test]
fn mock_analyzer_analyze_with_context_ignores_context() {
    // MockPageAnalyzer's analyze_with_context() should behave identically to analyze()
    let mut ctx = AppContext::new();
    ctx.update_domain(Some("some domain".to_string()));
    ctx.record_fill("Email", "prior@example.com");

    let screen = empty_screen();
    let analyzer = MockPageAnalyzer;
    let model_no_ctx = analyzer.analyze(&screen).unwrap();
    let model_with_ctx = analyzer.analyze_with_context(&screen, &ctx).unwrap();

    // Both should produce the same result (context is ignored by MockPageAnalyzer)
    assert_eq!(model_no_ctx.purpose, model_with_ctx.purpose);
    assert_eq!(model_no_ctx.domain, model_with_ctx.domain);
}
