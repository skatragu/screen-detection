use crate::state::state_model::ScreenState;
use crate::{screen::screen_model::ScreenSemantics, state::identity::IdentifiedElement};
use std::collections::HashMap;

use crate::screen::screen_model::{ElementKind, Form, OutputRegion, ScreenElement, Volatility};
use crate::state::normalize::{infer_output_region, normalize_output_text, text_fingerprint};

pub fn build_state(url: Option<&str>, title: &str, semantics: ScreenSemantics) -> ScreenState {
    let identities = resolve_identities(&semantics.forms, &semantics.outputs);

    ScreenState {
        url: Some(url.unwrap_or("<unknown>").to_string()),
        title: title.to_string(),
        forms: semantics.forms,
        standalone_actions: semantics.standalone_actions,
        outputs: semantics.outputs,
        identities: identities,
    }
}

/// Public entry point
pub fn resolve_identities(
    forms: &[Form],
    outputs: &[ScreenElement],
) -> HashMap<String, IdentifiedElement> {
    let mut map = HashMap::new();
    let mut region_counters: HashMap<OutputRegion, usize> = HashMap::new();

    for form in forms {
        let scope = format!("form:{}", form.id);

        for el in form.inputs.iter().chain(form.actions.iter()) {
            let id = element_identity(el, &scope);
            map.insert(
                id.clone(),
                IdentifiedElement {
                    id,
                    element: el.clone(),
                    scope: scope.clone(),
                    region: OutputRegion::Main,
                    volatility: Volatility::Stable,
                },
            );
        }
    }

    for el in outputs {
        let scope = "screen".to_string();
        let region = infer_output_region(el);

        let id = match normalize_output_text(el.label.as_deref().unwrap_or("")) {
            Some(text) => {
                format!("screen:output:{:?}:{}", region, text_fingerprint(&text))
            }

            None => {
                // Fallback: region + positional index
                let idx = region_counters
                    .entry(region.clone())
                    .and_modify(|v| *v += 1)
                    .or_insert(0);

                format!("screen:output:{:?}:idx:{}", region, idx)
            }
        };

        map.insert(
            id.clone(),
            IdentifiedElement {
                id,
                element: el.clone(),
                scope,
                region: region,
                volatility: Volatility::Volatile,
            },
        );
    }

    map
}

fn element_identity(el: &ScreenElement, scope: &str) -> String {
    let kind = match el.kind {
        ElementKind::Input => "input",
        ElementKind::Action => "action",
        ElementKind::Output => "output",
    };

    let label = el
        .label
        .as_deref()
        .unwrap_or("unknown")
        .to_lowercase()
        .replace(' ', "_");

    format!("{scope}:{kind}:{label}")
}
