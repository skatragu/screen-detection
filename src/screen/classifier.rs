use crate::screen::intent::infer_form_intent;
use crate::screen::screen_model::{
    DomElement, ElementKind, Form, FormIntent, ScreenElement, ScreenSemantics,
};

use std::collections::HashMap;

pub fn classify(elements: &[DomElement]) -> ScreenSemantics {
    let mut forms: HashMap<String, Form> = HashMap::new();
    let mut standalone_actions = Vec::new();
    let mut outputs = Vec::new();

    for el in elements {
        if is_output(el) {
            outputs.push(to_output(el));
            continue;
        }

        if let Some(form_id) = &el.form_id {
            let form = forms.entry(form_id.clone()).or_insert(Form {
                id: form_id.clone(),
                inputs: vec![],
                actions: vec![],
                primary_action: None,
                intent: Some(FormIntent {
                    label: "Unknown".to_string(),
                    confidence: 0.0,
                    signals: vec![],
                }),
            });

            if is_input(el) {
                form.inputs.push(to_input(el));
            } else if is_action(el) {
                form.actions.push(to_action(el));
            }
        } else if is_action(el) {
            standalone_actions.push(to_action(el));
        }
    }

    // Detect primary action per form
    for form in forms.values_mut() {
        form.primary_action = detect_primary_action(&form.actions);
        form.intent = Some(infer_form_intent(form));
    }

    let primary_action = detect_primary_action(
        &forms
            .values()
            .flat_map(|f| f.actions.clone())
            .chain(standalone_actions.clone())
            .collect::<Vec<_>>(),
    );

    ScreenSemantics {
        forms: forms.into_values().collect(),
        standalone_actions,
        outputs,
        primary_action,
    }
}

fn detect_primary_action(actions: &[ScreenElement]) -> Option<ScreenElement> {
    let keywords = ["submit", "save", "sign", "login", "continue", "next"];

    actions
        .iter()
        .find(|a| {
            a.kind == ElementKind::Action
                && a.label
                    .as_ref()
                    .map(|l| {
                        let lower = l.to_lowercase();
                        keywords.iter().any(|k| lower.contains(k))
                    })
                    .unwrap_or(false)
        })
        .cloned()
}

fn is_input(el: &DomElement) -> bool {
    if el.tag != "input" && el.tag != "textarea" && el.tag != "select" {
        return false;
    }

    match el.r#type.as_deref() {
        // Textual inputs
        None
        | Some("text")
        | Some("email")
        | Some("password")
        | Some("search")
        | Some("number")
        | Some("tel")
        | Some("url")
        | Some("date")
        | Some("time")
        | Some("month")
        | Some("week")
        | Some("range")

        // Choice inputs
        | Some("radio")
        | Some("checkbox") => true,

        // Explicit non-inputs
        Some("submit")
        | Some("button")
        | Some("reset")
        | Some("image")
        | Some("hidden")
        | Some("file") => false,

        // Unknown → be conservative
        _ => false,
    }
}

fn is_action(el: &DomElement) -> bool {
    if el.disabled {
        return false;
    }

    matches!(el.tag.as_str(), "button" | "a")
        || el.role.as_deref() == Some("button")
        || el.r#type.as_deref() == Some("submit")
}

fn is_output(el: &DomElement) -> bool {
    // Outputs must be non-interactive
    if is_action(el) || is_input(el) {
        return false;
    }

    // Exclude form chrome
    if matches!(el.tag.as_str(), "label" | "legend" | "option") {
        return false;
    }

    if let Some(text) = &el.text {
        let trimmed = text.trim();
        trimmed.len() > 2
    } else {
        false
    }
}

fn label_for(el: &DomElement) -> Option<String> {
    el.aria_label
        .clone()
        .or_else(|| el.text.clone())
        .map(|s| s.trim().to_string())
}

fn to_input(el: &DomElement) -> ScreenElement {
    ScreenElement {
        label: label_for(el),
        kind: ElementKind::Input,
    }
}

fn to_action(el: &DomElement) -> ScreenElement {
    ScreenElement {
        label: label_for(el),
        kind: ElementKind::Action,
    }
}

fn to_output(el: &DomElement) -> ScreenElement {
    ScreenElement {
        label: label_for(el),
        kind: ElementKind::Output,
    }
}
