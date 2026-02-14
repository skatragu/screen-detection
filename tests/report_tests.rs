use screen_detection::report::console::format_console_report;
use screen_detection::report::html::generate_html_report;
use screen_detection::report::junit::generate_junit_xml;
use screen_detection::report::report_model::TestSuiteReport;
use screen_detection::spec::spec_model::{AssertionResult, AssertionSpec, TestResult};

// ============================================================================
// Helper builders
// ============================================================================

fn passing_result(name: &str) -> TestResult {
    TestResult {
        spec_name: name.to_string(),
        passed: true,
        steps_run: 3,
        assertion_results: vec![AssertionResult {
            step_index: 2,
            spec: AssertionSpec::TitleContains {
                expected: "Home".into(),
            },
            passed: true,
            actual: Some("Home Page".into()),
            message: None,
        }],
        error: None,
    }
}

fn failing_result(name: &str) -> TestResult {
    TestResult {
        spec_name: name.to_string(),
        passed: false,
        steps_run: 5,
        assertion_results: vec![
            AssertionResult {
                step_index: 1,
                spec: AssertionSpec::TitleContains {
                    expected: "Login".into(),
                },
                passed: false,
                actual: Some("Home Page".into()),
                message: Some("Title does not contain 'Login'".into()),
            },
            AssertionResult {
                step_index: 3,
                spec: AssertionSpec::TextPresent {
                    expected: "Welcome".into(),
                },
                passed: false,
                actual: Some("(page text, 1234 chars)".into()),
                message: Some("Text 'Welcome' not found on page".into()),
            },
        ],
        error: None,
    }
}

fn mixed_suite_report() -> TestSuiteReport {
    TestSuiteReport::from_results(
        "Integration Tests",
        vec![
            passing_result("Smoke: Home Page"),
            passing_result("Smoke: About Page"),
            failing_result("Form: Login"),
        ],
    )
}

// ============================================================================
// 1. Suite report counts
// ============================================================================

#[test]
fn suite_report_from_results_counts() {
    let report = mixed_suite_report();
    assert_eq!(report.total, 3);
    assert_eq!(report.passed, 2);
    assert_eq!(report.failed, 1);
    assert_eq!(report.suite_name, "Integration Tests");
}

// ============================================================================
// 2. All passed
// ============================================================================

#[test]
fn suite_report_all_passed() {
    let report = TestSuiteReport::from_results(
        "All Green",
        vec![passing_result("Test A"), passing_result("Test B")],
    );
    assert!(report.all_passed());
}

// ============================================================================
// 3. Not all passed
// ============================================================================

#[test]
fn suite_report_not_all_passed() {
    let report = mixed_suite_report();
    assert!(!report.all_passed());
}

// ============================================================================
// 4. With duration
// ============================================================================

#[test]
fn suite_report_with_duration() {
    let report = TestSuiteReport::from_results("Suite", vec![]).with_duration(1234);
    assert_eq!(report.duration_ms, Some(1234));
}

// ============================================================================
// 5. Empty suite
// ============================================================================

#[test]
fn suite_report_empty() {
    let report = TestSuiteReport::from_results("Empty", vec![]);
    assert_eq!(report.total, 0);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failed, 0);
    assert!(report.all_passed());
}

// ============================================================================
// 6. JSON roundtrip
// ============================================================================

#[test]
fn suite_report_json_roundtrip() {
    let report = mixed_suite_report().with_duration(5000);
    let json = serde_json::to_string(&report).unwrap();
    let parsed: TestSuiteReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.suite_name, "Integration Tests");
    assert_eq!(parsed.total, 3);
    assert_eq!(parsed.passed, 2);
    assert_eq!(parsed.failed, 1);
    assert_eq!(parsed.duration_ms, Some(5000));
    assert_eq!(parsed.test_results.len(), 3);
}

// ============================================================================
// 7. Console report — pass marker
// ============================================================================

#[test]
fn console_report_pass_marker() {
    let report = TestSuiteReport::from_results("Suite", vec![passing_result("Good Test")]);
    let output = format_console_report(&report);
    assert!(output.contains("\u{2713} PASS"));
    assert!(output.contains("Good Test"));
}

// ============================================================================
// 8. Console report — fail marker
// ============================================================================

#[test]
fn console_report_fail_marker() {
    let report = TestSuiteReport::from_results("Suite", vec![failing_result("Bad Test")]);
    let output = format_console_report(&report);
    assert!(output.contains("\u{2717} FAIL"));
    assert!(output.contains("Bad Test"));
}

// ============================================================================
// 9. Console report — summary line
// ============================================================================

#[test]
fn console_report_summary_line() {
    let report = mixed_suite_report();
    let output = format_console_report(&report);
    assert!(output.contains("2 passed"));
    assert!(output.contains("1 failed"));
    assert!(output.contains("3 total"));
}

// ============================================================================
// 10. Console report — failure details
// ============================================================================

#[test]
fn console_report_failure_details() {
    let report = TestSuiteReport::from_results("Suite", vec![failing_result("Failing")]);
    let output = format_console_report(&report);
    assert!(output.contains("[FAIL]"));
    assert!(output.contains("Title does not contain 'Login'"));
    assert!(output.contains("Text 'Welcome' not found on page"));
}

// ============================================================================
// 11. HTML report — structure
// ============================================================================

#[test]
fn html_report_structure() {
    let report = mixed_suite_report();
    let html = generate_html_report(&report);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html"));
    assert!(html.contains("</html>"));
    assert!(html.contains("Integration Tests"));
}

// ============================================================================
// 12. HTML report — pass color (green)
// ============================================================================

#[test]
fn html_report_pass_color() {
    let report = TestSuiteReport::from_results("Suite", vec![passing_result("Test A")]);
    let html = generate_html_report(&report);
    assert!(html.contains("#4CAF50"));
    assert!(html.contains("ALL TESTS PASSED"));
}

// ============================================================================
// 13. HTML report — fail color (red)
// ============================================================================

#[test]
fn html_report_fail_color() {
    let report = mixed_suite_report();
    let html = generate_html_report(&report);
    assert!(html.contains("#f44336"));
    assert!(html.contains("SOME TESTS FAILED"));
}

// ============================================================================
// 14. JUnit XML — structure
// ============================================================================

#[test]
fn junit_xml_structure() {
    let report = mixed_suite_report();
    let xml = generate_junit_xml(&report);
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("<testsuite"));
    assert!(xml.contains("</testsuite>"));
    assert!(xml.contains("tests=\"3\""));
    assert!(xml.contains("failures=\"1\""));
}

// ============================================================================
// 15. JUnit XML — failure element
// ============================================================================

#[test]
fn junit_xml_failure_element() {
    let report = TestSuiteReport::from_results("Suite", vec![failing_result("Broken Test")]);
    let xml = generate_junit_xml(&report);
    assert!(xml.contains("<failure"));
    assert!(xml.contains("assertion(s) failed"));
    assert!(xml.contains("Broken Test"));
}
