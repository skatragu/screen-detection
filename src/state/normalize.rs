use crate::screen::screen_model::{OutputRegion, ScreenElement, Volatility};

pub fn normalize_output_text(raw: &str) -> Option<String> {
    let text = raw.trim();

    if text.is_empty() {
        return None;
    }

    // Drop obvious JS blobs
    if text.contains("function(")
        || text.contains("var ")
        || text.contains("window.")
        || text.contains("document.")
    {
        return None;
    }

    // Drop very long, token-like strings
    let non_alpha_ratio = text
        .chars()
        .filter(|c| !c.is_alphabetic() && !c.is_whitespace())
        .count() as f32
        / text.len().max(1) as f32;

    if non_alpha_ratio > 0.6 {
        return None;
    }

    // Collapse whitespace
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    if normalized.len() < 3 {
        return None;
    }

    Some(normalized)
}

pub fn infer_output_region(el: &ScreenElement) -> OutputRegion {
    let text = el.label.as_deref().unwrap_or("").to_lowercase();

    if text.contains("footer") || text.contains("privacy") || text.contains("terms") {
        OutputRegion::Footer
    } else if text.contains("header") || text.contains("sign in") || text.contains("login") {
        OutputRegion::Header
    } else {
        // IMPORTANT: safe fallback
        OutputRegion::Main
    }
}

pub fn classify_volatility(text: &str) -> Volatility {
    if text.len() > 200 {
        Volatility::Volatile
    } else {
        Volatility::Stable
    }
}

pub fn text_fingerprint(text: &str) -> String {
    use sha1::{Digest, Sha1};

    let mut hasher = Sha1::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}
