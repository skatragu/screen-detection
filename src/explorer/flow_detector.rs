use std::collections::{HashMap, HashSet};

use super::app_map::{AppMap, Flow, FlowStep, Transition, TransitionKind};

/// Detect multi-step flows from the AppMap transition graph.
///
/// A flow starts at a page with a FormSubmission transition and follows
/// the chain of form submissions through subsequent pages.
/// Only starts from "origin" pages — pages that have outgoing FormSubmission
/// transitions but are NOT the target of another FormSubmission transition.
pub fn detect_flows(app_map: &AppMap) -> Vec<Flow> {
    let mut flows = Vec::new();

    // Build adjacency: from_url -> Vec<&Transition> for FormSubmission transitions
    let mut form_edges: HashMap<&str, Vec<&Transition>> = HashMap::new();
    for t in &app_map.transitions {
        if matches!(&t.kind, TransitionKind::FormSubmission { .. }) {
            form_edges.entry(t.from_url.as_str()).or_default().push(t);
        }
    }

    // Find pages that are targets of form submissions (they are mid-flow, not origins)
    let targets_of_form: HashSet<&str> = app_map
        .transitions
        .iter()
        .filter(|t| matches!(&t.kind, TransitionKind::FormSubmission { .. }))
        .map(|t| t.to_url.as_str())
        .collect();

    // Walk from each origin page
    for start_url in form_edges.keys() {
        if targets_of_form.contains(*start_url) {
            continue; // Mid-flow page, not an origin
        }

        for transition in &form_edges[start_url] {
            let mut steps = Vec::new();
            let mut current = *transition;
            let mut visited = HashSet::new();
            visited.insert(*start_url);

            // Build the chain
            loop {
                steps.push(build_flow_step(current));
                visited.insert(current.to_url.as_str());

                // Try to continue the chain from to_url
                match form_edges.get(current.to_url.as_str()) {
                    Some(next_transitions) => {
                        match next_transitions
                            .iter()
                            .find(|t| !visited.contains(t.to_url.as_str()))
                        {
                            Some(next) => current = next,
                            None => break,
                        }
                    }
                    None => break,
                }
            }

            if !steps.is_empty() {
                let name = name_flow(app_map, start_url, &steps);
                let mut full_steps =
                    vec![FlowStep::Navigate {
                        url: start_url.to_string(),
                    }];
                full_steps.extend(steps);
                flows.push(Flow {
                    name,
                    steps: full_steps,
                });
            }
        }
    }

    // Sort by name for deterministic output
    flows.sort_by(|a, b| a.name.cmp(&b.name));
    flows
}

fn build_flow_step(transition: &Transition) -> FlowStep {
    match &transition.kind {
        TransitionKind::FormSubmission { form_id, values } => FlowStep::FillAndSubmit {
            url: transition.from_url.clone(),
            form_id: form_id.clone(),
            values: values.clone(),
            submit_label: Some(transition.label.clone()),
        },
        TransitionKind::Link => FlowStep::Navigate {
            url: transition.to_url.clone(),
        },
    }
}

fn name_flow(app_map: &AppMap, start_url: &str, steps: &[FlowStep]) -> String {
    let start_category = app_map
        .pages
        .get(start_url)
        .map(|p| format!("{:?}", p.page_model.category))
        .unwrap_or_else(|| "Unknown".into());

    let last_url = steps.last().and_then(|s| match s {
        FlowStep::Navigate { url } => Some(url.as_str()),
        FlowStep::FillAndSubmit { url, .. } => Some(url.as_str()),
    });

    let end_category = last_url
        .and_then(|u| app_map.pages.get(u))
        .map(|p| format!("{:?}", p.page_model.category))
        .unwrap_or_else(|| "Unknown".into());

    format!("Flow: {} -> {}", start_category, end_category)
}
