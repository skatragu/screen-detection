use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::error::AgentError;
use crate::browser::playwright::SelectorHint;

/// Request sent to browser_server.js over stdin (one JSON line).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BrowserRequest {
    Navigate {
        cmd: &'static str,
        url: String,
    },
    Extract {
        cmd: &'static str,
    },
    Action {
        cmd: &'static str,
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        selector: Option<SelectorHint>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    Screenshot {
        cmd: &'static str,
        path: String,
    },
    CurrentUrl {
        cmd: &'static str,
    },
    QueryText {
        cmd: &'static str,
        selector: String,
    },
    QueryVisible {
        cmd: &'static str,
        selector: String,
    },
    QueryCount {
        cmd: &'static str,
        selector: String,
    },
    Quit {
        cmd: &'static str,
    },
}

impl BrowserRequest {
    pub fn navigate(url: &str) -> Self {
        BrowserRequest::Navigate {
            cmd: "navigate",
            url: url.to_string(),
        }
    }

    pub fn extract() -> Self {
        BrowserRequest::Extract { cmd: "extract" }
    }

    pub fn fill(selector: &SelectorHint, value: &str) -> Self {
        BrowserRequest::Action {
            cmd: "action",
            action: "fill".into(),
            selector: Some(selector.clone()),
            value: Some(value.to_string()),
            duration_ms: None,
        }
    }

    pub fn click(selector: &SelectorHint) -> Self {
        BrowserRequest::Action {
            cmd: "action",
            action: "click".into(),
            selector: Some(selector.clone()),
            value: None,
            duration_ms: None,
        }
    }

    pub fn wait(duration_ms: u64) -> Self {
        BrowserRequest::Action {
            cmd: "action",
            action: "wait".into(),
            selector: None,
            value: None,
            duration_ms: Some(duration_ms),
        }
    }

    pub fn screenshot(path: &str) -> Self {
        BrowserRequest::Screenshot {
            cmd: "screenshot",
            path: path.to_string(),
        }
    }

    pub fn current_url() -> Self {
        BrowserRequest::CurrentUrl { cmd: "current_url" }
    }

    pub fn query_text(selector: &str) -> Self {
        BrowserRequest::QueryText {
            cmd: "query_text",
            selector: selector.to_string(),
        }
    }

    pub fn query_visible(selector: &str) -> Self {
        BrowserRequest::QueryVisible {
            cmd: "query_visible",
            selector: selector.to_string(),
        }
    }

    pub fn query_count(selector: &str) -> Self {
        BrowserRequest::QueryCount {
            cmd: "query_count",
            selector: selector.to_string(),
        }
    }

    pub fn quit() -> Self {
        BrowserRequest::Quit { cmd: "quit" }
    }
}

/// Response received from browser_server.js over stdout (one JSON line).
#[derive(Debug, Deserialize)]
pub struct BrowserResponse {
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub ready: Option<bool>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub visible: Option<bool>,
    #[serde(default)]
    pub count: Option<u32>,
}

/// A persistent browser session backed by browser_server.js.
///
/// Launches a long-lived Node.js process that keeps a Chromium browser open.
/// Commands are sent as NDJSON over stdin, responses read from stdout.
pub struct BrowserSession {
    child: Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    current_url: Option<String>,
}

impl BrowserSession {
    /// Launch a new browser session by spawning browser_server.js.
    pub fn launch() -> Result<Self, AgentError> {
        let mut child = Command::new("node")
            .arg("../../node/dom-extraction/browser_server.js")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AgentError::SubprocessSpawn {
                script: "browser_server.js".into(),
                source: e,
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AgentError::SessionIO("Failed to capture stdin of browser_server.js".into())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            AgentError::SessionIO("Failed to capture stdout of browser_server.js".into())
        })?;

        let mut reader = BufReader::new(stdout);

        // Wait for the ready signal
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| {
            AgentError::SessionIO(format!("Failed to read ready signal: {}", e))
        })?;

        let response: BrowserResponse = serde_json::from_str(line.trim()).map_err(|e| {
            AgentError::JsonParse {
                context: "browser_server.js ready signal".into(),
                source: e,
            }
        })?;

        if !response.ok || response.ready != Some(true) {
            return Err(AgentError::SessionProtocol {
                command: "launch".into(),
                error: "Did not receive ready signal from browser_server.js".into(),
            });
        }

        Ok(BrowserSession {
            child,
            stdin,
            reader,
            current_url: None,
        })
    }

    /// Send a request and read the response.
    fn send(&mut self, request: &BrowserRequest) -> Result<BrowserResponse, AgentError> {
        let json = serde_json::to_string(request).map_err(|e| AgentError::JsonSerialize {
            context: "BrowserRequest".into(),
            source: e,
        })?;

        writeln!(self.stdin, "{}", json).map_err(|e| {
            AgentError::SessionIO(format!("Failed to write to browser_server.js stdin: {}", e))
        })?;

        self.stdin.flush().map_err(|e| {
            AgentError::SessionIO(format!("Failed to flush browser_server.js stdin: {}", e))
        })?;

        let mut line = String::new();
        self.reader.read_line(&mut line).map_err(|e| {
            AgentError::SessionIO(format!("Failed to read from browser_server.js stdout: {}", e))
        })?;

        if line.trim().is_empty() {
            return Err(AgentError::SessionIO(
                "Empty response from browser_server.js (process may have died)".into(),
            ));
        }

        let response: BrowserResponse =
            serde_json::from_str(line.trim()).map_err(|e| AgentError::JsonParse {
                context: "browser_server.js response".into(),
                source: e,
            })?;

        Ok(response)
    }

    /// Send a request and verify it succeeded.
    fn send_ok(&mut self, request: &BrowserRequest, command_name: &str) -> Result<BrowserResponse, AgentError> {
        let response = self.send(request)?;
        if !response.ok {
            return Err(AgentError::SessionProtocol {
                command: command_name.into(),
                error: response.error.unwrap_or_else(|| "Unknown error".into()),
            });
        }
        Ok(response)
    }

    /// Navigate to a URL.
    pub fn navigate(&mut self, url: &str) -> Result<(), AgentError> {
        let request = BrowserRequest::navigate(url);
        self.send_ok(&request, "navigate")?;
        self.current_url = Some(url.to_string());
        Ok(())
    }

    /// Extract DOM from the current page.
    pub fn extract(&mut self) -> Result<Value, AgentError> {
        let request = BrowserRequest::extract();
        let response = self.send_ok(&request, "extract")?;
        response.data.ok_or_else(|| AgentError::SessionProtocol {
            command: "extract".into(),
            error: "No data in extract response".into(),
        })
    }

    /// Fill an input element.
    pub fn fill(&mut self, selector: &SelectorHint, value: &str) -> Result<(), AgentError> {
        let request = BrowserRequest::fill(selector, value);
        self.send_ok(&request, "fill")?;
        Ok(())
    }

    /// Click an element.
    pub fn click(&mut self, selector: &SelectorHint) -> Result<(), AgentError> {
        let request = BrowserRequest::click(selector);
        self.send_ok(&request, "click")?;
        Ok(())
    }

    /// Wait for the page to settle.
    pub fn wait_idle(&mut self, ms: u64) -> Result<(), AgentError> {
        let request = BrowserRequest::wait(ms);
        self.send_ok(&request, "wait")?;
        Ok(())
    }

    /// Take a screenshot.
    pub fn screenshot(&mut self, path: &str) -> Result<(), AgentError> {
        let request = BrowserRequest::screenshot(path);
        self.send_ok(&request, "screenshot")?;
        Ok(())
    }

    /// Get the current URL from the browser.
    pub fn current_url(&mut self) -> Result<String, AgentError> {
        let request = BrowserRequest::current_url();
        let response = self.send_ok(&request, "current_url")?;
        let url = response.url.ok_or_else(|| AgentError::SessionProtocol {
            command: "current_url".into(),
            error: "No URL in current_url response".into(),
        })?;
        self.current_url = Some(url.clone());
        Ok(url)
    }

    /// Query the text content of an element by CSS selector.
    /// Returns None if the element is not found.
    pub fn query_text(&mut self, selector: &str) -> Result<Option<String>, AgentError> {
        let request = BrowserRequest::query_text(selector);
        let response = self.send_ok(&request, "query_text")?;
        Ok(response.text)
    }

    /// Query whether an element is visible by CSS selector.
    pub fn query_visible(&mut self, selector: &str) -> Result<bool, AgentError> {
        let request = BrowserRequest::query_visible(selector);
        let response = self.send_ok(&request, "query_visible")?;
        Ok(response.visible.unwrap_or(false))
    }

    /// Query the count of elements matching a CSS selector.
    pub fn query_count(&mut self, selector: &str) -> Result<u32, AgentError> {
        let request = BrowserRequest::query_count(selector);
        let response = self.send_ok(&request, "query_count")?;
        Ok(response.count.unwrap_or(0))
    }

    /// Get the last known URL (cached, no browser call).
    pub fn last_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    /// Quit the browser session.
    pub fn quit(&mut self) -> Result<(), AgentError> {
        let request = BrowserRequest::quit();
        // Best-effort quit — don't fail hard if process is already gone
        let _ = self.send(&request);
        let _ = self.child.wait();
        Ok(())
    }
}

impl Drop for BrowserSession {
    fn drop(&mut self) {
        // Best-effort cleanup
        let _ = self.quit();
    }
}
