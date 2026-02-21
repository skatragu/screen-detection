use screen_detection::{
    screen::{
        intent::infer_form_intent,
        screen_model::{ElementKind, Form, ScreenElement},
    },
    state::normalize::normalize_output_text,
};

// =========================================================================
// normalize_output_text edge cases
// =========================================================================

#[test]
fn normalize_output_text_filters_junk() {
    assert_eq!(normalize_output_text(""), None, "Empty string");
    assert_eq!(normalize_output_text("   "), None, "Whitespace only");
    assert_eq!(normalize_output_text("function(x){return x}"), None, "JS blob");
    assert_eq!(normalize_output_text("var x = 42"), None, "JS var");
    assert_eq!(normalize_output_text("window.location"), None, "JS window");
    assert_eq!(normalize_output_text("document.getElementById"), None, "JS document");
    assert_eq!(normalize_output_text("a1!@#$%^&*()"), None, "High non-alpha ratio");
    assert_eq!(normalize_output_text("ab"), None, "Too short after normalize");
}

#[test]
fn normalize_output_text_normalizes_valid_text() {
    assert_eq!(
        normalize_output_text("  Hello   World  "),
        Some("hello world".into()),
        "Collapses whitespace and lowercases"
    );
    assert_eq!(
        normalize_output_text("Search Results: 42 items"),
        Some("search results: 42 items".into()),
        "Normalizes mixed-case text"
    );
    assert_eq!(
        normalize_output_text("This is a valid sentence"),
        Some("this is a valid sentence".into()),
        "Plain text passes through"
    );
}

// =========================================================================
// Form intent scoring boundaries
// =========================================================================

#[test]
fn form_intent_scoring_boundaries() {
    // No signals â†’ Unknown, confidence 0.0
    let plain_form = Form {
        id: "plain".into(),
        inputs: vec![ScreenElement {
            label: Some("Name".into()),
            kind: ElementKind::Input,
            tag: Some("input".into()),
            role: None,
            input_type: Some("text".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        actions: vec![ScreenElement {
            label: Some("Submit".into()),
            kind: ElementKind::Action,
            tag: Some("button".into()),
            role: None,
            input_type: Some("submit".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        primary_action: None,
        intent: None,
    };
    let intent = infer_form_intent(&plain_form);
    assert_eq!(intent.label, "Unknown", "No auth signals â†’ Unknown");
    assert_eq!(intent.confidence, 0.0, "No signals â†’ 0.0 confidence");

    // Password only (score=0.4) â†’ NOT > 0.4, so "Unknown"
    let password_form = Form {
        id: "pw".into(),
        inputs: vec![ScreenElement {
            label: Some("Password".into()),
            kind: ElementKind::Input,
            tag: Some("input".into()),
            role: None,
            input_type: Some("password".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        actions: vec![ScreenElement {
            label: Some("Submit".into()),
            kind: ElementKind::Action,
            tag: Some("button".into()),
            role: None,
            input_type: Some("submit".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        primary_action: None,
        intent: None,
    };
    let intent = infer_form_intent(&password_form);
    assert_eq!(intent.label, "Unknown", "score=0.4 is NOT > 0.4, so Unknown");

    // Login action only (score=0.4) â†’ "Unknown"
    let login_form = Form {
        id: "login".into(),
        inputs: vec![ScreenElement {
            label: Some("Username".into()),
            kind: ElementKind::Input,
            tag: Some("input".into()),
            role: None,
            input_type: Some("text".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        actions: vec![ScreenElement {
            label: Some("Login".into()),
            kind: ElementKind::Action,
            tag: Some("button".into()),
            role: None,
            input_type: Some("submit".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        primary_action: None,
        intent: None,
    };
    let intent = infer_form_intent(&login_form);
    assert_eq!(intent.label, "Unknown", "Login action only â†’ score=0.4 â†’ Unknown");

    // Password + Sign In (score=0.8) â†’ Authentication
    let auth_form = Form {
        id: "auth".into(),
        inputs: vec![ScreenElement {
            label: Some("Password".into()),
            kind: ElementKind::Input,
            tag: Some("input".into()),
            role: None,
            input_type: Some("password".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        actions: vec![ScreenElement {
            label: Some("Sign In".into()),
            kind: ElementKind::Action,
            tag: Some("button".into()),
            role: None,
            input_type: Some("submit".into()),
            required: false,
            placeholder: None,
            id: None,
            href: None,
            options: None,
        }],
        primary_action: None,
        intent: None,
    };
    let intent = infer_form_intent(&auth_form);
    assert_eq!(intent.label, "Authentication", "Password + Sign In â†’ Authentication");
    assert!(intent.confidence > 0.7, "Confidence should exceed 0.7");
}
