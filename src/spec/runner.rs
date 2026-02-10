use crate::agent::error::AgentError;
use crate::browser::playwright::SelectorHint;
use crate::browser::session::BrowserSession;
use crate::spec::context::TestContext;
use crate::spec::spec_model::{AssertionResult, AssertionSpec, TestResult, TestSpec, TestStep};

/// Executes a TestSpec step-by-step using a BrowserSession.
pub struct TestRunner;

impl TestRunner {
    /// Run a complete test spec against a browser session.
    ///
    /// Returns a TestResult with pass/fail status, assertion results,
    /// and any error that occurred during execution.
    pub fn run(spec: &TestSpec, session: &mut BrowserSession) -> TestResult {
        let mut ctx = TestContext::new();

        // Navigate to the start URL
        if let Err(e) = session.navigate(&spec.start_url) {
            return TestResult {
                spec_name: spec.name.clone(),
                passed: false,
                steps_run: 0,
                assertion_results: ctx.assertion_results,
                error: Some(format!("Failed to navigate to start_url: {}", e)),
            };
        }

        // Execute each step
        for (i, step) in spec.steps.iter().enumerate() {
            ctx.current_step = i;

            match Self::execute_step(step, i, session, &mut ctx) {
                Ok(()) => {}
                Err(e) => {
                    return TestResult {
                        spec_name: spec.name.clone(),
                        passed: false,
                        steps_run: i + 1,
                        assertion_results: ctx.assertion_results,
                        error: Some(format!("Step {} failed: {}", i, e)),
                    };
                }
            }
        }

        let passed = ctx.all_passed();
        TestResult {
            spec_name: spec.name.clone(),
            passed,
            steps_run: spec.steps.len(),
            assertion_results: ctx.assertion_results,
            error: None,
        }
    }

    /// Execute a single step.
    fn execute_step(
        step: &TestStep,
        step_index: usize,
        session: &mut BrowserSession,
        ctx: &mut TestContext,
    ) -> Result<(), AgentError> {
        match step {
            TestStep::FillForm { form, values } => {
                for (label, value) in values {
                    let selector = Self::input_selector(label, Some(form));
                    session.fill(&selector, value)?;
                }
                Ok(())
            }

            TestStep::FillAndSubmit {
                form,
                values,
                submit_label,
            } => {
                // Fill all inputs
                for (label, value) in values {
                    let selector = Self::input_selector(label, Some(form));
                    session.fill(&selector, value)?;
                }
                // Click submit if provided
                if let Some(label) = submit_label {
                    let selector = Self::button_selector(label);
                    session.click(&selector)?;
                }
                Ok(())
            }

            TestStep::Click { label } => {
                let selector = Self::button_selector(label);
                session.click(&selector)
            }

            TestStep::Navigate { url } => session.navigate(url),

            TestStep::Wait { duration_ms } => session.wait_idle(*duration_ms),

            TestStep::Assert { assertions } => {
                let results = Self::evaluate_assertions(assertions, step_index, session);
                ctx.record_assertions(results);
                Ok(())
            }
        }
    }

    /// Evaluate a list of assertions against the current page state.
    fn evaluate_assertions(
        assertions: &[AssertionSpec],
        step_index: usize,
        session: &mut BrowserSession,
    ) -> Vec<AssertionResult> {
        assertions
            .iter()
            .map(|spec| Self::evaluate_one(spec, step_index, session))
            .collect()
    }

    /// Evaluate a single assertion.
    fn evaluate_one(
        spec: &AssertionSpec,
        step_index: usize,
        session: &mut BrowserSession,
    ) -> AssertionResult {
        match spec {
            AssertionSpec::UrlContains { expected } => {
                match session.current_url() {
                    Ok(url) => {
                        let passed = url.contains(expected.as_str());
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(url),
                            message: if passed {
                                None
                            } else {
                                Some(format!("URL does not contain '{}'", expected))
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to get URL: {}", e)),
                    },
                }
            }

            AssertionSpec::UrlEquals { expected } => {
                match session.current_url() {
                    Ok(url) => {
                        let passed = url == *expected;
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(url),
                            message: if passed {
                                None
                            } else {
                                Some(format!("URL does not equal '{}'", expected))
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to get URL: {}", e)),
                    },
                }
            }

            AssertionSpec::TitleContains { expected } => {
                // Extract DOM to get title
                match session.extract() {
                    Ok(data) => {
                        let title = data["title"].as_str().unwrap_or("").to_string();
                        let passed = title.contains(expected.as_str());
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(title),
                            message: if passed {
                                None
                            } else {
                                Some(format!("Title does not contain '{}'", expected))
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to extract DOM: {}", e)),
                    },
                }
            }

            AssertionSpec::TextPresent { expected } => {
                match session.extract() {
                    Ok(data) => {
                        let all_text = Self::collect_text_from_dom(&data);
                        let passed = all_text.contains(&expected.to_lowercase());
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(format!("(page text, {} chars)", all_text.len())),
                            message: if passed {
                                None
                            } else {
                                Some(format!("Text '{}' not found on page", expected))
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to extract DOM: {}", e)),
                    },
                }
            }

            AssertionSpec::TextAbsent { expected } => {
                match session.extract() {
                    Ok(data) => {
                        let all_text = Self::collect_text_from_dom(&data);
                        let passed = !all_text.contains(&expected.to_lowercase());
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(format!("(page text, {} chars)", all_text.len())),
                            message: if passed {
                                None
                            } else {
                                Some(format!("Text '{}' was found on page but should be absent", expected))
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to extract DOM: {}", e)),
                    },
                }
            }

            AssertionSpec::ElementText { selector, expected } => {
                match session.query_text(selector) {
                    Ok(Some(text)) => {
                        let passed = text.to_lowercase().contains(&expected.to_lowercase());
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(text),
                            message: if passed {
                                None
                            } else {
                                Some(format!(
                                    "Element '{}' text does not contain '{}'",
                                    selector, expected
                                ))
                            },
                        }
                    }
                    Ok(None) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Element '{}' not found on page", selector)),
                    },
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to query element text: {}", e)),
                    },
                }
            }

            AssertionSpec::ElementVisible { selector } => {
                match session.query_visible(selector) {
                    Ok(visible) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: visible,
                        actual: Some(format!("{}", visible)),
                        message: if visible {
                            None
                        } else {
                            Some(format!("Element '{}' is not visible", selector))
                        },
                    },
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to query element visibility: {}", e)),
                    },
                }
            }

            AssertionSpec::ElementCount {
                selector,
                expected,
            } => {
                match session.query_count(selector) {
                    Ok(count) => {
                        let passed = count == *expected;
                        AssertionResult {
                            step_index,
                            spec: spec.clone(),
                            passed,
                            actual: Some(format!("{}", count)),
                            message: if passed {
                                None
                            } else {
                                Some(format!(
                                    "Element '{}' count is {} but expected {}",
                                    selector, count, expected
                                ))
                            },
                        }
                    }
                    Err(e) => AssertionResult {
                        step_index,
                        spec: spec.clone(),
                        passed: false,
                        actual: None,
                        message: Some(format!("Failed to query element count: {}", e)),
                    },
                }
            }
        }
    }

    /// Collect all visible text from extracted DOM data (lowercased for matching).
    fn collect_text_from_dom(data: &serde_json::Value) -> String {
        let mut texts = Vec::new();
        if let Some(dom) = data["dom"].as_array() {
            for el in dom {
                if let Some(text) = el["text"].as_str() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        texts.push(trimmed.to_lowercase());
                    }
                }
            }
        }
        texts.join(" ")
    }

    /// Build a SelectorHint for an input field by label and optional form ID.
    fn input_selector(label: &str, form_id: Option<&str>) -> SelectorHint {
        SelectorHint {
            role: Some("textbox".into()),
            name: Some(label.to_string()),
            tag: Some("input".into()),
            input_type: None,
            form_id: form_id.map(|s| s.to_string()),
        }
    }

    /// Build a SelectorHint for a button/link by label.
    fn button_selector(label: &str) -> SelectorHint {
        SelectorHint {
            role: Some("button".into()),
            name: Some(label.to_string()),
            tag: None,
            input_type: None,
            form_id: None,
        }
    }
}
