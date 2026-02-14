use crate::report::report_model::TestSuiteReport;
use crate::spec::spec_model::AssertionSpec;

// ============================================================================
// Console reporter — formatted terminal output
// ============================================================================

/// Format a test suite report for terminal output.
///
/// Produces output like:
/// ```text
/// === Test Suite: My Suite ===
///
/// ✓ PASS  Test name 1 (3 steps, 2 assertions)
/// ✗ FAIL  Test name 2 (5 steps, 4 assertions)
///     [FAIL] Step 2: TitleContains — expected "Login", actual "Home"
///
/// === Results: 1 passed, 1 failed (2 total) ===
/// ```
pub fn format_console_report(report: &TestSuiteReport) -> String {
    let mut out = String::new();

    out.push_str(&format!("=== Test Suite: {} ===\n\n", report.suite_name));

    for result in &report.test_results {
        let assertion_count = result.assertion_results.len();
        let marker = if result.passed {
            "\u{2713} PASS"
        } else {
            "\u{2717} FAIL"
        };

        out.push_str(&format!(
            "{}  {} ({} steps, {} assertions)\n",
            marker, result.spec_name, result.steps_run, assertion_count
        ));

        // Show error if the test failed due to execution error
        if let Some(ref error) = result.error {
            out.push_str(&format!("    [ERROR] {}\n", error));
        }

        // Show failed assertions
        if !result.passed {
            for ar in &result.assertion_results {
                if !ar.passed {
                    let assertion_name = format_assertion_type(&ar.spec);
                    let detail = ar
                        .message
                        .as_deref()
                        .unwrap_or("assertion failed");
                    out.push_str(&format!(
                        "    [FAIL] Step {}: {} — {}\n",
                        ar.step_index, assertion_name, detail
                    ));
                }
            }
        }
    }

    // Summary line
    out.push_str(&format!(
        "\n=== Results: {} passed, {} failed ({} total)",
        report.passed, report.failed, report.total
    ));

    if let Some(ms) = report.duration_ms {
        let secs = ms as f64 / 1000.0;
        out.push_str(&format!(" in {:.1}s", secs));
    }

    out.push_str(" ===\n");

    out
}

/// Format an AssertionSpec variant name for display.
fn format_assertion_type(spec: &AssertionSpec) -> &'static str {
    match spec {
        AssertionSpec::UrlContains { .. } => "UrlContains",
        AssertionSpec::UrlEquals { .. } => "UrlEquals",
        AssertionSpec::TitleContains { .. } => "TitleContains",
        AssertionSpec::TextPresent { .. } => "TextPresent",
        AssertionSpec::TextAbsent { .. } => "TextAbsent",
        AssertionSpec::ElementText { .. } => "ElementText",
        AssertionSpec::ElementVisible { .. } => "ElementVisible",
        AssertionSpec::ElementCount { .. } => "ElementCount",
    }
}
