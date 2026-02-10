use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A complete test specification. Built in-memory by AI (Phase 5) or
/// deserialized from YAML for human review and execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestSpec {
    /// Human-readable name for this test
    pub name: String,

    /// URL to navigate to before executing steps
    pub start_url: String,

    /// Ordered list of test steps to execute
    pub steps: Vec<TestStep>,
}

/// A single step in a test spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TestStep {
    /// Fill a single form field
    FillForm {
        form: String,
        values: HashMap<String, String>,
    },

    /// Fill all fields and submit a form in one step
    FillAndSubmit {
        form: String,
        values: HashMap<String, String>,
        submit_label: Option<String>,
    },

    /// Click a button/link by label
    Click {
        label: String,
    },

    /// Navigate to a different URL
    Navigate {
        url: String,
    },

    /// Wait for the page to settle
    Wait {
        duration_ms: u64,
    },

    /// Run assertions against the current page state
    Assert {
        assertions: Vec<AssertionSpec>,
    },
}

/// A single assertion to evaluate against the page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssertionSpec {
    /// Current URL contains the expected substring
    UrlContains { expected: String },

    /// Current URL exactly matches
    UrlEquals { expected: String },

    /// Page title contains the expected substring
    TitleContains { expected: String },

    /// Visible text on the page contains the expected string
    TextPresent { expected: String },

    /// No visible text on the page contains the expected string
    TextAbsent { expected: String },

    /// A specific element's text matches
    ElementText {
        selector: String,
        expected: String,
    },

    /// A specific element is visible on the page
    ElementVisible { selector: String },

    /// Count of matching elements equals expected
    ElementCount {
        selector: String,
        expected: u32,
    },
}

/// Result of evaluating a single assertion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssertionResult {
    /// Which step this assertion belongs to (0-indexed)
    pub step_index: usize,

    /// The assertion that was evaluated
    pub spec: AssertionSpec,

    /// Whether the assertion passed
    pub passed: bool,

    /// Actual value found (for debugging failed assertions)
    pub actual: Option<String>,

    /// Human-readable failure message
    pub message: Option<String>,
}

/// Result of running a complete test spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Name of the test spec that was run
    pub spec_name: String,

    /// Whether all steps and assertions passed
    pub passed: bool,

    /// Number of steps that were executed
    pub steps_run: usize,

    /// All assertion results collected during the run
    pub assertion_results: Vec<AssertionResult>,

    /// Error message if the test failed due to an error (not assertion failure)
    pub error: Option<String>,
}
