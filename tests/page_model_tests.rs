use std::collections::HashMap;

use screen_detection::agent::page_analyzer::{
    classify_field_type, LlmPageAnalyzer, MockPageAnalyzer, PageAnalyzer,
};
use screen_detection::agent::page_model::{
    FieldModel, FieldType, FormModel, NavigationTarget, OutputModel, PageCategory, PageModel,
    SuggestedAssertion,
};
use screen_detection::screen::screen_model::{
    ElementKind, Form, FormIntent, IntentSignal, ScreenElement,
};
use screen_detection::state::state_model::ScreenState;

// ============================================================================
// Helper builders
// ============================================================================

fn login_screen() -> ScreenState {
    ScreenState {
        url: Some("https://example.com/login".into()),
        title: "Login - Example App".into(),
        forms: vec![Form {
            id: "login".into(),
            inputs: vec![
                ScreenElement {
                    label: Some("Email".into()),
                    kind: ElementKind::Input,
                    tag: Some("input".into()),
                    role: Some("textbox".into()),
                    input_type: Some("email".into()),
                },
                ScreenElement {
                    label: Some("Password".into()),
                    kind: ElementKind::Input,
                    tag: Some("input".into()),
                    role: Some("textbox".into()),
                    input_type: Some("password".into()),
                },
            ],
            actions: vec![ScreenElement {
                label: Some("Sign In".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
            }],
            primary_action: Some(ScreenElement {
                label: Some("Sign In".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
            }),
            intent: Some(FormIntent {
                label: "Authentication".into(),
                confidence: 0.8,
                signals: vec![
                    IntentSignal::InputType("password".into()),
                    IntentSignal::ActionLabel("Sign In".into()),
                ],
            }),
        }],
        standalone_actions: vec![
            ScreenElement {
                label: Some("Sign Up".into()),
                kind: ElementKind::Action,
                tag: Some("a".into()),
                role: Some("link".into()),
                input_type: None,
            },
            ScreenElement {
                label: Some("Forgot Password".into()),
                kind: ElementKind::Action,
                tag: Some("a".into()),
                role: Some("link".into()),
                input_type: None,
            },
        ],
        outputs: vec![],
        identities: HashMap::new(),
    }
}

fn search_screen() -> ScreenState {
    ScreenState {
        url: Some("https://example.com/search".into()),
        title: "Search Products".into(),
        forms: vec![Form {
            id: "search".into(),
            inputs: vec![ScreenElement {
                label: Some("Search query".into()),
                kind: ElementKind::Input,
                tag: Some("input".into()),
                role: Some("searchbox".into()),
                input_type: Some("text".into()),
            }],
            actions: vec![ScreenElement {
                label: Some("Search".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
            }],
            primary_action: Some(ScreenElement {
                label: Some("Search".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
            }),
            intent: Some(FormIntent {
                label: "Search".into(),
                confidence: 0.5,
                signals: vec![],
            }),
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    }
}

fn empty_screen() -> ScreenState {
    ScreenState {
        url: Some("https://example.com".into()),
        title: "Welcome".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
    }
}

fn sample_page_model() -> PageModel {
    PageModel {
        purpose: "Login page for user authentication".into(),
        category: PageCategory::Login,
        forms: vec![FormModel {
            form_id: "login".into(),
            purpose: "Authentication".into(),
            fields: vec![
                FieldModel {
                    label: "Email".into(),
                    field_type: FieldType::Email,
                    required: true,
                    suggested_test_value: "user@example.com".into(),
                },
                FieldModel {
                    label: "Password".into(),
                    field_type: FieldType::Password,
                    required: true,
                    suggested_test_value: "TestPass123!".into(),
                },
            ],
            submit_label: Some("Sign In".into()),
        }],
        outputs: vec![OutputModel {
            description: "Welcome message".into(),
            region: "main".into(),
        }],
        suggested_assertions: vec![SuggestedAssertion {
            assertion_type: "title_contains".into(),
            expected: "Login".into(),
            description: "Verify page title contains 'Login'".into(),
        }],
        navigation_targets: vec![NavigationTarget {
            label: "Sign Up".into(),
            likely_destination: "Registration page".into(),
        }],
    }
}

// ============================================================================
// 1. PageModel JSON roundtrip
// ============================================================================

#[test]
fn page_model_json_roundtrip() {
    let model = sample_page_model();
    let json = serde_json::to_string(&model).unwrap();
    let parsed: PageModel = serde_json::from_str(&json).unwrap();
    assert_eq!(model, parsed);
}

// ============================================================================
// 2. PageModel YAML roundtrip
// ============================================================================

#[test]
fn page_model_yaml_roundtrip() {
    let model = sample_page_model();
    let yaml = serde_yaml::to_string(&model).unwrap();
    let parsed: PageModel = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(model, parsed);
}

// ============================================================================
// 3. All PageCategory variants serialize/deserialize
// ============================================================================

#[test]
fn page_category_serde() {
    let categories = vec![
        PageCategory::Login,
        PageCategory::Registration,
        PageCategory::Search,
        PageCategory::Dashboard,
        PageCategory::Settings,
        PageCategory::Listing,
        PageCategory::Detail,
        PageCategory::Checkout,
        PageCategory::Error,
        PageCategory::Other,
    ];

    for cat in &categories {
        let json = serde_json::to_string(cat).unwrap();
        let parsed: PageCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(*cat, parsed, "Failed roundtrip for {:?}", cat);
    }
    assert_eq!(categories.len(), 10);
}

// ============================================================================
// 4. All FieldType variants serialize/deserialize
// ============================================================================

#[test]
fn field_type_serde() {
    let types = vec![
        FieldType::Text,
        FieldType::Email,
        FieldType::Password,
        FieldType::Number,
        FieldType::Date,
        FieldType::Tel,
        FieldType::Url,
        FieldType::Select,
        FieldType::Checkbox,
        FieldType::Radio,
        FieldType::Other,
    ];

    for ft in &types {
        let json = serde_json::to_string(ft).unwrap();
        let parsed: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(*ft, parsed, "Failed roundtrip for {:?}", ft);
    }
    assert_eq!(types.len(), 11);
}

// ============================================================================
// 5. MockPageAnalyzer: login page → category=Login, correct fields
// ============================================================================

#[test]
fn mock_analyzer_login_page() {
    let analyzer = MockPageAnalyzer;
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.category, PageCategory::Login);
    assert_eq!(model.forms.len(), 1);

    let form = &model.forms[0];
    assert_eq!(form.form_id, "login");
    assert_eq!(form.purpose, "Authentication");
    assert_eq!(form.fields.len(), 2);
    assert_eq!(form.submit_label, Some("Sign In".into()));

    // Email field
    assert_eq!(form.fields[0].label, "Email");
    assert_eq!(form.fields[0].field_type, FieldType::Email);
    assert_eq!(form.fields[0].suggested_test_value, "user@example.com");

    // Password field
    assert_eq!(form.fields[1].label, "Password");
    assert_eq!(form.fields[1].field_type, FieldType::Password);
    assert_eq!(form.fields[1].suggested_test_value, "TestPass123!");
}

// ============================================================================
// 6. MockPageAnalyzer: search page → category=Search, correct fields
// ============================================================================

#[test]
fn mock_analyzer_search_page() {
    let analyzer = MockPageAnalyzer;
    let screen = search_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.category, PageCategory::Search);
    assert_eq!(model.forms.len(), 1);

    let form = &model.forms[0];
    assert_eq!(form.form_id, "search");
    assert_eq!(form.fields.len(), 1);
    assert_eq!(form.fields[0].label, "Search query");
    assert_eq!(form.fields[0].suggested_test_value, "test query");
    assert_eq!(form.submit_label, Some("Search".into()));
}

// ============================================================================
// 7. MockPageAnalyzer: empty page → category=Other, no forms
// ============================================================================

#[test]
fn mock_analyzer_empty_page() {
    let analyzer = MockPageAnalyzer;
    let screen = empty_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.category, PageCategory::Other);
    assert!(model.forms.is_empty());
    assert!(model.outputs.is_empty());
    assert!(model.navigation_targets.is_empty());
    // Should still have a title-based assertion
    assert!(!model.suggested_assertions.is_empty());
}

// ============================================================================
// 8. LlmPageAnalyzer: valid JSON response → correct PageModel
// ============================================================================

#[test]
fn llm_analyzer_valid_response() {
    let valid_json = r#"{
        "purpose": "Login page",
        "category": "Login",
        "forms": [{
            "form_id": "login",
            "purpose": "User authentication",
            "fields": [{
                "label": "Email",
                "field_type": "Email",
                "required": true,
                "suggested_test_value": "ai@test.com"
            }],
            "submit_label": "Log In"
        }],
        "outputs": [],
        "suggested_assertions": [{
            "assertion_type": "title_contains",
            "expected": "Login",
            "description": "Page should have Login in title"
        }],
        "navigation_targets": []
    }"#;

    let analyzer = LlmPageAnalyzer::with_mock_response(valid_json);
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.purpose, "Login page");
    assert_eq!(model.category, PageCategory::Login);
    assert_eq!(model.forms.len(), 1);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "ai@test.com");
    assert_eq!(model.suggested_assertions.len(), 1);
}

// ============================================================================
// 9. LlmPageAnalyzer: empty response → falls back to MockPageAnalyzer
// ============================================================================

#[test]
fn llm_analyzer_empty_response() {
    let analyzer = LlmPageAnalyzer::with_mock_response("");
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    // Should fall back to MockPageAnalyzer result
    assert_eq!(model.category, PageCategory::Login);
    assert_eq!(model.forms.len(), 1);
    assert_eq!(model.forms[0].form_id, "login");
}

// ============================================================================
// 10. LlmPageAnalyzer: malformed JSON → falls back to MockPageAnalyzer
// ============================================================================

#[test]
fn llm_analyzer_malformed_json() {
    let analyzer = LlmPageAnalyzer::with_mock_response("{ not valid json at all }}}");
    let screen = search_screen();
    let model = analyzer.analyze(&screen).unwrap();

    // Should fall back to MockPageAnalyzer result
    assert_eq!(model.category, PageCategory::Search);
    assert_eq!(model.forms.len(), 1);
    assert_eq!(model.forms[0].form_id, "search");
}

// ============================================================================
// 11. classify_field_type: various input types and labels
// ============================================================================

#[test]
fn field_classification() {
    // By input_type
    assert_eq!(classify_field_type(Some("email"), None), FieldType::Email);
    assert_eq!(
        classify_field_type(Some("password"), None),
        FieldType::Password
    );
    assert_eq!(classify_field_type(Some("number"), None), FieldType::Number);
    assert_eq!(classify_field_type(Some("date"), None), FieldType::Date);
    assert_eq!(classify_field_type(Some("tel"), None), FieldType::Tel);
    assert_eq!(classify_field_type(Some("url"), None), FieldType::Url);
    assert_eq!(
        classify_field_type(Some("checkbox"), None),
        FieldType::Checkbox
    );
    assert_eq!(classify_field_type(Some("radio"), None), FieldType::Radio);
    assert_eq!(
        classify_field_type(Some("select"), None),
        FieldType::Select
    );

    // By label (when input_type is generic "text")
    assert_eq!(
        classify_field_type(Some("text"), Some("Email Address")),
        FieldType::Email
    );
    assert_eq!(
        classify_field_type(Some("text"), Some("Your Password")),
        FieldType::Password
    );
    assert_eq!(
        classify_field_type(Some("text"), Some("Phone Number")),
        FieldType::Tel
    );

    // Unknown → Text
    assert_eq!(classify_field_type(Some("text"), None), FieldType::Text);
    assert_eq!(
        classify_field_type(Some("text"), Some("Something")),
        FieldType::Text
    );
    assert_eq!(classify_field_type(None, None), FieldType::Text);
}

// ============================================================================
// 12. Navigation targets from standalone actions
// ============================================================================

#[test]
fn navigation_targets_from_actions() {
    let analyzer = MockPageAnalyzer;
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.navigation_targets.len(), 2);
    assert_eq!(model.navigation_targets[0].label, "Sign Up");
    assert!(model.navigation_targets[0]
        .likely_destination
        .contains("Sign Up"));
    assert_eq!(model.navigation_targets[1].label, "Forgot Password");
    assert!(model.navigation_targets[1]
        .likely_destination
        .contains("Forgot Password"));
}
