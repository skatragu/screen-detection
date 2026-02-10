use std::collections::HashMap;

use screen_detection::spec::{
    context::TestContext,
    spec_model::{AssertionResult, AssertionSpec, TestResult, TestSpec, TestStep},
};

// =========================================================================
// Helpers
// =========================================================================

fn sample_test_spec() -> TestSpec {
    let mut values = HashMap::new();
    values.insert("query".to_string(), "Widget Pro".to_string());

    TestSpec {
        name: "Shopping cart checkout".into(),
        start_url: "https://shop.example.com/products".into(),
        steps: vec![
            TestStep::FillForm {
                form: "search".into(),
                values,
            },
            TestStep::Click {
                label: "Add to Cart".into(),
            },
            TestStep::Wait { duration_ms: 2000 },
            TestStep::Assert {
                assertions: vec![
                    AssertionSpec::TextPresent {
                        expected: "$49.98".into(),
                    },
                ],
            },
            TestStep::Click {
                label: "Checkout".into(),
            },
            TestStep::Assert {
                assertions: vec![
                    AssertionSpec::UrlContains {
                        expected: "/checkout".into(),
                    },
                ],
            },
        ],
    }
}

// =========================================================================
// TestSpec serde roundtrip tests
// =========================================================================

#[test]
fn test_spec_yaml_roundtrip() {
    let spec = sample_test_spec();

    let yaml = serde_yaml::to_string(&spec).expect("Failed to serialize TestSpec to YAML");
    println!("Serialized YAML:\n{}", yaml);

    let deserialized: TestSpec =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize TestSpec from YAML");

    assert_eq!(spec, deserialized, "Roundtrip must produce identical spec");
}

#[test]
fn test_spec_json_roundtrip() {
    let spec = sample_test_spec();

    let json = serde_json::to_string_pretty(&spec).expect("Failed to serialize to JSON");
    println!("Serialized JSON:\n{}", json);

    let deserialized: TestSpec =
        serde_json::from_str(&json).expect("Failed to deserialize from JSON");

    assert_eq!(spec, deserialized, "JSON roundtrip must produce identical spec");
}

#[test]
fn test_spec_deserialize_from_yaml_string() {
    let yaml = r#"
name: "Login test"
start_url: "https://app.example.com/login"
steps:
  - action: fill_and_submit
    form: "login"
    values:
      Email: "user@test.com"
      Password: "secret123"
    submit_label: "Sign In"
  - action: wait
    duration_ms: 1000
  - action: assert
    assertions:
      - type: url_contains
        expected: "/dashboard"
      - type: text_present
        expected: "Welcome"
      - type: text_absent
        expected: "Error"
"#;

    let spec: TestSpec = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    assert_eq!(spec.name, "Login test");
    assert_eq!(spec.start_url, "https://app.example.com/login");
    assert_eq!(spec.steps.len(), 3);

    // Verify FillAndSubmit step
    match &spec.steps[0] {
        TestStep::FillAndSubmit {
            form,
            values,
            submit_label,
        } => {
            assert_eq!(form, "login");
            assert_eq!(values.get("Email").unwrap(), "user@test.com");
            assert_eq!(values.get("Password").unwrap(), "secret123");
            assert_eq!(submit_label.as_deref(), Some("Sign In"));
        }
        other => panic!("Expected FillAndSubmit, got {:?}", other),
    }

    // Verify Wait step
    match &spec.steps[1] {
        TestStep::Wait { duration_ms } => assert_eq!(*duration_ms, 1000),
        other => panic!("Expected Wait, got {:?}", other),
    }

    // Verify Assert step with 3 assertions
    match &spec.steps[2] {
        TestStep::Assert { assertions } => {
            assert_eq!(assertions.len(), 3);
            assert!(matches!(&assertions[0], AssertionSpec::UrlContains { expected } if expected == "/dashboard"));
            assert!(matches!(&assertions[1], AssertionSpec::TextPresent { expected } if expected == "Welcome"));
            assert!(matches!(&assertions[2], AssertionSpec::TextAbsent { expected } if expected == "Error"));
        }
        other => panic!("Expected Assert, got {:?}", other),
    }
}

#[test]
fn test_spec_all_step_types_yaml() {
    let yaml = r##"
name: "All step types"
start_url: "https://example.com"
steps:
  - action: fill_form
    form: "search"
    values:
      query: "test"
  - action: fill_and_submit
    form: "login"
    values:
      user: "admin"
    submit_label: "Go"
  - action: click
    label: "Next"
  - action: navigate
    url: "https://example.com/page2"
  - action: wait
    duration_ms: 500
  - action: assert
    assertions:
      - type: url_equals
        expected: "https://example.com/page2"
      - type: title_contains
        expected: "Page 2"
      - type: element_text
        selector: "#heading"
        expected: "Welcome"
      - type: element_visible
        selector: ".banner"
      - type: element_count
        selector: "tr"
        expected: 5
"##;

    let spec: TestSpec = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    assert_eq!(spec.steps.len(), 6);

    // Verify Navigate step
    match &spec.steps[3] {
        TestStep::Navigate { url } => assert_eq!(url, "https://example.com/page2"),
        other => panic!("Expected Navigate, got {:?}", other),
    }

    // Verify all 5 assertion types in the Assert step
    match &spec.steps[5] {
        TestStep::Assert { assertions } => {
            assert_eq!(assertions.len(), 5);
            assert!(matches!(&assertions[0], AssertionSpec::UrlEquals { .. }));
            assert!(matches!(&assertions[1], AssertionSpec::TitleContains { .. }));
            assert!(matches!(&assertions[2], AssertionSpec::ElementText { .. }));
            assert!(matches!(&assertions[3], AssertionSpec::ElementVisible { .. }));
            assert!(matches!(&assertions[4], AssertionSpec::ElementCount { expected, .. } if *expected == 5));
        }
        other => panic!("Expected Assert, got {:?}", other),
    }
}

// =========================================================================
// TestContext tests
// =========================================================================

#[test]
fn test_context_tracks_assertions() {
    let mut ctx = TestContext::new();
    assert_eq!(ctx.current_step, 0);
    assert!(ctx.all_passed());
    assert_eq!(ctx.pass_count(), 0);
    assert_eq!(ctx.fail_count(), 0);
    assert_eq!(ctx.total_count(), 0);

    // Add passing assertion
    ctx.record_assertions(vec![AssertionResult {
        step_index: 0,
        spec: AssertionSpec::UrlContains {
            expected: "/home".into(),
        },
        passed: true,
        actual: Some("https://example.com/home".into()),
        message: None,
    }]);

    assert!(ctx.all_passed());
    assert_eq!(ctx.pass_count(), 1);
    assert_eq!(ctx.fail_count(), 0);
    assert_eq!(ctx.total_count(), 1);

    // Add failing assertion
    ctx.record_assertions(vec![AssertionResult {
        step_index: 1,
        spec: AssertionSpec::TextPresent {
            expected: "Welcome".into(),
        },
        passed: false,
        actual: Some("(page text, 200 chars)".into()),
        message: Some("Text 'Welcome' not found on page".into()),
    }]);

    assert!(!ctx.all_passed());
    assert_eq!(ctx.pass_count(), 1);
    assert_eq!(ctx.fail_count(), 1);
    assert_eq!(ctx.total_count(), 2);
}

#[test]
fn test_context_advance_step() {
    let mut ctx = TestContext::new();
    assert_eq!(ctx.current_step, 0);

    ctx.advance();
    assert_eq!(ctx.current_step, 1);

    ctx.advance();
    assert_eq!(ctx.current_step, 2);
}

#[test]
fn test_context_multiple_assertions_per_step() {
    let mut ctx = TestContext::new();

    // Record 3 assertions from step 0: 2 pass, 1 fail
    ctx.record_assertions(vec![
        AssertionResult {
            step_index: 0,
            spec: AssertionSpec::UrlContains {
                expected: "/dash".into(),
            },
            passed: true,
            actual: None,
            message: None,
        },
        AssertionResult {
            step_index: 0,
            spec: AssertionSpec::TextPresent {
                expected: "Hello".into(),
            },
            passed: true,
            actual: None,
            message: None,
        },
        AssertionResult {
            step_index: 0,
            spec: AssertionSpec::TextAbsent {
                expected: "Error".into(),
            },
            passed: false,
            actual: None,
            message: Some("Error text was found".into()),
        },
    ]);

    assert!(!ctx.all_passed());
    assert_eq!(ctx.pass_count(), 2);
    assert_eq!(ctx.fail_count(), 1);
    assert_eq!(ctx.total_count(), 3);
}

// =========================================================================
// TestResult serialization
// =========================================================================

#[test]
fn test_result_serializes_correctly() {
    let result = TestResult {
        spec_name: "My Test".into(),
        passed: true,
        steps_run: 5,
        assertion_results: vec![
            AssertionResult {
                step_index: 3,
                spec: AssertionSpec::UrlContains {
                    expected: "/home".into(),
                },
                passed: true,
                actual: Some("https://example.com/home".into()),
                message: None,
            },
        ],
        error: None,
    };

    let json = serde_json::to_string(&result).expect("serialize TestResult");
    assert!(json.contains("My Test"));
    assert!(json.contains("\"passed\":true"));
    assert!(json.contains("\"steps_run\":5"));
}

#[test]
fn test_result_with_error() {
    let result = TestResult {
        spec_name: "Failing Test".into(),
        passed: false,
        steps_run: 2,
        assertion_results: vec![],
        error: Some("Step 2 failed: element not found".into()),
    };

    let json = serde_json::to_string(&result).expect("serialize TestResult");
    assert!(json.contains("Failing Test"));
    assert!(json.contains("\"passed\":false"));
    assert!(json.contains("element not found"));
}

// =========================================================================
// Assertion spec YAML roundtrip tests
// =========================================================================

#[test]
fn test_element_text_yaml_roundtrip() {
    let spec = AssertionSpec::ElementText {
        selector: "#heading".into(),
        expected: "Welcome".into(),
    };
    let yaml = serde_yaml::to_string(&spec).expect("serialize");
    let deserialized: AssertionSpec = serde_yaml::from_str(&yaml).expect("deserialize");
    assert_eq!(spec, deserialized);
    assert!(yaml.contains("element_text"));
    assert!(yaml.contains("#heading"));
    assert!(yaml.contains("Welcome"));
}

#[test]
fn test_element_visible_yaml_roundtrip() {
    let spec = AssertionSpec::ElementVisible {
        selector: ".banner".into(),
    };
    let yaml = serde_yaml::to_string(&spec).expect("serialize");
    let deserialized: AssertionSpec = serde_yaml::from_str(&yaml).expect("deserialize");
    assert_eq!(spec, deserialized);
    assert!(yaml.contains("element_visible"));
    assert!(yaml.contains(".banner"));
}

#[test]
fn test_element_count_yaml_roundtrip() {
    let spec = AssertionSpec::ElementCount {
        selector: "tr".into(),
        expected: 5,
    };
    let yaml = serde_yaml::to_string(&spec).expect("serialize");
    let deserialized: AssertionSpec = serde_yaml::from_str(&yaml).expect("deserialize");
    assert_eq!(spec, deserialized);
    assert!(yaml.contains("element_count"));
    assert!(yaml.contains("tr"));
    assert!(yaml.contains("5"));
}

// =========================================================================
// Assertion evaluation structure tests
// =========================================================================

#[test]
fn test_element_text_assertion_result_structure() {
    // Test the shape of an ElementText assertion result (pass case)
    let result = AssertionResult {
        step_index: 2,
        spec: AssertionSpec::ElementText {
            selector: "#title".into(),
            expected: "Hello".into(),
        },
        passed: true,
        actual: Some("Hello World".into()),
        message: None,
    };
    assert!(result.passed);
    assert_eq!(result.step_index, 2);
    assert_eq!(result.actual, Some("Hello World".into()));

    // Test the shape of an ElementText assertion result (fail case: not found)
    let result_missing = AssertionResult {
        step_index: 2,
        spec: AssertionSpec::ElementText {
            selector: "#missing".into(),
            expected: "Hello".into(),
        },
        passed: false,
        actual: None,
        message: Some("Element '#missing' not found on page".into()),
    };
    assert!(!result_missing.passed);
    assert_eq!(result_missing.actual, None);
    assert!(result_missing.message.as_ref().unwrap().contains("not found"));
}

#[test]
fn test_element_visible_assertion_result_structure() {
    let result = AssertionResult {
        step_index: 1,
        spec: AssertionSpec::ElementVisible {
            selector: ".alert".into(),
        },
        passed: true,
        actual: Some("true".into()),
        message: None,
    };
    assert!(result.passed);

    let result_hidden = AssertionResult {
        step_index: 1,
        spec: AssertionSpec::ElementVisible {
            selector: ".hidden-el".into(),
        },
        passed: false,
        actual: Some("false".into()),
        message: Some("Element '.hidden-el' is not visible".into()),
    };
    assert!(!result_hidden.passed);
    assert!(result_hidden.message.as_ref().unwrap().contains("not visible"));
}

#[test]
fn test_element_count_assertion_result_structure() {
    let result = AssertionResult {
        step_index: 3,
        spec: AssertionSpec::ElementCount {
            selector: "li".into(),
            expected: 3,
        },
        passed: true,
        actual: Some("3".into()),
        message: None,
    };
    assert!(result.passed);
    assert_eq!(result.actual, Some("3".into()));

    let result_mismatch = AssertionResult {
        step_index: 3,
        spec: AssertionSpec::ElementCount {
            selector: "li".into(),
            expected: 5,
        },
        passed: false,
        actual: Some("3".into()),
        message: Some("Element 'li' count is 3 but expected 5".into()),
    };
    assert!(!result_mismatch.passed);
    assert!(result_mismatch.message.as_ref().unwrap().contains("count is 3"));
}
