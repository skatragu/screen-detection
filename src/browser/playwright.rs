use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;

pub fn extract_screen(url: &str) -> Value {
    let output = Command::new("node")
        .arg("../../node/dom-extraction/extract.js")
        .arg(url)
        .output()
        .expect("Failed to run Playwright");

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("Invalid JSON from Playwright")
}

/// Selector hints used by interact.js to locate elements in the DOM.
#[derive(Debug, Clone, Serialize)]
pub struct SelectorHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>, // ARIA role, e.g. "textbox", "button"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>, // accessible name (aria-label or visible text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>, // HTML tag, e.g. "input", "button", "a"
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>, // type attribute, e.g. "text", "submit", "email"
    #[serde(rename = "formId", skip_serializing_if = "Option::is_none")]
    pub form_id: Option<String>, // parent form ID, e.g. "search"
}

/// Command sent to interact.js for execution.
#[derive(Debug, Serialize)]
pub struct BrowserCommand {
    pub action: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<SelectorHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Response from interact.js.
#[derive(Debug, Deserialize)]
pub struct BrowserResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Execute a browser action via the interact.js Node.js subprocess.
pub fn execute_browser_action(command: &BrowserCommand) -> Result<(), String> {
    let command_json = serde_json::to_string(command)
        .map_err(|e| format!("Failed to serialize command: {}", e))?;

    let output = Command::new("node")
        .arg("../../node/dom-extraction/interact.js")
        .arg(&command_json)
        .output()
        .map_err(|e| format!("Failed to spawn interact.js: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() {
        eprintln!("[interact.js] {}", stderr.trim());
    }

    let result: BrowserResult = serde_json::from_str(&stdout)
        .map_err(|e| format!("Invalid JSON from interact.js: {} (stdout: {})", e, stdout))?;

    if result.success {
        Ok(())
    } else {
        Err(result.error.unwrap_or_else(|| "Unknown error".into()))
    }
}
