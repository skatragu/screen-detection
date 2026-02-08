use std::fmt;
use std::process::ExitStatus;

#[derive(Debug)]
pub enum AgentError {
    /// Node.js subprocess failed to spawn (extract.js or interact.js)
    SubprocessSpawn { script: String, source: std::io::Error },

    /// Node.js subprocess exited with non-zero status
    SubprocessFailed { script: String, status: ExitStatus, stderr: String },

    /// JSON parsing failed (from subprocess output or serde)
    JsonParse { context: String, source: serde_json::Error },

    /// JSON serialization failed (command to interact.js)
    JsonSerialize { context: String, source: serde_json::Error },

    /// Browser action reported failure (interact.js returned success=false)
    BrowserAction(String),

    /// DOM extraction returned unexpected structure
    DomStructure(String),

    /// Element not found in screen state
    ElementNotFound { element: String, context: String },

    /// Missing required data in screen state
    MissingState(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::SubprocessSpawn { script, source } => {
                write!(f, "Failed to spawn {} (is Node.js installed?): {}", script, source)
            }
            AgentError::SubprocessFailed { script, status, stderr } => {
                write!(f, "{} exited with {}: {}", script, status, stderr)
            }
            AgentError::JsonParse { context, source } => {
                write!(f, "JSON parse error ({}): {}", context, source)
            }
            AgentError::JsonSerialize { context, source } => {
                write!(f, "JSON serialize error ({}): {}", context, source)
            }
            AgentError::BrowserAction(msg) => {
                write!(f, "Browser action failed: {}", msg)
            }
            AgentError::DomStructure(msg) => {
                write!(f, "Unexpected DOM structure: {}", msg)
            }
            AgentError::ElementNotFound { element, context } => {
                write!(f, "Element '{}' not found: {}", element, context)
            }
            AgentError::MissingState(msg) => {
                write!(f, "Missing state: {}", msg)
            }
        }
    }
}

impl std::error::Error for AgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AgentError::SubprocessSpawn { source, .. } => Some(source),
            AgentError::JsonParse { source, .. } => Some(source),
            AgentError::JsonSerialize { source, .. } => Some(source),
            _ => None,
        }
    }
}
