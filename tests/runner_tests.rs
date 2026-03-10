use screen_detection::cli::config::RunConfig;
use screen_detection::report::console::format_console_report;
use screen_detection::report::html::generate_html_report;
use screen_detection::report::junit::generate_junit_xml;
use screen_detection::report::report_model::TestSuiteReport;
use screen_detection::spec::runner_config::RunnerConfig;
use screen_detection::spec::spec_model::{AssertionResult, AssertionSpec, TestResult};

// ============================================================================
// Helpers
// ============================================================================

fn make_result(duration_ms: Option<u128>, screenshots: Vec<String>, retry_attempts: usize) -> TestResult {
    TestResult {
        spec_name: "Test: Sample".to_string(),
        passed: true,
        steps_run: 2,
        assertion_results: vec![AssertionResult {
            step_index: 1,
            spec: AssertionSpec::TitleContains { expected: "Home".into() },
            passed: true,
            actual: Some("Home Page".into()),
            message: None,
        }],
        error: None,
        duration_ms,
        screenshots,
        retry_attempts,
    }
}

fn failing_result_with_duration(duration_ms: Option<u128>) -> TestResult {
    TestResult {
        spec_name: "Test: Failing".to_string(),
        passed: false,
        steps_run: 3,
        assertion_results: vec![AssertionResult {
            step_index: 2,
            spec: AssertionSpec::TitleContains { expected: "Login".into() },
            passed: false,
            actual: Some("Home Page".into()),
            message: Some("Title does not contain 'Login'".into()),
        }],
        error: None,
        duration_ms,
        screenshots: vec![],
        retry_attempts: 0,
    }
}

fn single_result_report(result: TestResult) -> TestSuiteReport {
    TestSuiteReport::from_results("Runner Tests", vec![result])
}

// ============================================================================
// 1. RunnerConfig defaults
// ============================================================================

#[test]
fn runner_config_defaults() {
    let config = RunnerConfig::default();
    assert_eq!(config.max_assertion_retries, 2);
    assert_eq!(config.retry_delay_ms, 500);
    assert!(config.screenshot_on_failure);
    assert_eq!(config.screenshot_dir, "screenshots");
}

// ============================================================================
// 2. RunnerConfig YAML roundtrip
// ============================================================================

#[test]
fn runner_config_yaml_roundtrip() {
    let config = RunnerConfig {
        max_assertion_retries: 3,
        retry_delay_ms: 1000,
        screenshot_on_failure: false,
        screenshot_dir: "ci-screenshots".to_string(),
    };

    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize RunnerConfig to YAML");
    let restored: RunnerConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize RunnerConfig from YAML");

    assert_eq!(restored.max_assertion_retries, 3);
    assert_eq!(restored.retry_delay_ms, 1000);
    assert!(!restored.screenshot_on_failure);
    assert_eq!(restored.screenshot_dir, "ci-screenshots");
}

// ============================================================================
// 3. TestResult duration_ms field: JSON roundtrip
// ============================================================================

#[test]
fn test_result_duration_field() {
    let result = make_result(Some(1500), vec![], 0);
    let json = serde_json::to_string(&result).expect("Failed to serialize");
    let restored: TestResult = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(restored.duration_ms, Some(1500));
}

// ============================================================================
// 4. TestResult screenshots field: JSON roundtrip
// ============================================================================

#[test]
fn test_result_screenshots_field() {
    let result = make_result(None, vec!["screenshots/test_step0_failure.png".into()], 0);
    let json = serde_json::to_string(&result).expect("Failed to serialize");
    let restored: TestResult = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(restored.screenshots, vec!["screenshots/test_step0_failure.png"]);
}

// ============================================================================
// 5. TestResult retry_attempts field: JSON roundtrip
// ============================================================================

#[test]
fn test_result_retry_attempts_field() {
    let result = make_result(None, vec![], 2);
    let json = serde_json::to_string(&result).expect("Failed to serialize");
    let restored: TestResult = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(restored.retry_attempts, 2);
}

// ============================================================================
// 6. TestResult backward compatibility: old JSON without new fields
// ============================================================================

#[test]
fn test_result_backward_compat() {
    // JSON that matches the old TestResult format (no duration_ms, screenshots, retry_attempts)
    let old_json = r#"{
        "spec_name": "Old Test",
        "passed": true,
        "steps_run": 2,
        "assertion_results": [],
        "error": null
    }"#;

    let result: TestResult =
        serde_json::from_str(old_json).expect("Old format should deserialize fine");

    assert_eq!(result.spec_name, "Old Test");
    assert!(result.passed);
    assert_eq!(result.duration_ms, None);
    assert!(result.screenshots.is_empty());
    assert_eq!(result.retry_attempts, 0);
}

// ============================================================================
// 7. Empty screenshots vec is omitted from JSON output
// ============================================================================

#[test]
fn test_result_skip_empty_screenshots() {
    let result = make_result(None, vec![], 0);
    let json = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(
        !json.contains("\"screenshots\""),
        "Empty screenshots vec should be skipped in JSON output, but got: {}",
        json
    );
}

// ============================================================================
// 8. None duration_ms is omitted from JSON output
// ============================================================================

#[test]
fn test_result_skip_none_duration() {
    let result = make_result(None, vec![], 0);
    let json = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(
        !json.contains("\"duration_ms\""),
        "None duration_ms should be skipped in JSON output, but got: {}",
        json
    );
}

// ============================================================================
// 9. Console report shows duration
// ============================================================================

#[test]
fn console_report_shows_duration() {
    let result = make_result(Some(1500), vec![], 0);
    let report = single_result_report(result);
    let output = format_console_report(&report);

    assert!(
        output.contains("[1.5s]"),
        "Console report should show duration [1.5s], got:\n{}",
        output
    );
}

// ============================================================================
// 10. Console report shows retry info when retries > 0
// ============================================================================

#[test]
fn console_report_shows_retry_info() {
    let result = make_result(None, vec![], 1);
    let report = single_result_report(result);
    let output = format_console_report(&report);

    assert!(
        output.contains("[RETRY]"),
        "Console report should show [RETRY] when retry_attempts > 0, got:\n{}",
        output
    );
}

// ============================================================================
// 11. Console report does NOT show retry info when retries == 0
// ============================================================================

#[test]
fn console_report_no_retry_when_zero() {
    let result = make_result(None, vec![], 0);
    let report = single_result_report(result);
    let output = format_console_report(&report);

    assert!(
        !output.contains("[RETRY]"),
        "Console report should NOT show [RETRY] when retry_attempts == 0, got:\n{}",
        output
    );
}

// ============================================================================
// 12. HTML report shows per-test duration
// ============================================================================

#[test]
fn html_report_shows_duration() {
    let result = make_result(Some(2000), vec![], 0);
    let report = single_result_report(result);
    let html = generate_html_report(&report);

    assert!(
        html.contains("Duration:"),
        "HTML report should contain 'Duration:', got length: {}",
        html.len()
    );
    assert!(
        html.contains("2.0s"),
        "HTML report should contain '2.0s' for 2000ms duration, got length: {}",
        html.len()
    );
}

// ============================================================================
// 13. HTML report contains screenshot CSS class
// ============================================================================

#[test]
fn html_report_screenshot_css() {
    let result = make_result(None, vec![], 0);
    let report = single_result_report(result);
    let html = generate_html_report(&report);

    assert!(
        html.contains(".screenshot"),
        "HTML report should contain .screenshot CSS class"
    );
}

// ============================================================================
// 14. JUnit report adds time attribute when duration is present
// ============================================================================

#[test]
fn junit_testcase_time_attribute() {
    let result = make_result(Some(1500), vec![], 0);
    let report = single_result_report(result);
    let xml = generate_junit_xml(&report);

    assert!(
        xml.contains("time=\"1.500\""),
        "JUnit XML should contain time=\"1.500\" for 1500ms, got:\n{}",
        xml
    );
}

// ============================================================================
// 15. JUnit report has no time attribute when duration is absent
// ============================================================================

#[test]
fn junit_testcase_no_time_when_none() {
    let result = make_result(None, vec![], 0);
    let report = single_result_report(result);
    let xml = generate_junit_xml(&report);

    // The testcase element itself should not have a time attribute
    // Find the testcase line and verify no time= in it
    let has_time_on_testcase = xml
        .lines()
        .filter(|line| line.contains("testcase"))
        .any(|line| line.contains("time="));

    assert!(
        !has_time_on_testcase,
        "JUnit XML testcase should NOT have time= attribute when duration is None, got:\n{}",
        xml
    );
}

// ============================================================================
// 16. HTML report embeds screenshot as base64 when file exists
// ============================================================================

#[test]
fn html_report_embeds_screenshot_when_file_exists() {
    // Write a temp PNG file
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join("runner_test_screenshot.png");
    std::fs::write(&tmp_path, b"\x89PNG fake png data").expect("Failed to write temp file");

    let path_str = tmp_path.to_string_lossy().to_string();
    let result = failing_result_with_duration(None);
    let result_with_screenshot = TestResult {
        screenshots: vec![path_str],
        ..result
    };

    let report = single_result_report(result_with_screenshot);
    let html = generate_html_report(&report);

    // Clean up before asserting (don't leave temp files)
    let _ = std::fs::remove_file(&tmp_path);

    assert!(
        html.contains("data:image/png;base64,"),
        "HTML report should embed screenshot as base64 data URI"
    );
}

// ============================================================================
// 17. HTML report silently skips screenshot when file doesn't exist
// ============================================================================

#[test]
fn html_report_skips_screenshot_when_file_missing() {
    let result = failing_result_with_duration(None);
    let result_with_missing = TestResult {
        screenshots: vec!["nonexistent/path/shot.png".into()],
        ..result
    };

    let report = single_result_report(result_with_missing);
    let html = generate_html_report(&report);

    assert!(
        !html.contains("data:image/png;base64,"),
        "HTML report should silently skip missing screenshot file"
    );
}

// ============================================================================
// 18. RunConfig YAML roundtrip with runner fields
// ============================================================================

#[test]
fn run_config_yaml_roundtrip() {
    let config = RunConfig {
        format: "html".to_string(),
        output: Some("report.html".to_string()),
        max_retries: 3,
        retry_delay_ms: 1000,
        screenshot_on_failure: false,
        screenshot_dir: "ci/screenshots".to_string(),
    };

    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize RunConfig");
    let restored: RunConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize RunConfig");

    assert_eq!(restored.format, "html");
    assert_eq!(restored.output, Some("report.html".to_string()));
    assert_eq!(restored.max_retries, 3);
    assert_eq!(restored.retry_delay_ms, 1000);
    assert!(!restored.screenshot_on_failure);
    assert_eq!(restored.screenshot_dir, "ci/screenshots");
}

// ============================================================================
// Phase 14 Step 5: console/HTML/JUnit output for UrlNotContains assertion
// ============================================================================

#[test]
fn url_not_contains_in_report() {
    // A failed UrlNotContains assertion result
    let failed_result = TestResult {
        spec_name: "Form: Login test".to_string(),
        passed: false,
        steps_run: 2,
        assertion_results: vec![AssertionResult {
            step_index: 1,
            spec: AssertionSpec::UrlNotContains { expected: "/login".into() },
            passed: false,
            actual: Some("https://example.com/login".into()),
            message: Some("URL should not contain '/login' but does".into()),
        }],
        error: None,
        duration_ms: None,
        screenshots: vec![],
        retry_attempts: 0,
    };

    let report = single_result_report(failed_result);

    // Console report should name the assertion correctly
    let console = format_console_report(&report);
    assert!(
        console.contains("UrlNotContains"),
        "Console report should display 'UrlNotContains' for failed assertion, got:\n{}",
        console
    );
    assert!(
        console.contains("[FAIL]"),
        "Console report should show [FAIL] for failed UrlNotContains, got:\n{}",
        console
    );

    // HTML report should contain the failure message text
    let html = generate_html_report(&report);
    assert!(
        html.contains("should not contain"),
        "HTML report should contain the UrlNotContains failure message, got length: {}",
        html.len()
    );

    // JUnit report should have a failure element
    let xml = generate_junit_xml(&report);
    assert!(
        xml.contains("<failure"),
        "JUnit XML should have <failure element for failed test, got:\n{}",
        xml
    );
}
