use screen_detection::canonical::diff::SemanticSignal;

use crate::common::{
    semantic_diff::{
        diff_between_pages, diff_mutating, diff_static, diff_static_with_initial_flag,
    },
    utils::{is_terminal_success, page},
};

mod common;

#[test]
fn initial_page_produces_screen_loaded() {
    let diff = diff_static(&page("01_search_form.html"));

    assert!(
        diff.signals.contains(&SemanticSignal::ScreenLoaded),
        "Expected ScreenLoaded for initial page"
    );

    assert!(
        !diff.signals.contains(&SemanticSignal::NoOp),
        "Initial page must not be NoOp"
    );
}

#[test]
fn form_submission_emits_signal() {
    let diff = diff_between_pages(
        &&page("01_search_form.html"),      // BEFORE
        &&page("02_search_submitted.html"), // AFTER
    );
    println!("diff={:?}", diff);
    assert!(diff.signals.contains(&SemanticSignal::FormSubmitted {
        form_id: "search".into()
    }));
}

#[test]
fn results_appearing_is_detected() {
    let diff = diff_between_pages(
        &&page("02_search_submitted.html"), // BEFORE
        &&page("03_search_results.html"),   // AFTER
    );

    println!("Diff = {:?}", diff);
    assert!(
        diff.signals.contains(&SemanticSignal::ResultsAppeared),
        "Expected ResultsAppeared signal"
    );
}

#[test]
fn direct_form_to_results_is_submission_not_navigation() {
    let diff = diff_between_pages(
        &page("01_search_form.html"),
        &page("03_search_results.html"),
    );

    assert!(
        diff.signals.iter().any(|s| matches!(
            s,
            SemanticSignal::FormSubmitted { form_id } if form_id == "search"
        )),
        "Expected form submission"
    );

    assert!(
        diff.signals.contains(&SemanticSignal::ResultsAppeared),
        "Expected results to appear"
    );

    assert!(
        !diff.signals.contains(&SemanticSignal::NavigationOccurred),
        "Submission with results must not be classified as navigation"
    );
}

#[test]
fn error_appearing_is_detected() {
    let diff = diff_between_pages(&page("02_search_submitted.html"), &page("05_error.html"));

    assert!(
        diff.signals.contains(&SemanticSignal::ErrorAppeared),
        "Expected ErrorAppeared when error first appears"
    );
}

#[test]
fn form_submission_is_detected() {
    let diff = diff_between_pages(
        &page("01_search_form.html"),
        &page("02_search_submitted.html"),
    );

    assert!(
        diff.signals
            .iter()
            .any(|s| matches!(s, SemanticSignal::FormSubmitted { .. })),
        "Expected FormSubmitted signal"
    );
}

#[test]
fn no_false_positive_signals() {
    let diff = diff_mutating(&page("01_search_form.html"), 100);

    assert_eq!(
        diff.signals,
        vec![SemanticSignal::NoOp],
        "Static diff should ONLY produce NoOp"
    );
}

#[test]
fn error_then_results_is_success() {
    let diff = diff_between_pages(&page("05_error.html"), &page("03_search_results.html"));
    println!("Diff = {:?}", diff);

    assert!(
        is_terminal_success(&diff),
        "Transition must be considered success"
    );

    assert!(
        !diff.signals.contains(&SemanticSignal::ErrorAppeared),
        "Old error must not persist"
    );
}

#[test]
fn form_disappears_with_content_is_submission() {
    let diff = diff_between_pages(&page("01_search_form.html"), &page("08_navigation.html"));

    assert!(
        diff.signals
            .iter()
            .any(|s| matches!(s, SemanticSignal::FormSubmitted { .. })),
        "Form disappearance with content is a submission"
    );
}

#[test]
fn repeated_results_page_is_noop() {
    let diff = diff_static_with_initial_flag(&page("03_search_results.html"), false);

    assert_eq!(
        diff.signals,
        vec![SemanticSignal::NoOp],
        "Repeated results page should be NoOp"
    );
}

#[test]
fn repeated_error_page_is_noop() {
    let diff = diff_static_with_initial_flag(&page("05_error.html"), false);

    assert_eq!(
        diff.signals,
        vec![SemanticSignal::NoOp],
        "Repeated error page should be NoOp"
    );
}

#[test]
fn correct_form_is_marked_submitted() {
    let diff = diff_between_pages(
        &page("06_multiple_forms.html"),
        &page("02_search_submitted.html"),
    );

    println!("Diff = {:?}", diff);

    assert!(
        diff.signals.iter().any(|s| matches!(
            s,
            SemanticSignal::FormSubmitted { form_id } if form_id == "search"
        )),
        "Correct form must be marked submitted"
    );
}
