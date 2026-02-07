use crate::state::state_model::ScreenState;

use std::collections::HashSet;

use super::identity::IdentifiedElement;

#[derive(Debug)]
pub struct StateDiff {
    pub added: Vec<IdentifiedElement>,
    pub removed: Vec<IdentifiedElement>,
    pub unchanged: Vec<IdentifiedElement>,
}

pub fn diff(before: &ScreenState, after: &ScreenState) -> StateDiff {
    let before_ids: HashSet<_> = before.identities.keys().cloned().collect();
    let after_ids: HashSet<_> = after.identities.keys().cloned().collect();

    let added = after_ids
        .difference(&before_ids)
        .filter_map(|id| after.identities.get(id).cloned())
        .collect();

    let removed = before_ids
        .difference(&after_ids)
        .filter_map(|id| before.identities.get(id).cloned())
        .collect();

    let unchanged = before_ids
        .intersection(&after_ids)
        .filter_map(|id| after.identities.get(id).cloned())
        .collect();

    StateDiff {
        added,
        removed,
        unchanged,
    }
}
