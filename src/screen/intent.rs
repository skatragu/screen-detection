use crate::screen::screen_model::{Form, FormIntent, IntentSignal};

pub fn infer_form_intent(form: &Form) -> FormIntent {
    let mut signals = Vec::new();
    let mut score: f32 = 0.0;

    let mut text = String::new();

    for input in &form.inputs {
        if let Some(label) = &input.label {
            let l = label.to_lowercase();
            text.push_str(&l);
            text.push(' ');

            if l.contains("password") {
                score += 0.4;
                signals.push(IntentSignal::InputType("password".into()));
            }
        }
    }

    for action in &form.actions {
        if let Some(label) = &action.label {
            let l = label.to_lowercase();
            text.push_str(&l);
            text.push(' ');

            if l.contains("sign") || l.contains("login") {
                score += 0.4;
                signals.push(IntentSignal::ActionLabel(label.clone()));
            }
        }
    }

    let label = if score > 0.7 {
        "Authentication"
    } else if score > 0.4 {
        "User Input"
    } else {
        "Unknown"
    };

    let confidence = score.clamp(0.0, 1.0);

    FormIntent {
        label: label.to_string(),
        confidence,
        signals,
    }
}
