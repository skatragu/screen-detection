use serde::{Deserialize, Serialize};

/// Configuration for test runner resilience features.
///
/// Controls assertion retry behavior, screenshot capture on failure,
/// and screenshot storage directory. Used by `TestRunner::run_with_config()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// Max retries for Assert steps when assertions fail (default: 2)
    #[serde(default = "default_max_retries")]
    pub max_assertion_retries: usize,

    /// Delay in ms between assertion retries (default: 500)
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,

    /// Take screenshot on test failure (default: true)
    #[serde(default = "default_true")]
    pub screenshot_on_failure: bool,

    /// Directory for screenshot files (default: "screenshots")
    #[serde(default = "default_screenshot_dir")]
    pub screenshot_dir: String,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            max_assertion_retries: 2,
            retry_delay_ms: 500,
            screenshot_on_failure: true,
            screenshot_dir: "screenshots".to_string(),
        }
    }
}

fn default_max_retries() -> usize {
    2
}
fn default_retry_delay() -> u64 {
    500
}
fn default_true() -> bool {
    true
}
fn default_screenshot_dir() -> String {
    "screenshots".to_string()
}
