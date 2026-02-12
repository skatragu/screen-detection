use std::collections::HashMap;

use screen_detection::agent::page_model::{
    FieldModel, FieldType, FormModel, NavigationTarget, PageCategory, PageModel,
    SuggestedAssertion,
};
use screen_detection::explorer::app_map::{AppMap, ExplorerConfig, PageNode, Transition};
use screen_detection::explorer::explorer::{explore, is_same_origin, resolve_url};
use screen_detection::explorer::test_generator::{
    generate_form_test, generate_smoke_test, generate_test_plan, map_suggested_assertion,
};
use screen_detection::screen::screen_model::{
    ElementKind, Form, FormIntent, IntentSignal, ScreenElement,
};
use screen_detection::spec::spec_model::{AssertionSpec, TestStep};
use screen_detection::state::state_model::ScreenState;

// ============================================================================
// Helper builders (reuse same patterns as page_model_tests.rs)
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
        outputs: vec![],
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

fn search_page_model() -> PageModel {
    PageModel {
        purpose: "Search page for querying content".into(),
        category: PageCategory::Search,
        forms: vec![FormModel {
            form_id: "search".into(),
            purpose: "Search".into(),
            fields: vec![FieldModel {
                label: "Search query".into(),
                field_type: FieldType::Text,
                required: false,
                suggested_test_value: "test query".into(),
            }],
            submit_label: Some("Search".into()),
        }],
        outputs: vec![],
        suggested_assertions: vec![SuggestedAssertion {
            assertion_type: "title_contains".into(),
            expected: "Search".into(),
            description: "Verify page title contains 'Search'".into(),
        }],
        navigation_targets: vec![],
    }
}

// ============================================================================
// 1. ExplorerConfig defaults
// ============================================================================

#[test]
fn explorer_config_defaults() {
    let config = ExplorerConfig::default();
    assert_eq!(config.start_url, "");
    assert_eq!(config.max_pages, 10);
    assert_eq!(config.max_depth, 3);
    assert!(config.same_origin_only);
}

// ============================================================================
// 2. ExplorerConfig YAML roundtrip
// ============================================================================

#[test]
fn explorer_config_yaml_roundtrip() {
    let config = ExplorerConfig {
        start_url: "https://example.com".into(),
        max_pages: 20,
        max_depth: 5,
        same_origin_only: false,
    };
    let yaml = serde_yaml::to_string(&config).unwrap();
    let parsed: ExplorerConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.start_url, "https://example.com");
    assert_eq!(parsed.max_pages, 20);
    assert_eq!(parsed.max_depth, 5);
    assert!(!parsed.same_origin_only);
}

// ============================================================================
// 3. AppMap empty
// ============================================================================

#[test]
fn app_map_empty() {
    let map = AppMap::new();
    assert_eq!(map.page_count(), 0);
    assert!(!map.has_page("https://example.com"));
    assert!(map.transitions.is_empty());
}

// ============================================================================
// 4. AppMap add_page
// ============================================================================

#[test]
fn app_map_add_page() {
    let mut map = AppMap::new();
    let node = PageNode {
        url: "https://example.com/login".into(),
        title: "Login".into(),
        depth: 0,
        page_model: sample_page_model(),
    };
    map.add_page(node);

    assert_eq!(map.page_count(), 1);
    assert!(map.has_page("https://example.com/login"));
    assert!(!map.has_page("https://example.com/other"));
}

// ============================================================================
// 5. AppMap add_transition
// ============================================================================

#[test]
fn app_map_add_transition() {
    let mut map = AppMap::new();
    map.add_transition(Transition {
        from_url: "https://example.com/login".into(),
        to_url: "https://example.com/register".into(),
        label: "Sign Up".into(),
    });

    assert_eq!(map.transitions.len(), 1);
    assert_eq!(map.transitions[0].from_url, "https://example.com/login");
    assert_eq!(map.transitions[0].to_url, "https://example.com/register");
    assert_eq!(map.transitions[0].label, "Sign Up");
}

// ============================================================================
// 6. AppMap JSON roundtrip
// ============================================================================

#[test]
fn app_map_json_roundtrip() {
    let mut map = AppMap::new();
    map.add_page(PageNode {
        url: "https://example.com/login".into(),
        title: "Login".into(),
        depth: 0,
        page_model: sample_page_model(),
    });
    map.add_page(PageNode {
        url: "https://example.com/search".into(),
        title: "Search".into(),
        depth: 1,
        page_model: search_page_model(),
    });
    map.add_transition(Transition {
        from_url: "https://example.com/login".into(),
        to_url: "https://example.com/search".into(),
        label: "Go to Search".into(),
    });

    let json = serde_json::to_string(&map).unwrap();
    let parsed: AppMap = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.page_count(), 2);
    assert!(parsed.has_page("https://example.com/login"));
    assert!(parsed.has_page("https://example.com/search"));
    assert_eq!(parsed.transitions.len(), 1);
}

// ============================================================================
// 7. explore() single page — login
// ============================================================================

#[test]
fn explore_single_page_login() {
    let config = ExplorerConfig {
        start_url: "https://example.com/login".into(),
        ..ExplorerConfig::default()
    };
    let screen = login_screen();
    let map = explore(&config, &screen).unwrap();

    assert_eq!(map.page_count(), 1);
    assert!(map.has_page("https://example.com/login"));

    let node = &map.pages["https://example.com/login"];
    assert_eq!(node.depth, 0);
    assert_eq!(node.page_model.category, PageCategory::Login);
    assert_eq!(node.page_model.forms.len(), 1);
    assert_eq!(node.page_model.navigation_targets.len(), 2);
}

// ============================================================================
// 8. explore() single page — search
// ============================================================================

#[test]
fn explore_single_page_search() {
    let config = ExplorerConfig {
        start_url: "https://example.com/search".into(),
        ..ExplorerConfig::default()
    };
    let screen = search_screen();
    let map = explore(&config, &screen).unwrap();

    assert_eq!(map.page_count(), 1);
    assert!(map.has_page("https://example.com/search"));

    let node = &map.pages["https://example.com/search"];
    assert_eq!(node.page_model.category, PageCategory::Search);
    assert_eq!(node.page_model.forms.len(), 1);
    assert_eq!(node.page_model.forms[0].form_id, "search");
}

// ============================================================================
// 9. Smoke test generation
// ============================================================================

#[test]
fn smoke_test_generation() {
    let model = sample_page_model();
    let spec = generate_smoke_test("https://example.com/login", &model);

    assert_eq!(spec.name, "Smoke: Login page for user authentication");
    assert_eq!(spec.start_url, "https://example.com/login");

    // First step: Wait
    assert!(matches!(spec.steps[0], TestStep::Wait { duration_ms: 1000 }));

    // Second step: Assert with title_contains
    match &spec.steps[1] {
        TestStep::Assert { assertions } => {
            assert_eq!(assertions.len(), 1);
            match &assertions[0] {
                AssertionSpec::TitleContains { expected } => {
                    assert_eq!(expected, "Login");
                }
                other => panic!("Expected TitleContains, got {:?}", other),
            }
        }
        other => panic!("Expected Assert step, got {:?}", other),
    }
}

// ============================================================================
// 10. Form test generation
// ============================================================================

#[test]
fn form_test_generation() {
    let model = sample_page_model();
    let form = &model.forms[0];
    let spec = generate_form_test("https://example.com/login", form, &model);

    assert_eq!(
        spec.name,
        "Form: Authentication on Login page for user authentication"
    );
    assert_eq!(spec.start_url, "https://example.com/login");
    assert_eq!(spec.steps.len(), 1);

    match &spec.steps[0] {
        TestStep::FillAndSubmit {
            form: form_id,
            values,
            submit_label,
        } => {
            assert_eq!(form_id, "login");
            assert_eq!(values.get("Email"), Some(&"user@example.com".to_string()));
            assert_eq!(values.get("Password"), Some(&"TestPass123!".to_string()));
            assert_eq!(*submit_label, Some("Sign In".into()));
        }
        other => panic!("Expected FillAndSubmit step, got {:?}", other),
    }
}

// ============================================================================
// 11. generate_test_plan from multi-page AppMap
// ============================================================================

#[test]
fn generate_test_plan_full() {
    let mut map = AppMap::new();
    map.add_page(PageNode {
        url: "https://example.com/login".into(),
        title: "Login".into(),
        depth: 0,
        page_model: sample_page_model(),
    });
    map.add_page(PageNode {
        url: "https://example.com/search".into(),
        title: "Search".into(),
        depth: 1,
        page_model: search_page_model(),
    });

    let specs = generate_test_plan(&map);

    // Login page: 1 smoke + 1 form = 2
    // Search page: 1 smoke + 1 form = 2
    // Total: 4
    assert_eq!(specs.len(), 4);

    // Verify we have both smoke and form tests
    let smoke_count = specs.iter().filter(|s| s.name.starts_with("Smoke:")).count();
    let form_count = specs.iter().filter(|s| s.name.starts_with("Form:")).count();
    assert_eq!(smoke_count, 2);
    assert_eq!(form_count, 2);

    // All start_urls should be valid
    for spec in &specs {
        assert!(spec.start_url.starts_with("https://example.com/"));
    }
}

// ============================================================================
// 12. map_suggested_assertion — all supported types + unknown
// ============================================================================

#[test]
fn map_suggested_assertion_variants() {
    // title_contains
    let sa = SuggestedAssertion {
        assertion_type: "title_contains".into(),
        expected: "Login".into(),
        description: "test".into(),
    };
    assert!(matches!(
        map_suggested_assertion(&sa),
        Some(AssertionSpec::TitleContains { .. })
    ));

    // text_present
    let sa = SuggestedAssertion {
        assertion_type: "text_present".into(),
        expected: "Welcome".into(),
        description: "test".into(),
    };
    assert!(matches!(
        map_suggested_assertion(&sa),
        Some(AssertionSpec::TextPresent { .. })
    ));

    // text_absent
    let sa = SuggestedAssertion {
        assertion_type: "text_absent".into(),
        expected: "Error".into(),
        description: "test".into(),
    };
    assert!(matches!(
        map_suggested_assertion(&sa),
        Some(AssertionSpec::TextAbsent { .. })
    ));

    // url_contains
    let sa = SuggestedAssertion {
        assertion_type: "url_contains".into(),
        expected: "/dashboard".into(),
        description: "test".into(),
    };
    assert!(matches!(
        map_suggested_assertion(&sa),
        Some(AssertionSpec::UrlContains { .. })
    ));

    // url_equals
    let sa = SuggestedAssertion {
        assertion_type: "url_equals".into(),
        expected: "https://example.com".into(),
        description: "test".into(),
    };
    assert!(matches!(
        map_suggested_assertion(&sa),
        Some(AssertionSpec::UrlEquals { .. })
    ));

    // element_visible
    let sa = SuggestedAssertion {
        assertion_type: "element_visible".into(),
        expected: "#results".into(),
        description: "test".into(),
    };
    assert!(matches!(
        map_suggested_assertion(&sa),
        Some(AssertionSpec::ElementVisible { .. })
    ));

    // unknown → None
    let sa = SuggestedAssertion {
        assertion_type: "some_future_type".into(),
        expected: "x".into(),
        description: "test".into(),
    };
    assert!(map_suggested_assertion(&sa).is_none());
}

// ============================================================================
// 13. is_same_origin check
// ============================================================================

#[test]
fn is_same_origin_check() {
    // Same origin (same scheme + host)
    assert!(is_same_origin(
        "https://example.com/page1",
        "https://example.com/page2"
    ));
    assert!(is_same_origin(
        "https://example.com",
        "https://example.com/about"
    ));

    // Different host
    assert!(!is_same_origin(
        "https://example.com/page",
        "https://other.com/page"
    ));

    // Different scheme
    assert!(!is_same_origin(
        "https://example.com",
        "http://example.com"
    ));

    // With port
    assert!(is_same_origin(
        "https://example.com:8080/a",
        "https://example.com:8080/b"
    ));
    assert!(!is_same_origin(
        "https://example.com:8080/a",
        "https://example.com:9090/b"
    ));

    // No scheme → returns false
    assert!(!is_same_origin("example.com", "example.com"));
}

// ============================================================================
// 14. resolve_url — absolute vs relative
// ============================================================================

#[test]
fn resolve_url_absolute() {
    // Absolute URLs are returned as-is
    assert_eq!(
        resolve_url("https://example.com", "https://example.com/about"),
        Some("https://example.com/about".into())
    );
    assert_eq!(
        resolve_url("https://example.com", "http://other.com"),
        Some("http://other.com".into())
    );

    // Non-URL labels return None
    assert_eq!(resolve_url("https://example.com", "Sign Up"), None);
    assert_eq!(resolve_url("https://example.com", "About Us"), None);
    assert_eq!(resolve_url("https://example.com", "/relative/path"), None);
}

// ============================================================================
// 15. AppMap no duplicate pages (HashMap dedup)
// ============================================================================

#[test]
fn app_map_no_duplicate_pages() {
    let mut map = AppMap::new();

    let node1 = PageNode {
        url: "https://example.com/login".into(),
        title: "Login v1".into(),
        depth: 0,
        page_model: sample_page_model(),
    };
    map.add_page(node1);
    assert_eq!(map.page_count(), 1);

    // Adding same URL again replaces the entry
    let node2 = PageNode {
        url: "https://example.com/login".into(),
        title: "Login v2".into(),
        depth: 0,
        page_model: sample_page_model(),
    };
    map.add_page(node2);
    assert_eq!(map.page_count(), 1); // Still 1, not 2

    // The title should be the updated one
    assert_eq!(map.pages["https://example.com/login"].title, "Login v2");
}
