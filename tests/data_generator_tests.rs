use std::collections::HashMap;

use screen_detection::agent::app_context::AppContext;
use screen_detection::agent::data_generator::DataGenerator;
use screen_detection::agent::page_model::{FieldAnalysis, FieldModel, FieldType, FormModel, PageModel};
use screen_detection::explorer::test_generator::generate_test_plan;
use screen_detection::explorer::app_map::{AppMap, ExplorerConfig, PageNode};

// ============================================================================
// Helpers
// ============================================================================

fn text_field(label: &str) -> FieldModel {
    FieldModel {
        label: label.to_string(),
        field_type: FieldType::Text,
        required: true,
        suggested_test_value: format!("default_{label}"),
    }
}

fn email_field(label: &str) -> FieldModel {
    FieldModel {
        label: label.to_string(),
        field_type: FieldType::Email,
        required: true,
        suggested_test_value: "test@example.com".to_string(),
    }
}

fn hidden_field(label: &str) -> FieldModel {
    FieldModel {
        label: label.to_string(),
        field_type: FieldType::Hidden,
        required: false,
        suggested_test_value: "hidden_value".to_string(),
    }
}

fn field_analysis_with_suggestion(label: &str, suggestion: &str) -> FieldAnalysis {
    FieldAnalysis {
        label: label.to_string(),
        context_clues: "from test".to_string(),
        validation_hint: None,
        suggested_value: Some(suggestion.to_string()),
        negative_values: vec![],
    }
}

fn page_model_with_form(fields: Vec<FieldModel>) -> PageModel {
    PageModel {
        purpose: "Test page".to_string(),
        domain: "test domain".to_string(),
        forms: vec![FormModel {
            form_id: "f1".to_string(),
            purpose: "Test form".to_string(),
            fields,
            submit_label: Some("Submit".to_string()),
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

fn page_node(url: &str, model: PageModel) -> PageNode {
    PageNode {
        url: url.to_string(),
        title: "Test".to_string(),
        depth: 0,
        page_model: model,
    }
}

// ============================================================================
// Phase 15 Step 5: DataGenerator tests
// ============================================================================

#[test]
fn data_generator_context_recall_takes_priority() {
    // Priority 1: AppContext recall beats everything
    let mut ctx = AppContext::new();
    ctx.record_fill("MSISDN", "447700111222");

    let field = text_field("MSISDN");
    let analysis = field_analysis_with_suggestion("MSISDN", "LLM_suggested_value");
    let generator = DataGenerator::new(&ctx);

    let value = generator.generate(&field, Some(&analysis));
    assert_eq!(value, "447700111222", "AppContext recall should beat LLM suggestion");
}

#[test]
fn data_generator_llm_suggestion_used_when_no_recall() {
    // Priority 2: LLM FieldAnalysis suggestion when no recall available
    let ctx = AppContext::new(); // empty

    let field = text_field("MSISDN");
    let analysis = field_analysis_with_suggestion("MSISDN", "447700999888");
    let generator = DataGenerator::new(&ctx);

    let value = generator.generate(&field, Some(&analysis));
    assert_eq!(value, "447700999888", "LLM suggested value should be used when no recall");
}

#[test]
fn data_generator_falls_back_to_guess_value() {
    // Priority 3: guess_value() when no recall and no LLM suggestion
    let ctx = AppContext::new();
    let field = email_field("Email");
    let generator = DataGenerator::new(&ctx);

    let value = generator.generate(&field, None);
    // guess_value("Email", Some("email")) should return something email-like
    assert!(!value.is_empty(), "fallback should produce a value");
    assert!(
        value.contains('@') || value.contains("test") || value.contains("email"),
        "email field fallback should be email-like: {value}"
    );
}

#[test]
fn data_generator_context_recall_case_insensitive() {
    // AppContext stores labels in lowercase; recall must work regardless of query casing
    let mut ctx = AppContext::new();
    ctx.record_fill("MSISDN", "447700123456");

    // Field has different casing than what was stored
    let field = FieldModel {
        label: "msisdn".to_string(),
        field_type: FieldType::Text,
        required: false,
        suggested_test_value: "fallback".to_string(),
    };
    let generator = DataGenerator::new(&ctx);
    let value = generator.generate(&field, None);
    assert_eq!(value, "447700123456");
}

#[test]
fn data_generator_cross_page_msisdn_reuse() {
    // Domain-specific test: MSISDN entered on page 1 recalled on page 3
    let mut ctx = AppContext::new();
    ctx.record_fill("MSISDN", "447700123456");
    ctx.record_fill("ICCID", "8944500102198304251");

    let msisdn_field = text_field("MSISDN");
    let iccid_field = text_field("ICCID");
    let generator = DataGenerator::new(&ctx);

    assert_eq!(generator.generate(&msisdn_field, None), "447700123456");
    assert_eq!(generator.generate(&iccid_field, None), "8944500102198304251");
}

#[test]
fn data_generator_cross_page_account_number_reuse() {
    // Another cross-page scenario: account number from registration reused on payment
    let mut ctx = AppContext::new();
    ctx.record_fill("Account Number", "ACC-12345");

    let field = text_field("Account Number");
    let generator = DataGenerator::new(&ctx);

    assert_eq!(generator.generate(&field, None), "ACC-12345");
}

#[test]
fn data_generator_llm_suggestion_empty_falls_through() {
    // Empty LLM suggestion should fall through to guess_value()
    let ctx = AppContext::new();
    let field = email_field("Email");
    let analysis = FieldAnalysis {
        label: "Email".to_string(),
        context_clues: "".to_string(),
        validation_hint: None,
        suggested_value: Some("".to_string()), // empty! should fall through
        negative_values: vec![],
    };
    let generator = DataGenerator::new(&ctx);
    let value = generator.generate(&field, Some(&analysis));

    // Should fall back to guess_value, not return empty string
    assert!(!value.is_empty(), "empty LLM suggestion should fall through to guess_value");
}

#[test]
fn data_generator_llm_suggestion_none_falls_through() {
    // None LLM analysis should fall through to guess_value()
    let ctx = AppContext::new();
    let field = text_field("First Name");
    let generator = DataGenerator::new(&ctx);
    let value = generator.generate(&field, None);

    assert!(!value.is_empty(), "None analysis should fall through to guess_value");
}

#[test]
fn data_generator_generate_all_skips_hidden_fields() {
    let ctx = AppContext::new();
    let fields = vec![
        text_field("First Name"),
        hidden_field("csrf_token"),
        email_field("Email"),
    ];
    let analyses = HashMap::new();
    let generator = DataGenerator::new(&ctx);
    let results = generator.generate_all(&fields, &analyses);

    // Hidden field should be excluded
    assert_eq!(results.len(), 2, "generate_all should skip hidden fields");
    assert!(results.iter().any(|(k, _)| k == "First Name"));
    assert!(results.iter().any(|(k, _)| k == "Email"));
    assert!(!results.iter().any(|(k, _)| k == "csrf_token"));
}

#[test]
fn data_generator_generate_all_with_analyses() {
    let ctx = AppContext::new();
    let fields = vec![
        text_field("Patient ID"),
        email_field("Email"),
    ];
    let fa_patient = field_analysis_with_suggestion("patient id", "P-999888");
    let analyses: HashMap<String, &FieldAnalysis> = HashMap::from([
        ("patient id".to_string(), &fa_patient),
    ]);
    let generator = DataGenerator::new(&ctx);
    let results = generator.generate_all(&fields, &analyses);

    assert_eq!(results.len(), 2);
    // Patient ID should use LLM suggestion (looked up case-insensitively)
    let patient_value = results.iter().find(|(k, _)| k == "Patient ID").map(|(_, v)| v.as_str());
    assert_eq!(patient_value, Some("P-999888"), "LLM suggestion should be used for Patient ID");
}

#[test]
fn data_generator_generate_all_with_context_recall() {
    let mut ctx = AppContext::new();
    ctx.record_fill("First Name", "John");
    ctx.record_fill("Email", "john@example.com");

    let fields = vec![text_field("First Name"), email_field("Email")];
    let analyses = HashMap::new();
    let generator = DataGenerator::new(&ctx);
    let results = generator.generate_all(&fields, &analyses);

    assert_eq!(results.len(), 2);
    let first_name = results.iter().find(|(k, _)| k == "First Name").map(|(_, v)| v.as_str());
    let email = results.iter().find(|(k, _)| k == "Email").map(|(_, v)| v.as_str());
    assert_eq!(first_name, Some("John"), "should recall First Name from context");
    assert_eq!(email, Some("john@example.com"), "should recall Email from context");
}

#[test]
fn test_plan_uses_app_context_values() {
    // generate_test_plan with app_context should use recalled values
    let mut ctx = AppContext::new();
    ctx.record_fill("MSISDN", "447700123456");
    ctx.record_fill("ICCID", "8944500102198304251");

    let model = page_model_with_form(vec![
        text_field("MSISDN"),
        text_field("ICCID"),
    ]);
    let mut map = AppMap::new();
    map.add_page(page_node("http://example.com/step2", model));

    let specs = generate_test_plan(&map, None, Some(&ctx));
    assert!(!specs.is_empty());

    // Find the form test spec
    let form_spec = specs.iter().find(|s| s.name.contains("Form")).unwrap();
    // Check fill step uses recalled values
    use screen_detection::spec::spec_model::TestStep;
    let fill_step = form_spec.steps.iter().find_map(|s| {
        if let TestStep::FillAndSubmit { values, .. } = s { Some(values) } else { None }
    });
    let fill_values = fill_step.expect("should have FillAndSubmit step");
    assert_eq!(
        fill_values.get("MSISDN").map(|s| s.as_str()),
        Some("447700123456"),
        "MSISDN should come from AppContext recall"
    );
    assert_eq!(
        fill_values.get("ICCID").map(|s| s.as_str()),
        Some("8944500102198304251"),
        "ICCID should come from AppContext recall"
    );
}

#[test]
fn test_plan_backward_compat_no_context() {
    // generate_test_plan with None context uses suggested_test_value
    let model = page_model_with_form(vec![text_field("Username")]);
    let mut map = AppMap::new();
    map.add_page(page_node("http://example.com/login", model));

    let specs = generate_test_plan(&map, None, None);
    assert!(!specs.is_empty());

    use screen_detection::spec::spec_model::TestStep;
    let form_spec = specs.iter().find(|s| s.name.contains("Form")).unwrap();
    let fill_step = form_spec.steps.iter().find_map(|s| {
        if let TestStep::FillAndSubmit { values, .. } = s { Some(values) } else { None }
    });
    let fill_values = fill_step.expect("should have FillAndSubmit step");
    // With no context, uses suggested_test_value ("default_Username")
    assert_eq!(
        fill_values.get("Username").map(|s| s.as_str()),
        Some("default_Username"),
        "no-context plan should use suggested_test_value"
    );
}

#[test]
fn test_plan_value_overrides_beat_context() {
    // value_overrides take priority over AppContext recall
    use screen_detection::cli::config::ValueConfig;
    let mut ctx = AppContext::new();
    ctx.record_fill("Email", "context@example.com");

    let mut vc = ValueConfig::default();
    vc.fields.insert("Email".to_string(), "override@example.com".to_string());

    let model = page_model_with_form(vec![email_field("Email")]);
    let mut map = AppMap::new();
    map.add_page(page_node("http://example.com/register", model));

    let specs = generate_test_plan(&map, Some(&vc), Some(&ctx));
    use screen_detection::spec::spec_model::TestStep;
    let form_spec = specs.iter().find(|s| s.name.contains("Form")).unwrap();
    let fill_step = form_spec.steps.iter().find_map(|s| {
        if let TestStep::FillAndSubmit { values, .. } = s { Some(values) } else { None }
    });
    let fill_values = fill_step.expect("should have FillAndSubmit step");
    assert_eq!(
        fill_values.get("Email").map(|s| s.as_str()),
        Some("override@example.com"),
        "value_overrides should beat AppContext recall"
    );
}

#[test]
fn field_model_input_type_str_email() {
    let f = email_field("Email");
    assert_eq!(f.input_type_str(), Some("email"));
}

#[test]
fn field_model_input_type_str_password() {
    let f = FieldModel {
        label: "Password".to_string(),
        field_type: FieldType::Password,
        required: true,
        suggested_test_value: "pass".to_string(),
    };
    assert_eq!(f.input_type_str(), Some("password"));
}

#[test]
fn field_model_input_type_str_select_returns_none() {
    let f = FieldModel {
        label: "Country".to_string(),
        field_type: FieldType::Select,
        required: false,
        suggested_test_value: "US".to_string(),
    };
    assert_eq!(f.input_type_str(), None);
}
