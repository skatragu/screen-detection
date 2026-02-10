use crate::spec::spec_model::AssertionResult;

/// Tracks the execution state and results of a running test.
#[derive(Debug, Clone)]
pub struct TestContext {
    /// Current step index (0-based)
    pub current_step: usize,

    /// All assertion results collected during execution
    pub assertion_results: Vec<AssertionResult>,
}

impl TestContext {
    pub fn new() -> Self {
        TestContext {
            current_step: 0,
            assertion_results: Vec::new(),
        }
    }

    /// Record assertion results from a step.
    pub fn record_assertions(&mut self, results: Vec<AssertionResult>) {
        self.assertion_results.extend(results);
    }

    /// Advance to the next step.
    pub fn advance(&mut self) {
        self.current_step += 1;
    }

    /// Check if all recorded assertions passed.
    pub fn all_passed(&self) -> bool {
        self.assertion_results.iter().all(|r| r.passed)
    }

    /// Count of passing assertions.
    pub fn pass_count(&self) -> usize {
        self.assertion_results.iter().filter(|r| r.passed).count()
    }

    /// Count of failing assertions.
    pub fn fail_count(&self) -> usize {
        self.assertion_results.iter().filter(|r| !r.passed).count()
    }

    /// Total number of assertions evaluated.
    pub fn total_count(&self) -> usize {
        self.assertion_results.len()
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}
