use crate::report::report_model::TestSuiteReport;

// ============================================================================
// HTML reporter — self-contained HTML report
// ============================================================================

/// Generate a self-contained HTML report.
///
/// Features:
/// - Green/red header based on overall pass/fail
/// - Summary bar with pass/fail/total counts
/// - Each test case in its own section
/// - Failed assertions highlighted in red
/// - Inline CSS (no external dependencies)
pub fn generate_html_report(report: &TestSuiteReport) -> String {
    let header_color = if report.all_passed() {
        "#4CAF50"
    } else {
        "#f44336"
    };

    let status_text = if report.all_passed() {
        "ALL TESTS PASSED"
    } else {
        "SOME TESTS FAILED"
    };

    let duration_text = report
        .duration_ms
        .map(|ms| format!(" in {:.1}s", ms as f64 / 1000.0))
        .unwrap_or_default();

    let mut test_cases = String::new();
    for result in &report.test_results {
        let case_class = if result.passed { "pass" } else { "fail" };
        let case_marker = if result.passed {
            "\u{2713}"
        } else {
            "\u{2717}"
        };

        test_cases.push_str(&format!(
            r#"<div class="test-case {class}">
<h3>{marker} {name}</h3>
<p>Steps: {steps} | Assertions: {assertions}</p>
"#,
            class = case_class,
            marker = case_marker,
            name = escape_html(&result.spec_name),
            steps = result.steps_run,
            assertions = result.assertion_results.len(),
        ));

        // Show error
        if let Some(ref error) = result.error {
            test_cases.push_str(&format!(
                "<p class=\"error\">Error: {}</p>\n",
                escape_html(error)
            ));
        }

        // Show failed assertions
        let failed: Vec<_> = result
            .assertion_results
            .iter()
            .filter(|ar| !ar.passed)
            .collect();
        if !failed.is_empty() {
            test_cases.push_str("<ul class=\"failures\">\n");
            for ar in failed {
                let msg = ar.message.as_deref().unwrap_or("assertion failed");
                test_cases.push_str(&format!(
                    "<li>Step {}: {}</li>\n",
                    ar.step_index,
                    escape_html(msg)
                ));
            }
            test_cases.push_str("</ul>\n");
        }

        test_cases.push_str("</div>\n");
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{suite_name} — Test Report</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; margin: 0; padding: 0; background: #f5f5f5; }}
.header {{ background: {header_color}; color: white; padding: 20px 30px; }}
.header h1 {{ margin: 0 0 8px 0; font-size: 24px; }}
.header p {{ margin: 0; font-size: 16px; opacity: 0.9; }}
.content {{ max-width: 900px; margin: 20px auto; padding: 0 20px; }}
.test-case {{ background: white; border-radius: 6px; padding: 16px 20px; margin-bottom: 12px; border-left: 4px solid #ccc; }}
.test-case.pass {{ border-left-color: #4CAF50; }}
.test-case.fail {{ border-left-color: #f44336; }}
.test-case h3 {{ margin: 0 0 8px 0; font-size: 16px; }}
.test-case p {{ margin: 4px 0; color: #666; font-size: 14px; }}
.test-case .error {{ color: #f44336; font-weight: bold; }}
.failures {{ margin: 8px 0 0 0; padding-left: 20px; }}
.failures li {{ color: #c62828; font-size: 13px; margin-bottom: 4px; }}
</style>
</head>
<body>
<div class="header">
<h1>{status_text}</h1>
<p>{suite_name}: {passed} passed, {failed} failed ({total} total){duration}</p>
</div>
<div class="content">
{test_cases}
</div>
</body>
</html>"##,
        suite_name = escape_html(&report.suite_name),
        header_color = header_color,
        status_text = status_text,
        passed = report.passed,
        failed = report.failed,
        total = report.total,
        duration = duration_text,
        test_cases = test_cases,
    )
}

/// Escape HTML special characters.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
