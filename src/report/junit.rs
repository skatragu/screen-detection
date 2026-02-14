use crate::report::report_model::TestSuiteReport;

// ============================================================================
// JUnit XML reporter — standard CI integration format
// ============================================================================

/// Generate a JUnit XML report for CI systems (Jenkins, GitHub Actions, GitLab CI).
///
/// Produces standard JUnit XML:
/// ```xml
/// <?xml version="1.0" encoding="UTF-8"?>
/// <testsuite name="..." tests="3" failures="1" time="1.234">
///   <testcase name="Test 1" classname="screen-detection" />
///   <testcase name="Test 2" classname="screen-detection">
///     <failure message="1 assertion(s) failed" type="AssertionFailure">
///       Step 1: Title does not contain 'Login'
///     </failure>
///   </testcase>
/// </testsuite>
/// ```
pub fn generate_junit_xml(report: &TestSuiteReport) -> String {
    let time_attr = report
        .duration_ms
        .map(|ms| format!(" time=\"{:.3}\"", ms as f64 / 1000.0))
        .unwrap_or_default();

    let mut cases = String::new();
    for result in &report.test_results {
        if result.passed {
            cases.push_str(&format!(
                "  <testcase name=\"{}\" classname=\"screen-detection\" />\n",
                escape_xml(&result.spec_name)
            ));
        } else {
            // Collect failure details
            let failed_assertions: Vec<String> = result
                .assertion_results
                .iter()
                .filter(|ar| !ar.passed)
                .map(|ar| {
                    let msg = ar.message.as_deref().unwrap_or("assertion failed");
                    format!("Step {}: {}", ar.step_index, msg)
                })
                .collect();

            let failure_count = failed_assertions.len();
            let error_detail = result
                .error
                .as_ref()
                .map(|e| format!("Error: {}", e))
                .unwrap_or_default();

            let mut body_parts = failed_assertions;
            if !error_detail.is_empty() {
                body_parts.push(error_detail);
            }
            let failure_body = body_parts.join("\n");

            let failure_message = if failure_count > 0 {
                format!("{} assertion(s) failed", failure_count)
            } else {
                "execution error".to_string()
            };

            cases.push_str(&format!(
                "  <testcase name=\"{name}\" classname=\"screen-detection\">\n    <failure message=\"{message}\" type=\"AssertionFailure\">{body}</failure>\n  </testcase>\n",
                name = escape_xml(&result.spec_name),
                message = escape_xml(&failure_message),
                body = escape_xml(&failure_body),
            ));
        }
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"{name}\" tests=\"{tests}\" failures=\"{failures}\"{time}>\n{cases}</testsuite>\n",
        name = escape_xml(&report.suite_name),
        tests = report.total,
        failures = report.failed,
        time = time_attr,
        cases = cases,
    )
}

/// Escape XML special characters.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
