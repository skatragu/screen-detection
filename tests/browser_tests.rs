use screen_detection::{
    browser::{
        playwright::SelectorHint,
        session::{BrowserRequest, BrowserResponse},
    },
    screen::{
        classifier::classify,
        screen_model::DomElement,
    },
};

// =========================================================================
// Classifier field population
// =========================================================================

#[test]
fn classifier_populates_screen_element_fields() {
    let elements = vec![
        DomElement {
            tag: "input".into(),
            text: None,
            role: Some("textbox".into()),
            r#type: Some("text".into()),
            aria_label: Some("Search query".into()),
            disabled: false,
            required: false,
            form_id: Some("search".into()),
            id: None,
            name: None,
            placeholder: None,
            href: None,
            value: None,
            options: None,
        },
        DomElement {
            tag: "button".into(),
            text: Some("Submit".into()),
            role: Some("button".into()),
            r#type: Some("submit".into()),
            aria_label: None,
            disabled: false,
            required: false,
            form_id: Some("search".into()),
            id: None,
            name: None,
            placeholder: None,
            href: None,
            value: None,
            options: None,
        },
        DomElement {
            tag: "div".into(),
            text: Some("Results will appear here".into()),
            role: None,
            r#type: None,
            aria_label: None,
            disabled: false,
            required: false,
            form_id: None,
            id: None,
            name: None,
            placeholder: None,
            href: None,
            value: None,
            options: None,
        },
    ];

    let semantics = classify(&elements);

    // Check the input in the form
    let form = semantics.forms.iter().find(|f| f.id == "search").unwrap();
    let input = &form.inputs[0];
    assert_eq!(input.tag, Some("input".into()));
    assert_eq!(input.role, Some("textbox".into()));
    assert_eq!(input.input_type, Some("text".into()));
    assert_eq!(input.label, Some("Search query".into()));

    // Check the action in the form
    let action = &form.actions[0];
    assert_eq!(action.tag, Some("button".into()));
    assert_eq!(action.role, Some("button".into()));
    assert_eq!(action.input_type, Some("submit".into()));

    // Check the output
    let output = &semantics.outputs[0];
    assert_eq!(output.tag, Some("div".into()));
    assert_eq!(output.role, None);
    assert_eq!(output.input_type, None);
}

// =========================================================================
// SelectorHint serialization
// =========================================================================

#[test]
fn selector_hint_serializes_with_correct_field_names() {
    let hint = SelectorHint {
        role: Some("textbox".into()),
        name: Some("Email".into()),
        tag: Some("input".into()),
        input_type: Some("email".into()),
        form_id: Some("login".into()),
    };

    let json: serde_json::Value = serde_json::to_value(&hint).unwrap();

    // Verify serde renames: input_type -> "type", form_id -> "formId"
    assert_eq!(json["role"], "textbox");
    assert_eq!(json["name"], "Email");
    assert_eq!(json["tag"], "input");
    assert_eq!(json["type"], "email", "input_type must serialize as 'type'");
    assert_eq!(json["formId"], "login", "form_id must serialize as 'formId'");

    // Verify the Rust field names are NOT in the JSON
    assert!(json.get("input_type").is_none(), "Must not contain 'input_type' key");
    assert!(json.get("form_id").is_none(), "Must not contain 'form_id' key");
}

#[test]
fn selector_hint_skips_none_fields() {
    let hint = SelectorHint {
        role: None,
        name: Some("Click me".into()),
        tag: Some("a".into()),
        input_type: None,
        form_id: None,
    };

    let json_str = serde_json::to_string(&hint).unwrap();

    assert!(!json_str.contains("role"), "None fields must be skipped");
    assert!(!json_str.contains("type"), "None fields must be skipped");
    assert!(!json_str.contains("formId"), "None fields must be skipped");
    assert!(json_str.contains("name"), "Present fields must appear");
    assert!(json_str.contains("tag"), "Present fields must appear");
}

// =========================================================================
// BrowserRequest serialization
// =========================================================================

#[test]
fn browser_request_navigate_serializes_correctly() {
    let req = BrowserRequest::navigate("https://example.com");
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "navigate");
    assert_eq!(json["url"], "https://example.com");
}

#[test]
fn browser_request_extract_serializes_correctly() {
    let req = BrowserRequest::extract();
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "extract");
    assert!(json.get("url").is_none());
}

#[test]
fn browser_request_fill_serializes_correctly() {
    let selector = SelectorHint {
        role: Some("textbox".into()),
        name: Some("Email".into()),
        tag: Some("input".into()),
        input_type: Some("email".into()),
        form_id: Some("login".into()),
    };
    let req = BrowserRequest::fill(&selector, "test@example.com");
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "action");
    assert_eq!(json["action"], "fill");
    assert_eq!(json["value"], "test@example.com");
    assert_eq!(json["selector"]["role"], "textbox");
    assert_eq!(json["selector"]["name"], "Email");
    assert_eq!(json["selector"]["type"], "email", "input_type serializes as 'type'");
    assert_eq!(json["selector"]["formId"], "login", "form_id serializes as 'formId'");
    assert!(json.get("duration_ms").is_none());
}

#[test]
fn browser_request_click_serializes_correctly() {
    let selector = SelectorHint {
        role: Some("button".into()),
        name: Some("Submit".into()),
        tag: None,
        input_type: None,
        form_id: None,
    };
    let req = BrowserRequest::click(&selector);
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "action");
    assert_eq!(json["action"], "click");
    assert!(json.get("value").is_none(), "click has no value");
    assert_eq!(json["selector"]["role"], "button");
    assert_eq!(json["selector"]["name"], "Submit");
}

#[test]
fn browser_request_wait_serializes_correctly() {
    let req = BrowserRequest::wait(3000);
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "action");
    assert_eq!(json["action"], "wait");
    assert_eq!(json["duration_ms"], 3000);
    assert!(json.get("selector").is_none(), "wait has no selector");
    assert!(json.get("value").is_none(), "wait has no value");
}

#[test]
fn browser_request_screenshot_serializes_correctly() {
    let req = BrowserRequest::screenshot("/tmp/test.png");
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "screenshot");
    assert_eq!(json["path"], "/tmp/test.png");
}

#[test]
fn browser_request_current_url_serializes_correctly() {
    let req = BrowserRequest::current_url();
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "current_url");
}

#[test]
fn browser_request_quit_serializes_correctly() {
    let req = BrowserRequest::quit();
    let json: serde_json::Value = serde_json::to_value(&req).unwrap();

    assert_eq!(json["cmd"], "quit");
}

#[test]
fn test_browser_request_query_text_json() {
    let req = BrowserRequest::query_text("#heading");
    let json = serde_json::to_string(&req).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(v["cmd"], "query_text");
    assert_eq!(v["selector"], "#heading");
}

#[test]
fn test_browser_request_query_visible_json() {
    let req = BrowserRequest::query_visible(".banner");
    let json = serde_json::to_string(&req).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(v["cmd"], "query_visible");
    assert_eq!(v["selector"], ".banner");
}

#[test]
fn test_browser_request_query_count_json() {
    let req = BrowserRequest::query_count("tr");
    let json = serde_json::to_string(&req).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(v["cmd"], "query_count");
    assert_eq!(v["selector"], "tr");
}

// =========================================================================
// BrowserResponse deserialization
// =========================================================================

#[test]
fn browser_response_deserializes_success() {
    let json = r#"{"ok":true}"#;
    let resp: BrowserResponse = serde_json::from_str(json).unwrap();
    assert!(resp.ok);
    assert!(resp.error.is_none());
    assert!(resp.data.is_none());
    assert!(resp.url.is_none());
    assert!(resp.ready.is_none());
}

#[test]
fn browser_response_deserializes_error() {
    let json = r#"{"ok":false,"error":"Element not found"}"#;
    let resp: BrowserResponse = serde_json::from_str(json).unwrap();
    assert!(!resp.ok);
    assert_eq!(resp.error.as_deref(), Some("Element not found"));
}

#[test]
fn browser_response_deserializes_with_data() {
    let json = r#"{"ok":true,"data":{"title":"Test","url":"https://example.com","dom":[]}}"#;
    let resp: BrowserResponse = serde_json::from_str(json).unwrap();
    assert!(resp.ok);
    let data = resp.data.unwrap();
    assert_eq!(data["title"], "Test");
    assert_eq!(data["url"], "https://example.com");
    assert!(data["dom"].as_array().unwrap().is_empty());
}

#[test]
fn browser_response_deserializes_with_url() {
    let json = r#"{"ok":true,"url":"https://example.com/page"}"#;
    let resp: BrowserResponse = serde_json::from_str(json).unwrap();
    assert!(resp.ok);
    assert_eq!(resp.url.as_deref(), Some("https://example.com/page"));
}

#[test]
fn browser_response_deserializes_ready_signal() {
    let json = r#"{"ok":true,"ready":true}"#;
    let resp: BrowserResponse = serde_json::from_str(json).unwrap();
    assert!(resp.ok);
    assert_eq!(resp.ready, Some(true));
}

#[test]
fn test_browser_response_with_text() {
    let json = r#"{"ok":true,"text":"Hello World"}"#;
    let resp: BrowserResponse = serde_json::from_str(json).expect("parse");
    assert!(resp.ok);
    assert_eq!(resp.text, Some("Hello World".into()));
    assert_eq!(resp.visible, None);
    assert_eq!(resp.count, None);
}

#[test]
fn test_browser_response_with_visible() {
    let json = r#"{"ok":true,"visible":true}"#;
    let resp: BrowserResponse = serde_json::from_str(json).expect("parse");
    assert!(resp.ok);
    assert_eq!(resp.visible, Some(true));
    assert_eq!(resp.text, None);
    assert_eq!(resp.count, None);
}

#[test]
fn test_browser_response_with_visible_false() {
    let json = r#"{"ok":true,"visible":false}"#;
    let resp: BrowserResponse = serde_json::from_str(json).expect("parse");
    assert!(resp.ok);
    assert_eq!(resp.visible, Some(false));
}

#[test]
fn test_browser_response_with_count() {
    let json = r#"{"ok":true,"count":5}"#;
    let resp: BrowserResponse = serde_json::from_str(json).expect("parse");
    assert!(resp.ok);
    assert_eq!(resp.count, Some(5));
    assert_eq!(resp.text, None);
    assert_eq!(resp.visible, None);
}

#[test]
fn test_browser_response_with_text_null() {
    let json = r#"{"ok":true,"text":null}"#;
    let resp: BrowserResponse = serde_json::from_str(json).expect("parse");
    assert!(resp.ok);
    assert_eq!(resp.text, None);
}

// =========================================================================
// New: BrowserRequest select/check/uncheck serialization
// =========================================================================

#[test]
fn browser_request_select_option_serialization() {
    let selector = SelectorHint {
        role: None,
        name: Some("Country".into()),
        tag: Some("select".into()),
        input_type: None,
        form_id: Some("address".into()),
    };
    let request = BrowserRequest::select_option(&selector, "us");
    let json: serde_json::Value = serde_json::to_value(&request).unwrap();

    assert_eq!(json["cmd"], "action");
    assert_eq!(json["action"], "select");
    assert_eq!(json["value"], "us");
    assert_eq!(json["selector"]["name"], "Country");
    assert_eq!(json["selector"]["tag"], "select");
}

#[test]
fn browser_request_check_serialization() {
    let selector = SelectorHint {
        role: Some("checkbox".into()),
        name: Some("Terms".into()),
        tag: Some("input".into()),
        input_type: Some("checkbox".into()),
        form_id: None,
    };
    let request = BrowserRequest::check(&selector);
    let json: serde_json::Value = serde_json::to_value(&request).unwrap();

    assert_eq!(json["cmd"], "action");
    assert_eq!(json["action"], "check");
    assert!(json.get("value").is_none() || json["value"].is_null());
}

#[test]
fn browser_request_uncheck_serialization() {
    let selector = SelectorHint {
        role: Some("checkbox".into()),
        name: Some("Newsletter".into()),
        tag: Some("input".into()),
        input_type: Some("checkbox".into()),
        form_id: None,
    };
    let request = BrowserRequest::uncheck(&selector);
    let json: serde_json::Value = serde_json::to_value(&request).unwrap();

    assert_eq!(json["cmd"], "action");
    assert_eq!(json["action"], "uncheck");
}

// =========================================================================
// New: DomElement with new fields deserialization
// =========================================================================

#[test]
fn dom_element_new_fields_deserialization() {
    let json = r#"{
        "tag": "select",
        "text": null,
        "role": null,
        "type": null,
        "ariaLabel": "Country",
        "disabled": false,
        "required": true,
        "formId": "address",
        "id": "country-select",
        "name": "country",
        "placeholder": null,
        "href": null,
        "value": "us",
        "options": [
            {"value": "us", "text": "United States"},
            {"value": "ca", "text": "Canada"}
        ]
    }"#;
    let el: DomElement = serde_json::from_str(json).unwrap();
    assert_eq!(el.tag, "select");
    assert_eq!(el.id, Some("country-select".into()));
    assert_eq!(el.name, Some("country".into()));
    assert_eq!(el.value, Some("us".into()));
    assert!(el.required);
    let opts = el.options.unwrap();
    assert_eq!(opts.len(), 2);
    assert_eq!(opts[0].value, "us");
    assert_eq!(opts[0].text, "United States");
}

#[test]
fn select_option_serde_roundtrip() {
    use screen_detection::screen::screen_model::SelectOption;

    let opt = SelectOption {
        value: "val1".into(),
        text: "Option One".into(),
    };
    let json = serde_json::to_string(&opt).unwrap();
    let back: SelectOption = serde_json::from_str(&json).unwrap();
    assert_eq!(opt, back);
}
