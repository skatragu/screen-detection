use serde::{Deserialize, Serialize};

use crate::spec::spec_model::TestResult;

// ============================================================================
// Test suite report — aggregates multiple TestResult instances
// ============================================================================

/// Aggregated report for a suite of test runs.
///
/// Built from a `Vec<TestResult>` via `from_results()`. Consumed by
/// console, HTML, and JUnit reporters to produce human-readable output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteReport {
    /// Name of the test suite
    pub suite_name: String,

    /// Total number of tests
    pub total: usize,

    /// Number of passing tests
    pub passed: usize,

    /// Number of failing tests
    pub failed: usize,

    /// Total execution duration in milliseconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,

    /// Individual test results
    pub test_results: Vec<TestResult>,
}

impl TestSuiteReport {
    /// Build a suite report from a list of test results.
    ///
    /// Automatically computes total, passed, and failed counts.
    pub fn from_results(suite_name: &str, results: Vec<TestResult>) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        Self {
            suite_name: suite_name.to_string(),
            total,
            passed,
            failed,
            duration_ms: None,
            test_results: results,
        }
    }

    /// Set the total execution duration.
    pub fn with_duration(mut self, duration_ms: u128) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Whether all tests in the suite passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}
