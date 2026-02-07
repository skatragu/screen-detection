use serde_json::Value;
use std::process::Command;

pub fn extract_screen(url: &str) -> Value {
    let output = Command::new("node")
        .arg("../../node/dom-extraction/extract.js")
        .arg(url)
        .output()
        .expect("Failed to run Playwright");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // print!("Output from node:{}", stdout);
    serde_json::from_str(&stdout).expect("Invalid JSON from Playwright")
}
