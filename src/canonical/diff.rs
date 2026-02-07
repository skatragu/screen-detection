use std::collections::BTreeSet;

use crate::{canonical::canonical_model::CanonicalScreenState, screen::screen_model::ElementKind};

#[derive(Debug, Clone)]
pub struct SemanticStateDiff {
    /// High-level form changes
    pub forms: FormDiff,

    /// Screen-level actions (outside forms)
    pub standalone_actions: ActionDiff,

    /// Screen outputs (results, messages, data)
    pub outputs: OutputDiff,

    /// Signals derived from change patterns
    pub signals: Vec<SemanticSignal>,
}

#[derive(Debug, Clone)]
pub struct FormDiff {
    pub added: Vec<String>,   // form IDs
    pub removed: Vec<String>, // form IDs
    pub changed: Vec<FormChange>,
}

#[derive(Debug, Clone)]
pub struct FormChange {
    pub form_id: String,

    pub inputs_added: Vec<String>, // element IDs
    pub inputs_removed: Vec<String>,

    pub actions_added: Vec<String>,
    pub actions_removed: Vec<String>,

    pub primary_action_changed: bool,
    pub intent_changed: bool,
}

#[derive(Debug, Clone)]
pub struct ActionDiff {
    pub added: Vec<String>,   // element IDs
    pub removed: Vec<String>, // element IDs
}

#[derive(Debug, Clone)]
pub struct OutputDiff {
    pub added: Vec<String>,   // element IDs
    pub removed: Vec<String>, // element IDs
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticSignal {
    ScreenLoaded,
    NavigationOccurred,
    FormSubmitted { form_id: String },
    ResultsAppeared,
    ErrorAppeared,
    NoOp, // action produced no visible change
}

fn diff_sets<T: Ord + Clone>(before: &[T], after: &[T]) -> (Vec<T>, Vec<T>) {
    let before_set: BTreeSet<_> = before.iter().cloned().collect();
    let after_set: BTreeSet<_> = after.iter().cloned().collect();

    let added = after_set.difference(&before_set).cloned().collect();

    let removed = before_set.difference(&after_set).cloned().collect();

    (added, removed)
}

fn diff_forms(before: &CanonicalScreenState, after: &CanonicalScreenState) -> FormDiff {
    let before_ids: BTreeSet<_> = before.forms.keys().cloned().collect();
    let after_ids: BTreeSet<_> = after.forms.keys().cloned().collect();

    let added = after_ids.difference(&before_ids).cloned().collect();

    let removed = before_ids.difference(&after_ids).cloned().collect();

    let mut changed = vec![];

    for form_id in before_ids.intersection(&after_ids) {
        let b = &before.forms[form_id];
        let a = &after.forms[form_id];

        let (inputs_added, inputs_removed) = diff_sets(&b.inputs, &a.inputs);

        let (actions_added, actions_removed) = diff_sets(&b.actions, &a.actions);

        let primary_action_changed = b.primary_action != a.primary_action;

        let intent_changed = b.intent != a.intent;

        if !inputs_added.is_empty()
            || !inputs_removed.is_empty()
            || !actions_added.is_empty()
            || !actions_removed.is_empty()
            || primary_action_changed
            || intent_changed
        {
            changed.push(FormChange {
                form_id: form_id.clone(),
                inputs_added,
                inputs_removed,
                actions_added,
                actions_removed,
                primary_action_changed,
                intent_changed,
            });
        }
    }

    FormDiff {
        added,
        removed,
        changed,
    }
}

fn diff_actions(before: &CanonicalScreenState, after: &CanonicalScreenState) -> ActionDiff {
    let (added, removed) = diff_sets(&before.standalone_actions, &after.standalone_actions);

    ActionDiff { added, removed }
}

fn diff_outputs(before: &CanonicalScreenState, after: &CanonicalScreenState) -> OutputDiff {
    let (added, removed) = diff_sets(&before.outputs, &after.outputs);

    OutputDiff { added, removed }
}

fn derive_signals(
    forms: &FormDiff,
    _actions: &ActionDiff,
    outputs: &OutputDiff,
    before: &CanonicalScreenState,
    after: &CanonicalScreenState,
    is_initial: bool,
) -> Vec<SemanticSignal> {
    let mut signals = vec![];

    // ---- Initial observation ----
    if is_initial {
        signals.push(SemanticSignal::ScreenLoaded);
        return signals;
    }

    let form_disappeared = !forms.removed.is_empty();
    let outputs_appeared = !outputs.added.is_empty();

    // ---- Navigation ----
    if form_disappeared && !outputs_appeared {
        signals.push(SemanticSignal::NavigationOccurred);
    }

    // ---- Form submission ----
    if form_disappeared && outputs_appeared {
        for form_id in &forms.removed {
            signals.push(SemanticSignal::FormSubmitted {
                form_id: form_id.clone(),
            });
        }
    }

    // ---- Terminal outcomes ----
    let error_added = outputs.added.iter().any(|id| output_is_error(id, after));

    if error_added {
        signals.push(SemanticSignal::ErrorAppeared);
    } else if outputs_appeared {
        signals.push(SemanticSignal::ResultsAppeared);
    }

    // ---- No-op ----
    if signals.is_empty() {
        signals.push(SemanticSignal::NoOp);
    }

    signals
}

// fn derive_signals(
//     forms: &FormDiff,
//     _actions: &ActionDiff,
//     outputs: &OutputDiff,
//     before: &CanonicalScreenState,
//     after: &CanonicalScreenState,
//     is_initial: bool,
// ) -> Vec<SemanticSignal> {
//     let mut signals = vec![];

//     // ---- Screen loaded (first observation only) ----
//     if is_initial {
//         signals.push(SemanticSignal::ScreenLoaded);
//         return signals; // important: isolate initial observation
//     }

//     let form_disappeared = !forms.removed.is_empty();
//     let outputs_appeared = !outputs.added.is_empty();

//     // ---- Navigation ----
//     let navigation = form_disappeared && outputs_appeared && !outputs.removed.is_empty();

//     if navigation {
//         signals.push(SemanticSignal::NavigationOccurred);
//     }

//     let terminal_output_appeared = outputs
//         .added
//         .iter()
//         .any(|id| output_is_error(id, after) || output_is_result(id, after));

//     // ---- Form submission ----
//     if form_disappeared && terminal_output_appeared {
//         for form_id in &forms.removed {
//             signals.push(SemanticSignal::FormSubmitted {
//                 form_id: form_id.clone(),
//             });
//         }
//     }

//     // ---- Terminal outcome detection ----
//     let error_added = outputs.added.iter().any(|id| output_is_error(id, after));
//     let error_removed = outputs.removed.iter().any(|id| output_is_error(id, before));

//     if error_added && !error_removed {
//         signals.push(SemanticSignal::ErrorAppeared);
//     } else if outputs_appeared && !navigation {
//         signals.push(SemanticSignal::ResultsAppeared);
//     }

//     // ---- No-op fallback ----
//     if signals.is_empty() {
//         signals.push(SemanticSignal::NoOp);
//     }

//     signals
// }

fn output_is_error(id: &str, state: &CanonicalScreenState) -> bool {
    let el = match state.elements.get(id) {
        Some(e) => e,
        None => return false,
    };

    // Only outputs can be errors
    if el.kind != ElementKind::Output {
        return false;
    }

    let text = el
        .label
        .as_ref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    text.contains("error")
        || text.contains("failed")
        || text.contains("invalid")
        || text.contains("unable")
        || text.contains("not found")
}

// fn output_is_result(output_id: &str, after: &CanonicalScreenState) -> bool {
//     !output_is_error(output_id, after)
// }

// fn output_is_result(output_id: &str, after: &CanonicalScreenState) -> bool {
//     after
//         .outputs
//         .iter()
//         .any(|o| o.id == output_id && !o.is_error && o.region == "Main")
// }

pub fn semantic_diff(
    before: &CanonicalScreenState,
    after: &CanonicalScreenState,
    is_initial: bool,
) -> SemanticStateDiff {
    let forms = diff_forms(before, after);
    let standalone_actions = diff_actions(before, after);
    let outputs = diff_outputs(before, after);

    let signals = derive_signals(
        &forms,
        &standalone_actions,
        &outputs,
        before,
        after,
        is_initial,
    );

    SemanticStateDiff {
        forms,
        standalone_actions,
        outputs,
        signals,
    }
}
