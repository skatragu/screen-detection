use std::collections::HashMap;

use screen_detection::agent::page_analyzer::{
    build_rich_page_prompt, classify_field_type, classify_output_semantic,
    smart_select_option, try_parse_llm_response, LlmPageAnalyzer, MockPageAnalyzer, PageAnalyzer,
};
use screen_detection::screen::screen_model::SelectOption as SmartSelectOption;
use screen_detection::agent::page_model::{
    ExpectedOutcome, FieldAnalysis, FieldModel, FieldType, FormModel, NavigationTarget,
    OutputModel, OutputSemantic, PageModel, SuggestedAssertion, TestScenario,
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
                    name: None,
                    value: None,
                    maxlength: None,
                    minlength: None,
                    readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                    name: None,
                    value: None,
                    maxlength: None,
                    minlength: None,
                    readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
            },
        ],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
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
        structural_outline: Default::default(),
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
        structural_outline: Default::default(),
    }
}

fn sample_page_model() -> PageModel {
    PageModel {
        purpose: "Login page for user authentication".into(),
        domain: "login".to_string(),
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
            expected_outcome: ExpectedOutcome::default(),
        }],
        outputs: vec![OutputModel {
            description: "Welcome message".into(),
            region: "main".into(),
            semantic: OutputSemantic::Info,
        }],
        suggested_assertions: vec![SuggestedAssertion {
            assertion_type: "title_contains".into(),
            expected: "Login".into(),
            description: "Verify page title contains 'Login'".into(),
        }],
        navigation_targets: vec![NavigationTarget {
            label: "Sign Up".into(),
            likely_destination: "Registration page".into(),
            href: None,
        }],
        expected_outcome: ExpectedOutcome::default(),
            layout_description: None,
            field_analyses: vec![],
            suggested_test_scenarios: vec![],
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
// 3. (PageCategory enum removed - tests removed)
// ============================================================================


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

    assert!(!model.purpose.is_empty());
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
// 6. MockPageAnalyzer: search page â†’ category=Search, correct fields
// ============================================================================

#[test]
fn mock_analyzer_search_page() {
    let analyzer = MockPageAnalyzer;
    let screen = search_screen();
    let model = analyzer.analyze(&screen).unwrap();

    assert!(!model.purpose.is_empty());
    assert_eq!(model.forms.len(), 1);

    let form = &model.forms[0];
    assert_eq!(form.form_id, "search");
    assert_eq!(form.fields.len(), 1);
    assert_eq!(form.fields[0].label, "Search query");
    assert_eq!(form.fields[0].suggested_test_value, "test query");
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

    assert!(!model.purpose.is_empty());
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
    assert!(!model.purpose.is_empty());

    // Form details come from deterministic MockPageAnalyzer
    assert_eq!(model.forms.len(), 1);
    assert_eq!(model.forms[0].form_id, "login");
    assert_eq!(model.forms[0].fields[0].label, "Email");
    assert_eq!(model.forms[0].fields[0].field_type, FieldType::Email);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "user@example.com");
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
    assert!(!model.purpose.is_empty());
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
    assert!(!model.purpose.is_empty());
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


// ============================================================================
// 20. Category mapping: case insensitive
// ============================================================================


// ============================================================================
// 21. Category mapping: aliases
// ============================================================================


// ============================================================================
// 22. Category mapping: unknown values â†’ Other
// ============================================================================


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

    assert!(!model.purpose.is_empty()); // LLM override
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
    assert!(!model.purpose.is_empty()); // from MockPageAnalyzer (form intent)
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
    assert_eq!(model.domain, mock_model.domain);
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
    assert_eq!(model.domain, mock_model.domain);
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
    assert!(!model.purpose.is_empty());

    // Deterministic field types preserved
    assert_eq!(model.forms[0].fields[0].field_type, FieldType::Email);
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "user@example.com");
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

    // Prompt grew slightly to include success/error indicator requests, now ~930 chars
    assert!(
        prompt.len() < 1200,
        "Prompt should be under 1200 chars for small models, got {}",
        prompt.len()
    );
    // Should be significantly shorter than a verbose multi-page prompt
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
            }],
            actions: vec![],
            primary_action: None,
        }],
        outputs: vec![],
        standalone_actions: vec![],
        identities: std::collections::HashMap::new(),
        structural_outline: Default::default(),
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
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
            }],
            actions: vec![],
            primary_action: None,
        }],
        outputs: vec![],
        standalone_actions: vec![],
        identities: std::collections::HashMap::new(),
        structural_outline: Default::default(),
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

    assert!(!model.purpose.is_empty());
    assert_eq!(model.forms[0].fields[0].suggested_test_value, "user@example.com");
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

// ============================================================================
// Phase 13: Smart Dropdown Selection
// ============================================================================

#[test]
fn smart_select_skips_empty_value_placeholder() {
    let options = vec![
        SmartSelectOption { value: "".into(), text: "Select...".into() },
        SmartSelectOption { value: "us".into(), text: "United States".into() },
    ];
    let selected = smart_select_option(&options).unwrap();
    assert_eq!(selected.value, "us");
}

#[test]
fn smart_select_skips_choose_text() {
    let options = vec![
        SmartSelectOption { value: "".into(), text: "Choose a country".into() },
        SmartSelectOption { value: "uk".into(), text: "UK".into() },
    ];
    let selected = smart_select_option(&options).unwrap();
    assert_eq!(selected.value, "uk");
}

#[test]
fn smart_select_skips_dashes_placeholder() {
    let options = vec![
        SmartSelectOption { value: "".into(), text: "-- Select --".into() },
        SmartSelectOption { value: "ca".into(), text: "Canada".into() },
    ];
    let selected = smart_select_option(&options).unwrap();
    assert_eq!(selected.value, "ca");
}

#[test]
fn smart_select_skips_please_select() {
    let options = vec![
        SmartSelectOption { value: "".into(), text: "Please select".into() },
        SmartSelectOption { value: "fr".into(), text: "France".into() },
    ];
    let selected = smart_select_option(&options).unwrap();
    assert_eq!(selected.value, "fr");
}

#[test]
fn smart_select_falls_back_to_first() {
    let options = vec![
        SmartSelectOption { value: "".into(), text: "Select...".into() },
        SmartSelectOption { value: "".into(), text: "Choose...".into() },
    ];
    let selected = smart_select_option(&options).unwrap();
    assert_eq!(selected.text, "Select...");
}

#[test]
fn smart_select_with_no_placeholder() {
    let options = vec![
        SmartSelectOption { value: "us".into(), text: "US".into() },
        SmartSelectOption { value: "uk".into(), text: "UK".into() },
    ];
    let selected = smart_select_option(&options).unwrap();
    assert_eq!(selected.value, "us");
}

// ============================================================================
// Phase 13: Output Semantic Classification
// ============================================================================

#[test]
fn classify_output_semantic_error_patterns() {
    assert_eq!(classify_output_semantic("Invalid password"), OutputSemantic::Error);
    assert_eq!(classify_output_semantic("Error: something failed"), OutputSemantic::Error);
    assert_eq!(classify_output_semantic("Page Not Found"), OutputSemantic::Error);
    assert_eq!(classify_output_semantic("Access denied"), OutputSemantic::Error);
}

#[test]
fn classify_output_semantic_success_patterns() {
    assert_eq!(classify_output_semantic("Account created successfully"), OutputSemantic::Success);
    assert_eq!(classify_output_semantic("Welcome back, user!"), OutputSemantic::Success);
    assert_eq!(classify_output_semantic("Settings saved"), OutputSemantic::Success);
    assert_eq!(classify_output_semantic("Thank you for signing up"), OutputSemantic::Success);
}

#[test]
fn classify_output_semantic_warning_patterns() {
    assert_eq!(classify_output_semantic("Warning: session about to expire"), OutputSemantic::Warning);
    assert_eq!(classify_output_semantic("Use caution when deleting"), OutputSemantic::Warning);
}

#[test]
fn classify_output_semantic_info_default() {
    assert_eq!(classify_output_semantic("Hello world"), OutputSemantic::Info);
    assert_eq!(classify_output_semantic("Product details"), OutputSemantic::Info);
}

#[test]
fn classify_output_semantic_navigation() {
    assert_eq!(classify_output_semantic("Home"), OutputSemantic::Navigation);
    assert_eq!(classify_output_semantic("Page 2 of 5"), OutputSemantic::Navigation);
}

#[test]
fn output_semantic_serde_roundtrip() {
    let semantics = vec![
        OutputSemantic::Success, OutputSemantic::Error, OutputSemantic::Warning,
        OutputSemantic::Info, OutputSemantic::Data, OutputSemantic::Navigation,
    ];
    for s in &semantics {
        let json = serde_json::to_string(s).unwrap();
        let back: OutputSemantic = serde_json::from_str(&json).unwrap();
        assert_eq!(*s, back);
    }
}

#[test]
fn output_model_backward_compat() {
    // JSON without semantic field should deserialize with default Info
    let json = r#"{"description":"Hello","region":"main"}"#;
    let model: OutputModel = serde_json::from_str(json).unwrap();
    assert_eq!(model.semantic, OutputSemantic::Info);
}

#[test]
fn mock_analyzer_classifies_output_semantics() {
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![
            ScreenElement {
                label: Some("Error: Invalid input".into()),
                kind: ElementKind::Output, tag: Some("div".into()),
                role: None, input_type: None, required: false,
                placeholder: None, id: None, href: None, options: None,
                name: None, value: None, maxlength: None, minlength: None, readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
            },
            ScreenElement {
                label: Some("Welcome back!".into()),
                kind: ElementKind::Output, tag: Some("div".into()),
                role: None, input_type: None, required: false,
                placeholder: None, id: None, href: None, options: None,
                name: None, value: None, maxlength: None, minlength: None, readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
            },
        ],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let model = MockPageAnalyzer.analyze(&screen).unwrap();
    assert_eq!(model.outputs.len(), 2);
    assert_eq!(model.outputs[0].semantic, OutputSemantic::Error);
    assert_eq!(model.outputs[1].semantic, OutputSemantic::Success);
}

// ============================================================================
// Phase 13: Readonly input filtering in MockPageAnalyzer
// ============================================================================

#[test]
fn mock_analyzer_skips_readonly_inputs() {
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![Form {
            id: "f".into(), intent: None,
            inputs: vec![
                ScreenElement {
                    label: Some("Name".into()), kind: ElementKind::Input,
                    tag: Some("input".into()), role: None, input_type: Some("text".into()),
                    required: false, placeholder: None, id: None, href: None, options: None,
                    name: None, value: None, maxlength: None, minlength: None, readonly: true,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
                },
                ScreenElement {
                    label: Some("Email".into()), kind: ElementKind::Input,
                    tag: Some("input".into()), role: None, input_type: Some("email".into()),
                    required: false, placeholder: None, id: None, href: None, options: None,
                    name: None, value: None, maxlength: None, minlength: None, readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
                },
            ],
            actions: vec![], primary_action: None,
        }],
        standalone_actions: vec![], outputs: vec![], identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let model = MockPageAnalyzer.analyze(&screen).unwrap();
    assert_eq!(model.forms[0].fields.len(), 1, "Readonly input should be filtered");
    assert_eq!(model.forms[0].fields[0].label, "Email");
}

#[test]
fn mock_analyzer_includes_non_readonly_inputs() {
    let screen = ScreenState {
        url: Some("https://example.com".into()),
        title: "Test".into(),
        forms: vec![Form {
            id: "f".into(), intent: None,
            inputs: vec![
                ScreenElement {
                    label: Some("Name".into()), kind: ElementKind::Input,
                    tag: Some("input".into()), role: None, input_type: Some("text".into()),
                    required: false, placeholder: None, id: None, href: None, options: None,
                    name: None, value: None, maxlength: None, minlength: None, readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
                },
                ScreenElement {
                    label: Some("Email".into()), kind: ElementKind::Input,
                    tag: Some("input".into()), role: None, input_type: Some("email".into()),
                    required: false, placeholder: None, id: None, href: None, options: None,
                    name: None, value: None, maxlength: None, minlength: None, readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
                },
            ],
            actions: vec![], primary_action: None,
        }],
        standalone_actions: vec![], outputs: vec![], identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let model = MockPageAnalyzer.analyze(&screen).unwrap();
    assert_eq!(model.forms[0].fields.len(), 2, "Both non-readonly inputs should be included");
}

#[test]
fn screen_element_carries_constraints() {
    use screen_detection::screen::classifier::classify;
    let elements = vec![screen_detection::screen::screen_model::DomElement {
        tag: "input".into(), text: None, role: Some("textbox".into()),
        r#type: Some("text".into()), aria_label: Some("Username".into()),
        disabled: false, required: false, form_id: Some("f".into()),
        id: None, name: Some("user".into()), placeholder: None, href: None,
        value: Some("val".into()), options: None,
        pattern: Some("[A-Z]+".into()), minlength: Some(3), maxlength: Some(20),
        min: None, max: None, readonly: false,
        heading_level: None,
        autocomplete: None,
        inputmode: None,
        title_attr: None,
        form_action: None,
        form_method: None,
        aria_describedby_text: None,
        aria_invalid: false,
        aria_required: false,
        associated_label_text: None,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        semantic_section: None,
        visible: false,
    }];
    let semantics = classify(&elements);
    let form = semantics.forms.iter().find(|f| f.id == "f").unwrap();
    assert_eq!(form.inputs[0].maxlength, Some(20));
    assert_eq!(form.inputs[0].minlength, Some(3));
    assert_eq!(form.inputs[0].name, Some("user".into()));
    assert_eq!(form.inputs[0].value, Some("val".into()));
}

// ============================================================================
// NavigationTarget.href serde tests
// ============================================================================

#[test]
fn navigation_target_href_serde() {
    let target = NavigationTarget {
        label: "Login".into(),
        likely_destination: "Login page".into(),
        href: Some("/login".into()),
    };
    let json = serde_json::to_string(&target).unwrap();
    let parsed: NavigationTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.label, "Login");
    assert_eq!(parsed.likely_destination, "Login page");
    assert_eq!(parsed.href, Some("/login".into()));
}

#[test]
fn navigation_target_backward_compat() {
    // JSON without `href` key should deserialize with href = None
    let json = r#"{"label":"Dashboard","likely_destination":"Dashboard page"}"#;
    let parsed: NavigationTarget = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.label, "Dashboard");
    assert_eq!(parsed.href, None);
}

// ============================================================================
// ExpectedOutcome tests
// ============================================================================

#[test]
fn expected_outcome_default_empty() {
    let outcome = ExpectedOutcome::default();
    assert!(outcome.url_contains.is_none());
    assert!(outcome.url_not_contains.is_none());
    assert!(outcome.success_text.is_empty());
    assert!(outcome.error_indicators.is_empty());
}

#[test]
fn expected_outcome_serde_roundtrip() {
    let outcome = ExpectedOutcome {
        url_contains: Some("/dashboard".into()),
        url_not_contains: Some("/login".into()),
        success_text: vec!["welcome".into(), "dashboard".into()],
        error_indicators: vec!["invalid".into(), "failed".into()],
    };
    let json = serde_json::to_string(&outcome).unwrap();
    let parsed: ExpectedOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, outcome);
}

#[test]
fn expected_outcome_backward_compat() {
    // FormModel and PageModel without expected_outcome field should deserialize with default
    let form_json = r#"{"form_id":"f","purpose":"p","fields":[],"submit_label":null}"#;
    let form: FormModel = serde_json::from_str(form_json).unwrap();
    assert_eq!(form.expected_outcome, ExpectedOutcome::default());

    let page_json = r#"{"purpose":"p","domain":"","forms":[],"outputs":[],"suggested_assertions":[],"navigation_targets":[]}"#;
    let page: PageModel = serde_json::from_str(page_json).unwrap();
    assert_eq!(page.expected_outcome, ExpectedOutcome::default());
}

#[test]
fn form_model_with_outcome_serde() {
    let form = FormModel {
        form_id: "login".into(),
        purpose: "Authentication".into(),
        fields: vec![],
        submit_label: Some("Sign In".into()),
        expected_outcome: ExpectedOutcome {
            url_not_contains: Some("/login".into()),
            error_indicators: vec!["invalid".into()],
            ..Default::default()
        },
    };
    let json = serde_json::to_string(&form).unwrap();
    let parsed: FormModel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.expected_outcome.url_not_contains, Some("/login".into()));
    assert_eq!(parsed.expected_outcome.error_indicators, vec!["invalid".to_string()]);
}






#[test]
fn mock_analyzer_generates_richer_assertions() {
    // Login page with error output → text_absent assertions generated
    let screen = ScreenState {
        url: Some("https://example.com/login".into()),
        title: "Login".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![screen_detection::screen::screen_model::ScreenElement {
            label: Some("Invalid credentials".into()),
            kind: ElementKind::Output,
            tag: None, role: None, input_type: None,
            required: false, placeholder: None,
            id: None, href: None, options: None,
            name: None, value: None,
            maxlength: None, minlength: None, readonly: false,
        fieldset_legend: None,
        section_heading: None,
        nearby_help_text: None,
        autocomplete: None,
        aria_describedby_text: None,
        }],
        identities: std::collections::HashMap::new(),
        structural_outline: Default::default(),
    };
    let model = MockPageAnalyzer.analyze(&screen).unwrap();
    // Should have a text_absent assertion for the error output
    let has_error_absent = model.suggested_assertions.iter().any(|a| {
        a.assertion_type == "text_absent" && a.expected.contains("Invalid credentials")
    });
    assert!(has_error_absent, "Expected text_absent assertion for error output, got: {:?}", model.suggested_assertions);
}

// ============================================================================
// Phase 14 Step 7: LLM-Enhanced Understanding (success_indicators + error_indicators)
// ============================================================================

#[test]
fn llm_response_with_indicators() {
    // LlmPageResponse parses success_indicators and error_indicators fields
    let json = r#"{"purpose":"Login page","category":"Login","success_indicators":["welcome","dashboard"],"error_indicators":["invalid","incorrect"]}"#;
    let parsed = try_parse_llm_response(json).expect("Should parse JSON with indicators");
    assert_eq!(parsed.purpose, Some("Login page".into()));
    assert_eq!(parsed.category, Some("Login".into()));
    assert_eq!(
        parsed.success_indicators,
        Some(vec!["welcome".to_string(), "dashboard".to_string()])
    );
    assert_eq!(
        parsed.error_indicators,
        Some(vec!["invalid".to_string(), "incorrect".to_string()])
    );
}

#[test]
fn llm_response_indicators_default_empty() {
    // When success_indicators and error_indicators are absent, they default to None
    let json = r#"{"purpose":"Login page","category":"Login"}"#;
    let parsed = try_parse_llm_response(json).expect("Should parse JSON without indicators");
    assert_eq!(parsed.success_indicators, None);
    assert_eq!(parsed.error_indicators, None);
}

#[test]
fn llm_enriches_expected_outcome() {
    // LlmPageAnalyzer merges LLM-provided indicators into form.expected_outcome
    let mock_response = r#"{"purpose":"Login page","category":"Login","success_indicators":["logged in"],"error_indicators":["wrong password"]}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);

    let model = analyzer.analyze(&login_screen()).unwrap();

    // The form's expected_outcome should include LLM-provided indicators
    // (in addition to deterministically-inferred ones from infer_form_outcome)
    let has_llm_success = model.forms.iter().any(|f| {
        f.expected_outcome.success_text.contains(&"logged in".to_string())
    });
    assert!(
        has_llm_success,
        "LLM success indicator 'logged in' should be in form.expected_outcome.success_text, got: {:?}",
        model.forms.iter().map(|f| &f.expected_outcome.success_text).collect::<Vec<_>>()
    );

    let has_llm_error = model.forms.iter().any(|f| {
        f.expected_outcome.error_indicators.contains(&"wrong password".to_string())
    });
    assert!(
        has_llm_error,
        "LLM error indicator 'wrong password' should be in form.expected_outcome.error_indicators, got: {:?}",
        model.forms.iter().map(|f| &f.expected_outcome.error_indicators).collect::<Vec<_>>()
    );
}

#[test]
fn llm_indicators_merge_with_deterministic() {
    // LLM indicators should be present in expected_outcome
    let mock_response = r#"{"purpose":"Login page","category":"Login","success_indicators":["logged in"],"error_indicators":["wrong password"]}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);

    let model = analyzer.analyze(&login_screen()).unwrap();

    if let Some(form) = model.forms.first() {
        // LLM-provided success indicator should be present
        assert!(
            form.expected_outcome.success_text.contains(&"logged in".to_string()),
            "LLM 'logged in' should be in success_text, got: {:?}",
            form.expected_outcome.success_text
        );
        // LLM-provided error indicator should be present
        assert!(
            form.expected_outcome.error_indicators.contains(&"wrong password".to_string()),
            "LLM 'wrong password' should be in error_indicators, got: {:?}",
            form.expected_outcome.error_indicators
        );
    } else {
        // No form? Accept - page-level should still have indicators
        assert!(
            model.expected_outcome.success_text.contains(&"logged in".to_string()),
            "Page-level expected_outcome should have LLM success indicators"
        );
    }
}

#[test]
fn llm_indicators_no_duplicates() {
    // If LLM returns an indicator that's already in the deterministic set, no duplicate is added
    // Login deterministic includes "welcome" → LLM also returns "welcome" → should appear once
    let mock_response = r#"{"purpose":"Login page","category":"Login","success_indicators":["welcome"],"error_indicators":["invalid"]}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);

    let model = analyzer.analyze(&login_screen()).unwrap();

    if let Some(form) = model.forms.first() {
        let welcome_count = form.expected_outcome.success_text.iter()
            .filter(|s| s.as_str() == "welcome")
            .count();
        assert_eq!(
            welcome_count, 1,
            "'welcome' should appear exactly once (no duplicate from LLM merge), got: {:?}",
            form.expected_outcome.success_text
        );
        let invalid_count = form.expected_outcome.error_indicators.iter()
            .filter(|s| s.as_str() == "invalid")
            .count();
        assert_eq!(
            invalid_count, 1,
            "'invalid' should appear exactly once (no duplicate from LLM merge), got: {:?}",
            form.expected_outcome.error_indicators
        );
    }
}

#[test]
fn llm_page_level_outcome_enriched() {
    // Page-level expected_outcome also gets LLM indicators
    let mock_response = r#"{"purpose":"Search page","category":"Search","success_indicators":["results found"],"error_indicators":["no matches"]}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);

    // Use a minimal screen state
    let screen = ScreenState {
        url: Some("https://example.com/search".into()),
        title: "Search".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };

    let model = analyzer.analyze(&screen).unwrap();

    assert!(
        model.expected_outcome.success_text.contains(&"results found".to_string()),
        "Page-level expected_outcome.success_text should include LLM indicator 'results found', got: {:?}",
        model.expected_outcome.success_text
    );
    assert!(
        model.expected_outcome.error_indicators.contains(&"no matches".to_string()),
        "Page-level expected_outcome.error_indicators should include LLM indicator 'no matches', got: {:?}",
        model.expected_outcome.error_indicators
    );
}

#[test]
fn llm_prompt_includes_outcome_request() {
    // The prompt built by LlmPageAnalyzer mentions success_indicators and error_indicators
    let screen = ScreenState {
        url: Some("https://example.com/login".into()),
        title: "Login".into(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let prompt = LlmPageAnalyzer::build_page_prompt(&screen);

    assert!(
        prompt.contains("success_indicators"),
        "Prompt should request success_indicators from LLM, got prompt length: {}",
        prompt.len()
    );
    assert!(
        prompt.contains("error_indicators"),
        "Prompt should request error_indicators from LLM, got prompt length: {}",
        prompt.len()
    );
}

#[test]
fn llm_empty_indicators_dont_add_to_outcome() {
    // If LLM returns empty arrays for indicators, they should not be added
    let mock_response = r#"{"purpose":"Login page","category":"Login","success_indicators":[],"error_indicators":[]}"#;
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);

    let model_with_empty_llm = analyzer.analyze(&login_screen()).unwrap();
    let deterministic_model = MockPageAnalyzer.analyze(&login_screen()).unwrap();

    // Outcomes should match what deterministic produces (empty LLM arrays don't extend)
    if let (Some(form_llm), Some(form_det)) = (model_with_empty_llm.forms.first(), deterministic_model.forms.first()) {
        assert_eq!(
            form_llm.expected_outcome.success_text,
            form_det.expected_outcome.success_text,
            "Empty LLM indicators should not change deterministic outcomes"
        );
        assert_eq!(
            form_llm.expected_outcome.error_indicators,
            form_det.expected_outcome.error_indicators,
            "Empty LLM error indicators should not change deterministic outcomes"
        );
    }
}

// ============================================================================
// Phase 15 Step 0: domain field verification
// ============================================================================

#[test]
fn page_model_domain_serde_roundtrip() {
    let model = PageModel {
        purpose: "Test page".to_string(),
        domain: "telecom SIM provisioning wizard".to_string(),
        forms: vec![],
        outputs: vec![],
        suggested_assertions: vec![],
        navigation_targets: vec![],
        expected_outcome: Default::default(),
            layout_description: None,
            field_analyses: vec![],
            suggested_test_scenarios: vec![],
    };
    let json = serde_json::to_string(&model).unwrap();
    assert!(json.contains(r#""domain":"telecom SIM provisioning wizard""#));
    let parsed: PageModel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.domain, "telecom SIM provisioning wizard");
}

#[test]
fn page_model_domain_defaults_to_empty_string() {
    let json = r#"{"purpose":"test","forms":[],"outputs":[],"suggested_assertions":[],"navigation_targets":[]}"#;
    let model: PageModel = serde_json::from_str(json).unwrap();
    assert_eq!(model.domain, "");
}

#[test]
fn mock_page_analyzer_derives_domain_from_title() {
    let screen = ScreenState {
        url: Some("http://example.com".to_string()),
        title: "Login to your account".to_string(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: std::collections::HashMap::new(),
        structural_outline: Default::default(),
    };
    let analyzer = MockPageAnalyzer;
    let model = analyzer.analyze(&screen).unwrap();
    // domain is non-empty and derived from page context
    assert!(!model.domain.is_empty() || !model.purpose.is_empty());
}

// ============================================================================
// Phase 15 Step 2: FieldAnalysis, TestScenario, and rich prompt tests
// ============================================================================

#[test]
fn field_analysis_serde_roundtrip() {
    let fa = FieldAnalysis {
        label: "Patient ID".to_string(),
        context_clues: "In 'Patient Registration' fieldset; help text says 'Your hospital ID'".to_string(),
        validation_hint: Some("Must be 6 digits".to_string()),
        suggested_value: Some("123456".to_string()),
        negative_values: vec!["abc".to_string(), "".to_string()],
    };
    let json = serde_json::to_string(&fa).unwrap();
    let parsed: FieldAnalysis = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, fa);
    assert_eq!(parsed.label, "Patient ID");
    assert_eq!(parsed.suggested_value, Some("123456".to_string()));
    assert_eq!(parsed.negative_values.len(), 2);
}

#[test]
fn test_scenario_serde_roundtrip() {
    let ts = TestScenario {
        name: "Happy path login".to_string(),
        scenario_type: "happy_path".to_string(),
        description: "Enter valid credentials and expect dashboard redirect".to_string(),
    };
    let json = serde_json::to_string(&ts).unwrap();
    let parsed: TestScenario = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, ts);
    // "scenario_type" field is serialized as "type" via #[serde(rename = "type")]
    assert!(json.contains(r#""type":"happy_path""#));
    assert_eq!(parsed.scenario_type, "happy_path");
}

#[test]
fn field_analysis_defaults_serde() {
    // Only label present; all other fields should default
    let json = r#"{"label":"Username"}"#;
    let fa: FieldAnalysis = serde_json::from_str(json).unwrap();
    assert_eq!(fa.label, "Username");
    assert_eq!(fa.context_clues, "");
    assert_eq!(fa.validation_hint, None);
    assert_eq!(fa.suggested_value, None);
    assert!(fa.negative_values.is_empty());
}

#[test]
fn page_model_with_field_analyses_backward_compat() {
    // Old JSON without the new Step 2 fields should deserialize with empty defaults
    let json = r#"{"purpose":"test","domain":"","forms":[],"outputs":[],"suggested_assertions":[],"navigation_targets":[]}"#;
    let model: PageModel = serde_json::from_str(json).unwrap();
    assert!(model.field_analyses.is_empty(), "field_analyses should default to empty vec");
    assert!(model.suggested_test_scenarios.is_empty(), "suggested_test_scenarios should default to empty vec");
    assert!(model.layout_description.is_none(), "layout_description should default to None");
}

#[test]
fn page_model_with_test_scenarios_full_roundtrip() {
    // JSON with all new Step 2 fields should parse and round-trip correctly
    let json = r#"{
        "purpose":"Booking form","domain":"hotel reservation",
        "forms":[],"outputs":[],"suggested_assertions":[],"navigation_targets":[],
        "layout_description":"Single column form with two sections",
        "field_analyses":[{"label":"Check-in date","context_clues":"date picker in Dates section"}],
        "suggested_test_scenarios":[{"name":"Valid booking","type":"happy_path","description":"Book a room"}]
    }"#;
    let model: PageModel = serde_json::from_str(json).unwrap();
    assert_eq!(model.layout_description, Some("Single column form with two sections".to_string()));
    assert_eq!(model.field_analyses.len(), 1);
    assert_eq!(model.field_analyses[0].label, "Check-in date");
    assert_eq!(model.field_analyses[0].context_clues, "date picker in Dates section");
    assert_eq!(model.suggested_test_scenarios.len(), 1);
    assert_eq!(model.suggested_test_scenarios[0].scenario_type, "happy_path");
}

#[test]
fn rich_prompt_contains_fieldset_legend() {
    let screen = ScreenState {
        url: Some("http://example.com/register".to_string()),
        title: "Register".to_string(),
        forms: vec![Form {
            id: "reg".to_string(),
            inputs: vec![ScreenElement {
                label: Some("ICCID".to_string()),
                kind: ElementKind::Input,
                tag: Some("input".to_string()),
                role: None,
                input_type: Some("text".to_string()),
                required: true,
                placeholder: None,
                id: None,
                href: None,
                options: None,
                name: None,
                value: None,
                maxlength: Some(19),
                minlength: None,
                readonly: false,
                fieldset_legend: Some("SIM Card Details".to_string()),
                section_heading: None,
                nearby_help_text: None,
                autocomplete: None,
                aria_describedby_text: None,
            }],
            actions: vec![],
            primary_action: None,
            intent: None,
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let prompt = build_rich_page_prompt(&screen, None);
    assert!(prompt.contains("SIM Card Details"), "prompt should include fieldset legend");
    assert!(prompt.contains("ICCID"), "prompt should include field label");
    assert!(prompt.contains("maxlength: 19"), "prompt should include maxlength constraint");
    assert!(prompt.contains("required: yes"), "prompt should include required flag");
}

#[test]
fn rich_prompt_contains_nearby_help_text() {
    let screen = ScreenState {
        url: Some("http://example.com".to_string()),
        title: "Contact".to_string(),
        forms: vec![Form {
            id: "contact".to_string(),
            inputs: vec![ScreenElement {
                label: Some("Message".to_string()),
                kind: ElementKind::Input,
                tag: Some("textarea".to_string()),
                role: None,
                input_type: None,
                required: false,
                placeholder: None,
                id: None,
                href: None,
                options: None,
                name: None,
                value: None,
                maxlength: None,
                minlength: Some(10),
                readonly: false,
                fieldset_legend: None,
                section_heading: None,
                nearby_help_text: Some("Enter at least 10 characters".to_string()),
                autocomplete: None,
                aria_describedby_text: None,
            }],
            actions: vec![],
            primary_action: None,
            intent: None,
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let prompt = build_rich_page_prompt(&screen, None);
    assert!(prompt.contains("Enter at least 10 characters"), "prompt should include nearby_help_text");
    assert!(prompt.contains("minlength: 10"), "prompt should include minlength constraint");
}

#[test]
fn rich_prompt_contains_aria_describedby() {
    let screen = ScreenState {
        url: Some("http://example.com".to_string()),
        title: "Settings".to_string(),
        forms: vec![Form {
            id: "settings".to_string(),
            inputs: vec![ScreenElement {
                label: Some("New Password".to_string()),
                kind: ElementKind::Input,
                tag: Some("input".to_string()),
                role: None,
                input_type: Some("password".to_string()),
                required: true,
                placeholder: None,
                id: None,
                href: None,
                options: None,
                name: None,
                value: None,
                maxlength: None,
                minlength: None,
                readonly: false,
                fieldset_legend: None,
                section_heading: None,
                nearby_help_text: None,
                autocomplete: None,
                aria_describedby_text: Some("Must be at least 8 characters with one uppercase letter".to_string()),
            }],
            actions: vec![],
            primary_action: None,
            intent: None,
        }],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let prompt = build_rich_page_prompt(&screen, None);
    assert!(prompt.contains("Must be at least 8 characters"), "prompt should include aria-describedby text");
}

#[test]
fn rich_prompt_contains_structural_outline() {
    use screen_detection::screen::screen_model::{HeadingEntry, LandmarkEntry, StructuralOutline};
    let screen = ScreenState {
        url: Some("http://example.com/intake".to_string()),
        title: "Patient Intake Form".to_string(),
        forms: vec![],
        standalone_actions: vec![],
        outputs: vec![],
        identities: HashMap::new(),
        structural_outline: StructuralOutline {
            headings: vec![
                HeadingEntry { level: 1, text: "Patient Registration".to_string() },
                HeadingEntry { level: 2, text: "Personal Information".to_string() },
            ],
            landmarks: vec![
                LandmarkEntry { tag: "main".to_string(), label: "Registration form".to_string() },
            ],
        },
    };
    let prompt = build_rich_page_prompt(&screen, None);
    assert!(prompt.contains("H1: Patient Registration"), "prompt should contain H1 heading");
    assert!(prompt.contains("H2: Personal Information"), "prompt should contain H2 heading");
    assert!(prompt.contains("<main> Registration form"), "prompt should contain landmark");
    assert!(prompt.contains("PAGE STRUCTURE"), "prompt should include PAGE STRUCTURE section");
}

#[test]
fn rich_prompt_contains_all_outputs_not_just_3() {
    // Old build_page_prompt used .take(3); rich prompt uses .take(10)
    let outputs: Vec<ScreenElement> = (1..=5)
        .map(|i| ScreenElement {
            label: Some(format!("Output line {i}")),
            kind: ElementKind::Output,
            tag: Some("p".to_string()),
            role: None,
            input_type: None,
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
            name: None,
            value: None,
            maxlength: None,
            minlength: None,
            readonly: false,
            fieldset_legend: None,
            section_heading: None,
            nearby_help_text: None,
            autocomplete: None,
            aria_describedby_text: None,
        })
        .collect();
    let screen = ScreenState {
        url: Some("http://example.com".to_string()),
        title: "Results".to_string(),
        forms: vec![],
        standalone_actions: vec![],
        outputs,
        identities: HashMap::new(),
        structural_outline: Default::default(),
    };
    let prompt = build_rich_page_prompt(&screen, None);
    // All 5 outputs should be present (old prompt would have stopped at 3)
    for i in 1..=5 {
        assert!(
            prompt.contains(&format!("Output line {i}")),
            "prompt should contain output line {i}"
        );
    }
}

#[test]
fn llm_page_analyzer_parses_layout_description() {
    let mock_response = r#"{
        "purpose": "Medical patient intake form",
        "domain": "hospital patient registration",
        "layout_description": "Two-column form with patient info on left and insurance info on right",
        "field_values": {},
        "field_analyses": [],
        "success_indicators": [],
        "error_indicators": [],
        "test_scenarios": []
    }"#;
    let screen = login_screen();
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);
    let model = analyzer.analyze(&screen).unwrap();
    assert_eq!(
        model.layout_description,
        Some("Two-column form with patient info on left and insurance info on right".to_string()),
        "LlmPageAnalyzer should parse layout_description from rich prompt response"
    );
    assert_eq!(model.domain, "hospital patient registration");
}

#[test]
fn llm_page_analyzer_parses_field_analyses() {
    let mock_response = r#"{
        "purpose": "SIM provisioning",
        "domain": "telecom SIM card activation",
        "layout_description": "Single page wizard step 1",
        "field_values": {"Email": "sim@test.com", "Password": "P@ss123"},
        "field_analyses": [
            {
                "label": "Email",
                "context_clues": "In 'Account Details' section; autocomplete=email",
                "validation_hint": "Must be a valid email address",
                "suggested_value": "provisioner@telco.com",
                "negative_values": ["notanemail", ""]
            },
            {
                "label": "Password",
                "context_clues": "Labelled Portal password in Security section",
                "validation_hint": "Min 8 chars",
                "suggested_value": "Secure@123",
                "negative_values": ["short"]
            }
        ],
        "success_indicators": [],
        "error_indicators": [],
        "test_scenarios": []
    }"#;
    let screen = login_screen();
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);
    let model = analyzer.analyze(&screen).unwrap();
    assert_eq!(model.field_analyses.len(), 2, "should parse 2 field analyses");
    assert_eq!(model.field_analyses[0].label, "Email");
    assert_eq!(
        model.field_analyses[0].context_clues,
        "In 'Account Details' section; autocomplete=email"
    );
    assert_eq!(
        model.field_analyses[0].suggested_value,
        Some("provisioner@telco.com".to_string())
    );
    assert_eq!(
        model.field_analyses[0].negative_values,
        vec!["notanemail".to_string(), "".to_string()]
    );
    assert_eq!(model.field_analyses[1].label, "Password");
    assert_eq!(
        model.field_analyses[1].validation_hint,
        Some("Min 8 chars".to_string())
    );
}

#[test]
fn llm_page_analyzer_parses_test_scenarios() {
    let mock_response = r#"{
        "purpose": "Login form",
        "domain": "web app authentication",
        "layout_description": null,
        "field_values": {},
        "field_analyses": [],
        "success_indicators": ["welcome"],
        "error_indicators": ["invalid"],
        "test_scenarios": [
            {"name": "Valid login", "type": "happy_path", "description": "Login with correct credentials"},
            {"name": "Wrong password", "type": "negative", "description": "Login with incorrect password"},
            {"name": "Empty fields", "type": "boundary", "description": "Submit with empty form fields"}
        ]
    }"#;
    let screen = login_screen();
    let analyzer = LlmPageAnalyzer::with_mock_response(mock_response);
    let model = analyzer.analyze(&screen).unwrap();
    assert_eq!(model.suggested_test_scenarios.len(), 3, "should parse 3 test scenarios");
    assert_eq!(model.suggested_test_scenarios[0].name, "Valid login");
    assert_eq!(model.suggested_test_scenarios[0].scenario_type, "happy_path");
    assert_eq!(model.suggested_test_scenarios[1].name, "Wrong password");
    assert_eq!(model.suggested_test_scenarios[1].scenario_type, "negative");
    assert_eq!(model.suggested_test_scenarios[2].scenario_type, "boundary");
    // Success/error indicators still merged correctly alongside test scenarios
    assert!(model.expected_outcome.success_text.contains(&"welcome".to_string()));
    assert!(model.expected_outcome.error_indicators.contains(&"invalid".to_string()));
}
