use std::collections::BTreeMap;

use crate::{
    screen::screen_model::{ElementKind, FormIntent, ScreenElement},
    state::{diff::StateDiff, state_model::ScreenState},
};

#[derive(Debug, Clone)]
pub struct CanonicalScreenState {
    pub url: String,
    pub title: String,

    /// All elements, indexed by stable identity
    pub elements: BTreeMap<String, CanonicalElement>,

    /// Forms indexed by form id
    pub forms: BTreeMap<String, CanonicalForm>,

    /// Screen-level actions (not part of any form)
    pub standalone_actions: Vec<String>, // element IDs

    /// Outputs visible on screen
    pub outputs: Vec<String>, // element IDs
}

impl CanonicalScreenState {
    pub fn empty() -> Self {
        CanonicalScreenState {
            url: String::new(),
            title: String::new(),
            forms: Default::default(),
            standalone_actions: vec![],
            outputs: vec![],
            elements: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalForm {
    pub id: String,

    pub inputs: Vec<String>,  // element IDs
    pub actions: Vec<String>, // element IDs

    pub primary_action: Option<String>, // element ID
    pub intent: Option<FormIntent>,
}

#[derive(Debug, Clone)]
pub struct CanonicalElement {
    pub id: String,
    pub kind: ElementKind,
    pub label: Option<String>,
    pub scope: String, // "screen" | "form:<id>"
}

pub fn canonicalize(
    state: &ScreenState,
    _diff: Option<&StateDiff>, // not always needed immediately, but passed intentionally
) -> CanonicalScreenState {
    let mut elements = BTreeMap::new();
    let mut forms = BTreeMap::new();

    // Register all elements by identity
    for (id, identified) in &state.identities {
        elements.insert(
            id.clone(),
            CanonicalElement {
                id: id.clone(),
                kind: identified.element.kind.clone(),
                label: identified.element.label.clone(),
                scope: identified.scope.clone(),
            },
        );
    }

    // Canonicalize forms
    for form in &state.forms {
        let mut input_ids = vec![];
        let mut action_ids = vec![];

        for input in &form.inputs {
            if let Some(id) = resolve_id(state, input) {
                input_ids.push(id);
            }
        }

        for action in &form.actions {
            if let Some(id) = resolve_id(state, action) {
                action_ids.push(id);
            }
        }

        let primary_action = form
            .primary_action
            .as_ref()
            .and_then(|a| resolve_id(state, a));

        forms.insert(
            form.id.clone(),
            CanonicalForm {
                id: form.id.clone(),
                inputs: input_ids,
                actions: action_ids,
                primary_action,
                intent: form.intent.clone(),
            },
        );
    }

    // Standalone actions
    let standalone_actions = state
        .standalone_actions
        .iter()
        .filter_map(|a| resolve_id(state, a))
        .collect();

    // Outputs
    let outputs = state
        .outputs
        .iter()
        .filter_map(|o| resolve_id(state, o))
        .collect();

    CanonicalScreenState {
        url: state.url.clone().unwrap_or(String::from("<unknown>")),
        title: state.title.clone(),
        elements,
        forms,
        standalone_actions,
        outputs,
    }
}

fn resolve_id(state: &ScreenState, el: &ScreenElement) -> Option<String> {
    state
        .identities
        .iter()
        .find(|(_, ident)| &ident.element == el)
        .map(|(id, _)| id.clone())
}
