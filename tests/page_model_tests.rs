use std::collections::HashMap;

use screen_detection::agent::page_analyzer::{
    classify_field_type, map_llm_category, try_parse_llm_response, LlmPageAnalyzer,
    MockPageAnalyzer, PageAnalyzer,
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
                    required: false,
                    placeholder: None,
                    id: None,
                    href: None,
                    options: None,
                },
                ScreenElement {
                    label: Some("Password".into()),
                    kind: ElementKind::Input,
                    tag: Some("input".into()),
                    role: Some("textbox".into()),
                    input_type: Some("password".into()),
                    required: false,
                    placeholder: None,
                    id: None,
                    href: None,
                    options: None,
                },
            ],
            actions: vec![ScreenElement {
                label: Some("Sign In".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
            }],
            primary_action: Some(ScreenElement {
                label: Some("Sign In".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
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
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
            },
            ScreenElement {
                label: Some("Forgot Password".into()),
                kind: ElementKind::Action,
                tag: Some("a".into()),
                role: Some("link".into()),
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
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
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
            }],
            actions: vec![ScreenElement {
                label: Some("Search".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
            }],
            primary_action: Some(ScreenElement {
                label: Some("Search".into()),
                kind: ElementKind::Action,
                tag: Some("button".into()),
                role: Some("button".into()),
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
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
// 5. MockPageAnalyzer: login page â†’ category=Login, correct fields
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

    // Email field — Login category produces "testuser@example.com"
    assert_eq!(form.fields[0].label, "Email");
    assert_eq!(form.fields[0].field_type, FieldType::Email);
    assert_eq!(form.fields[0].suggested_test_value, "testuser@example.com");

    // Password field
    assert_eq!(form.fields[1].label, "Password");
    assert_eq!(form.fields[1].field_type, FieldType::Password);
    assert_eq!(form.fields[1].suggested_test_value, "TestPass123!");
}

// ============================================================================
// 6. MockPageAnalyzer: search page â†’ category=Search, correct fields
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
    assert_eq!(form.fields[0].suggested_test_value, "test search query"); // Search category overrides
    assert_eq!(form.submit_label, Some("Search".into()));
}

// ============================================================================
// 7. MockPageAnalyzer: empty page â†’ category=Other, no forms
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
// 8. LlmPageAnalyzer: valid JSON response â†’ hybrid enrichment
// ============================================================================

#[test]
fn llm_analyzer_valid_response() {
    // Under hybrid enrichment, LLM only provides purpose + category
    let valid_json = r#"{"purpose":"User authentication portal","category":"Login"}"#;

    let analyzer = LlmPageAnalyzer::with_mock_response(valid_json);
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    // LLM-provided values override defaults
    assert_eq!(model.purpose, "User authentication portal");
    assert_eq!(model.category, PageCategory::Login);

    // Form details come from deterministic MockPageAnalyzer
    assert_eq!(model.forms.len(), 1);
    assert_eq!(model.forms[0].form_id, "login");
    assert_eq!(model.forms[0].fields[0].label, "Email");
    assert_eq!(model.forms[0].fields[0].field_type, FieldType::Email);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "testuser@example.com"); // from guess_value() with Login category
    assert_eq!(model.forms[0].fields[1].label, "Password");
    assert_eq!(model.forms[0].fields[1].field_type, FieldType::Password);
    assert_eq!(model.forms[0].submit_label, Some("Sign In".into()));

    // Assertions and nav targets come from deterministic rules
    assert!(!model.suggested_assertions.is_empty());
    assert_eq!(model.navigation_targets.len(), 2);
}

// ============================================================================
// 9. LlmPageAnalyzer: empty response â†’ falls back to MockPageAnalyzer
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
// 10. LlmPageAnalyzer: malformed JSON â†’ falls back to MockPageAnalyzer
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
    // By input_type (tag=None, existing behavior preserved)
    assert_eq!(classify_field_type(Some("email"), None, None), FieldType::Email);
    assert_eq!(classify_field_type(Some("password"), None, None), FieldType::Password);
    assert_eq!(classify_field_type(Some("number"), None, None), FieldType::Number);
    assert_eq!(classify_field_type(Some("date"), None, None), FieldType::Date);
    assert_eq!(classify_field_type(Some("tel"), None, None), FieldType::Tel);
    assert_eq!(classify_field_type(Some("url"), None, None), FieldType::Url);
    assert_eq!(classify_field_type(Some("checkbox"), None, None), FieldType::Checkbox);
    assert_eq!(classify_field_type(Some("radio"), None, None), FieldType::Radio);
    assert_eq!(classify_field_type(Some("select"), None, None), FieldType::Select);

    // By label (when input_type is generic "text")
    assert_eq!(classify_field_type(Some("text"), Some("Email Address"), None), FieldType::Email);
    assert_eq!(classify_field_type(Some("text"), Some("Your Password"), None), FieldType::Password);
    assert_eq!(classify_field_type(Some("text"), Some("Phone Number"), None), FieldType::Tel);

    // Unknown -> Text
    assert_eq!(classify_field_type(Some("text"), None, None), FieldType::Text);
    assert_eq!(classify_field_type(Some("text"), Some("Something"), None), FieldType::Text);
    assert_eq!(classify_field_type(None, None, None), FieldType::Text);
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

// ============================================================================
// 13. JSON recovery: valid JSON
// ============================================================================

#[test]
fn try_parse_valid_json() {
    let result = try_parse_llm_response(r#"{"purpose":"Login page","category":"Login"}"#);
    let parsed = result.unwrap();
    assert_eq!(parsed.purpose.unwrap(), "Login page");
    assert_eq!(parsed.category.unwrap(), "Login");
}

// ============================================================================
// 14. JSON recovery: markdown code fences
// ============================================================================

#[test]
fn try_parse_markdown_fences() {
    let input = "```json\n{\"purpose\":\"Search\",\"category\":\"Search\"}\n```";
    let result = try_parse_llm_response(input);
    let parsed = result.unwrap();
    assert_eq!(parsed.purpose.unwrap(), "Search");
    assert_eq!(parsed.category.unwrap(), "Search");
}

// ============================================================================
// 15. JSON recovery: trailing comma
// ============================================================================

#[test]
fn try_parse_trailing_comma() {
    let input = r#"{"purpose":"Dashboard","category":"Dashboard",}"#;
    let result = try_parse_llm_response(input);
    let parsed = result.unwrap();
    assert_eq!(parsed.purpose.unwrap(), "Dashboard");
    assert_eq!(parsed.category.unwrap(), "Dashboard");
}

// ============================================================================
// 16. JSON recovery: preamble text before JSON
// ============================================================================

#[test]
fn try_parse_preamble_text() {
    let input = "Here is the analysis:\n{\"purpose\":\"Error page\",\"category\":\"Error\"}";
    let result = try_parse_llm_response(input);
    let parsed = result.unwrap();
    assert_eq!(parsed.purpose.unwrap(), "Error page");
    assert_eq!(parsed.category.unwrap(), "Error");
}

// ============================================================================
// 17. JSON recovery: partial fields (only purpose, no category)
// ============================================================================

#[test]
fn try_parse_partial_fields() {
    let input = r#"{"purpose":"Some page"}"#;
    let result = try_parse_llm_response(input);
    let parsed = result.unwrap();
    assert_eq!(parsed.purpose.unwrap(), "Some page");
    assert!(parsed.category.is_none());
}

// ============================================================================
// 18. JSON recovery: garbage input
// ============================================================================

#[test]
fn try_parse_garbage() {
    let result = try_parse_llm_response("this is not json at all");
    assert!(result.is_none());
}

// ============================================================================
// 19. Category mapping: standard values
// ============================================================================

#[test]
fn map_category_standard() {
    assert_eq!(map_llm_category("Login"), PageCategory::Login);
    assert_eq!(map_llm_category("Search"), PageCategory::Search);
    assert_eq!(map_llm_category("Dashboard"), PageCategory::Dashboard);
    assert_eq!(map_llm_category("Error"), PageCategory::Error);
    assert_eq!(map_llm_category("Other"), PageCategory::Other);
}

// ============================================================================
// 20. Category mapping: case insensitive
// ============================================================================

#[test]
fn map_category_case_insensitive() {
    assert_eq!(map_llm_category("login"), PageCategory::Login);
    assert_eq!(map_llm_category("LOGIN"), PageCategory::Login);
    assert_eq!(map_llm_category("Login"), PageCategory::Login);
    assert_eq!(map_llm_category("search"), PageCategory::Search);
    assert_eq!(map_llm_category("SEARCH"), PageCategory::Search);
}

// ============================================================================
// 21. Category mapping: aliases
// ============================================================================

#[test]
fn map_category_aliases() {
    assert_eq!(map_llm_category("registration"), PageCategory::Registration);
    assert_eq!(map_llm_category("signup"), PageCategory::Registration);
    assert_eq!(map_llm_category("sign up"), PageCategory::Registration);
    assert_eq!(map_llm_category("payment"), PageCategory::Checkout);
    assert_eq!(map_llm_category("checkout"), PageCategory::Checkout);
    assert_eq!(map_llm_category("settings"), PageCategory::Settings);
    assert_eq!(map_llm_category("preferences"), PageCategory::Settings);
    assert_eq!(map_llm_category("listing"), PageCategory::Listing);
    assert_eq!(map_llm_category("detail"), PageCategory::Detail);
}

// ============================================================================
// 22. Category mapping: unknown values â†’ Other
// ============================================================================

#[test]
fn map_category_unknown() {
    assert_eq!(map_llm_category("SomethingRandom"), PageCategory::Other);
    assert_eq!(map_llm_category("Form"), PageCategory::Other);
    assert_eq!(map_llm_category(""), PageCategory::Other);
    assert_eq!(map_llm_category("xyz"), PageCategory::Other);
}

// ============================================================================
// 23. Hybrid enrichment: LLM overrides category on non-matching page
// ============================================================================

#[test]
fn hybrid_llm_overrides_category() {
    // LLM says Login, but empty_screen() has no forms â†’ MockPageAnalyzer would say Other
    let response = r#"{"purpose":"Auth page","category":"Login"}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(response);
    let screen = empty_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.category, PageCategory::Login); // LLM override
    assert_eq!(model.purpose, "Auth page"); // LLM override
    // MockPageAnalyzer still populates the rest
    assert!(model.forms.is_empty()); // empty screen has no forms
    assert!(!model.suggested_assertions.is_empty()); // title-based assertion from Mock
}

// ============================================================================
// 24. Hybrid enrichment: LLM provides purpose only (no category)
// ============================================================================

#[test]
fn hybrid_llm_purpose_only() {
    let response = r#"{"purpose":"A custom search interface"}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(response);
    let screen = search_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.purpose, "A custom search interface"); // LLM override
    assert_eq!(model.category, PageCategory::Search); // from MockPageAnalyzer (form intent)
    assert_eq!(model.forms.len(), 1); // from MockPageAnalyzer
}

// ============================================================================
// 25. Hybrid enrichment: LLM returns garbage â†’ full fallback
// ============================================================================

#[test]
fn hybrid_llm_garbage_response() {
    let analyzer = LlmPageAnalyzer::with_mock_response("completely invalid response");
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    // Should be identical to MockPageAnalyzer output
    let mock_model = MockPageAnalyzer.analyze(&login_screen()).unwrap();
    assert_eq!(model.purpose, mock_model.purpose);
    assert_eq!(model.category, mock_model.category);
    assert_eq!(model.forms.len(), mock_model.forms.len());
}

// ============================================================================
// 26. Hybrid enrichment: LLM unavailable (empty string)
// ============================================================================

#[test]
fn hybrid_llm_unavailable() {
    let analyzer = LlmPageAnalyzer::with_mock_response("");
    let screen = search_screen();
    let model = analyzer.analyze(&screen).unwrap();

    // Should be identical to MockPageAnalyzer output
    let mock_model = MockPageAnalyzer.analyze(&search_screen()).unwrap();
    assert_eq!(model.purpose, mock_model.purpose);
    assert_eq!(model.category, mock_model.category);
    assert_eq!(model.forms.len(), mock_model.forms.len());
}

// ============================================================================
// 27. Hybrid enrichment: preserves field types from deterministic rules
// ============================================================================

#[test]
fn hybrid_preserves_field_types() {
    let response = r#"{"purpose":"User authentication","category":"Login"}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(response);
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    // LLM overrides
    assert_eq!(model.purpose, "User authentication");
    assert_eq!(model.category, PageCategory::Login);

    // Deterministic field types preserved — Login category values
    assert_eq!(model.forms[0].fields[0].field_type, FieldType::Email);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "testuser@example.com");
    assert_eq!(model.forms[0].fields[1].field_type, FieldType::Password);
    assert_eq!(model.forms[0].fields[1].suggested_test_value, "TestPass123!");
    assert_eq!(model.forms[0].submit_label, Some("Sign In".into()));

    // Navigation targets preserved
    assert_eq!(model.navigation_targets.len(), 2);
}

// ============================================================================
// 28. Prompt construction: contains URL and title
// ============================================================================

#[test]
fn prompt_contains_url_and_title() {
    let screen = login_screen();
    let prompt = LlmPageAnalyzer::build_page_prompt(&screen);

    assert!(prompt.contains("example.com/login"), "Prompt should contain URL");
    assert!(prompt.contains("Login"), "Prompt should contain title");
    assert!(prompt.contains("Email"), "Prompt should contain form field labels");
    assert!(prompt.contains("Password"), "Prompt should contain form field labels");
    assert!(prompt.contains("Sign In"), "Prompt should contain button labels");
}

// ============================================================================
// 29. Prompt construction: empty page
// ============================================================================

#[test]
fn prompt_empty_page() {
    let screen = empty_screen();
    let prompt = LlmPageAnalyzer::build_page_prompt(&screen);

    assert!(prompt.contains("(none)"), "Empty forms should show (none)");
    assert!(prompt.contains("Welcome"), "Should contain the page title");
    // Should NOT contain old-style full schema
    assert!(
        !prompt.contains("field_type"),
        "Simplified prompt should not ask for field_type"
    );
    assert!(
        !prompt.contains("suggested_test_value"),
        "Simplified prompt should not ask for test values"
    );
}

// ============================================================================
// 30. Prompt construction: reasonable length for small models
// ============================================================================

#[test]
fn prompt_length_reasonable() {
    let screen = login_screen();
    let prompt = LlmPageAnalyzer::build_page_prompt(&screen);

    assert!(
        prompt.len() < 800,
        "Prompt should be under 800 chars for small models, got {}",
        prompt.len()
    );
    // Should be significantly shorter than the old ~1500 char prompt
    assert!(
        prompt.len() > 100,
        "Prompt should be at least 100 chars, got {}",
        prompt.len()
    );
}

// ============================================================================
// New FieldType classification tests — expanded types
// ============================================================================

#[test]
fn field_classification_new_input_types() {
    assert_eq!(classify_field_type(Some("search"), None, None), FieldType::Search);
    assert_eq!(classify_field_type(Some("time"), None, None), FieldType::Time);
    assert_eq!(classify_field_type(Some("hidden"), None, None), FieldType::Hidden);
    assert_eq!(classify_field_type(Some("color"), None, None), FieldType::Other);
    assert_eq!(classify_field_type(Some("range"), None, None), FieldType::Other);
    assert_eq!(classify_field_type(Some("file"), None, None), FieldType::Other);
    assert_eq!(classify_field_type(Some("month"), None, None), FieldType::Other);
    assert_eq!(classify_field_type(Some("week"), None, None), FieldType::Other);
}

#[test]
fn field_classification_textarea_tag() {
    assert_eq!(classify_field_type(None, None, Some("textarea")), FieldType::Textarea);
    assert_eq!(classify_field_type(Some("text"), None, Some("textarea")), FieldType::Textarea);
}

#[test]
fn field_classification_select_tag() {
    assert_eq!(classify_field_type(None, None, Some("select")), FieldType::Select);
}

#[test]
fn field_classification_label_search() {
    assert_eq!(classify_field_type(Some("text"), Some("Search"), None), FieldType::Search);
    assert_eq!(classify_field_type(Some("text"), Some("Search query"), None), FieldType::Search);
}

#[test]
fn field_classification_label_textarea() {
    assert_eq!(classify_field_type(Some("text"), Some("Comment"), None), FieldType::Textarea);
    assert_eq!(classify_field_type(Some("text"), Some("Message"), None), FieldType::Textarea);
    assert_eq!(classify_field_type(Some("text"), Some("Description"), None), FieldType::Textarea);
}

#[test]
fn classify_preserves_existing_behavior() {
    // Existing mappings still work with tag=None
    assert_eq!(classify_field_type(Some("email"), None, None), FieldType::Email);
    assert_eq!(classify_field_type(Some("password"), None, None), FieldType::Password);
    assert_eq!(classify_field_type(Some("number"), None, None), FieldType::Number);
    assert_eq!(classify_field_type(Some("text"), Some("Email"), None), FieldType::Email);
    assert_eq!(classify_field_type(Some("text"), Some("Phone"), None), FieldType::Tel);
    assert_eq!(classify_field_type(None, None, None), FieldType::Text);
}

// ============================================================================
// MockPageAnalyzer tests — new field wiring
// ============================================================================

#[test]
fn mock_analyzer_extracts_required() {
    let analyzer = MockPageAnalyzer;
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![Form {
            id: "test".into(),
            intent: None,
            inputs: vec![ScreenElement {
                label: Some("Email".into()),
                kind: ElementKind::Input,
                tag: Some("input".into()),
                role: Some("textbox".into()),
                input_type: Some("email".into()),
                required: true,
                placeholder: None,
                id: None,
                href: None,
                options: None,
            }],
            actions: vec![],
            primary_action: None,
        }],
        outputs: vec![],
        standalone_actions: vec![],
        identities: std::collections::HashMap::new(),
    };

    let model = analyzer.analyze(&screen).unwrap();
    assert!(model.forms[0].fields[0].required);
}

#[test]
fn mock_analyzer_select_uses_first_option() {
    use screen_detection::screen::screen_model::SelectOption;

    let analyzer = MockPageAnalyzer;
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![Form {
            id: "test".into(),
            intent: None,
            inputs: vec![ScreenElement {
                label: Some("Country".into()),
                kind: ElementKind::Input,
                tag: Some("select".into()),
                role: None,
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: Some(vec![
                    SelectOption { value: "us".into(), text: "United States".into() },
                    SelectOption { value: "ca".into(), text: "Canada".into() },
                ]),
            }],
            actions: vec![],
            primary_action: None,
        }],
        outputs: vec![],
        standalone_actions: vec![],
        identities: std::collections::HashMap::new(),
    };

    let model = analyzer.analyze(&screen).unwrap();
    assert_eq!(model.forms[0].fields[0].field_type, FieldType::Select);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "us");
}

#[test]
fn mock_analyzer_login_page_uses_login_values() {
    let analyzer = MockPageAnalyzer;
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.category, PageCategory::Login);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "testuser@example.com");
}

#[test]
fn llm_analyzer_field_value_override() {
    let response = r##"{"purpose":"Login","category":"Login","field_values":{"Email":"custom@ai.com","Password":"AiPass!"}}"##;
    let analyzer = LlmPageAnalyzer::with_mock_response(response);
    let screen = login_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert_eq!(model.forms[0].fields[0].suggested_test_value, "custom@ai.com");
    assert_eq!(model.forms[0].fields[1].suggested_test_value, "AiPass!");
}

// ============================================================================
// FieldType serde — verify new variants
// ============================================================================

#[test]
fn field_type_serde_new_variants() {
    let variants = vec![
        FieldType::Textarea,
        FieldType::Search,
        FieldType::Hidden,
        FieldType::Time,
    ];
    for variant in &variants {
        let json = serde_json::to_string(variant).unwrap();
        let back: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(*variant, back);
    }
}
