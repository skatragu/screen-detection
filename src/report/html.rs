use crate::report::report_model::TestSuiteReport;

// ============================================================================
// HTML reporter — self-contained HTML report with screenshots
// ============================================================================

/// Generate a self-contained HTML report.
///
/// Features:
/// - Green/red header based on overall pass/fail
/// - Summary bar with pass/fail/total counts
/// - Each test case in its own section with per-test duration
/// - Failed assertions highlighted in red
/// - Screenshots on failure embedded as base64 data URIs
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

        // Show per-test duration
        if let Some(ms) = result.duration_ms {
            test_cases.push_str(&format!(
                "<p class=\"timing\">Duration: {:.1}s</p>\n",
                ms as f64 / 1000.0
            ));
        }

        // Show retry info
        if result.retry_attempts > 0 {
            test_cases.push_str(&format!(
                "<p class=\"timing\">Retries: {} assertion retry attempt(s) used</p>\n",
                result.retry_attempts
            ));
        }

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

        // Embed screenshots on failure as base64 data URIs
        for screenshot_path in &result.screenshots {
            if let Ok(bytes) = std::fs::read(screenshot_path) {
                let b64 = base64_encode(&bytes);
                test_cases.push_str(&format!(
                    "<div class=\"screenshot\"><p>Screenshot on failure:</p><img src=\"data:image/png;base64,{}\" /></div>\n",
                    b64
                ));
            }
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
.test-case .timing {{ color: #888; font-size: 13px; }}
.failures {{ margin: 8px 0 0 0; padding-left: 20px; }}
.failures li {{ color: #c62828; font-size: 13px; margin-bottom: 4px; }}
.screenshot {{ margin: 12px 0; }}
.screenshot img {{ max-width: 100%; border: 1px solid #ddd; border-radius: 4px; }}
.screenshot p {{ font-size: 12px; color: #888; margin: 4px 0; }}
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

/// Encode bytes as base64 (no external dependency).
pub fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((combined >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((combined >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((combined >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(combined & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
