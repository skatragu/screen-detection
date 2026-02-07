use screen_detection::canonical::diff::{SemanticSignal, SemanticStateDiff};

pub fn page(name: &str) -> String {
    let base = std::env::current_dir().unwrap();
    let path = base.join("tests").join("fixtures").join(name);

    format!("file://{}", path.display())
}

pub fn is_terminal_success(diff: &SemanticStateDiff) -> bool {
    diff.signals.iter().any(|s| {
        matches!(
            s,
            SemanticSignal::ResultsAppeared | SemanticSignal::NavigationOccurred
        )
    })
}
