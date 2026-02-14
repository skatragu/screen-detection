use std::collections::{HashMap, VecDeque};

use crate::agent::error::AgentError;
use crate::agent::page_analyzer::{MockPageAnalyzer, PageAnalyzer};
use crate::agent::page_model::FormModel;
use crate::browser::playwright::SelectorHint;
use crate::browser::session::BrowserSession;
use crate::state::state_model::ScreenState;

use super::app_map::{AppMap, ExplorerConfig, PageNode, Transition, TransitionKind};

// ============================================================================
// Offline exploration (unit-testable, no browser needed)
// ============================================================================

/// Explore a single page from a pre-built `ScreenState`.
///
/// Takes a `PageAnalyzer` implementation to classify the page and produce
/// a `PageModel`. No browser session required — used in unit tests and
/// when a `ScreenState` is already available.
pub fn explore_with_analyzer(
    config: &ExplorerConfig,
    analyzer: &dyn PageAnalyzer,
    screen: &ScreenState,
) -> Result<AppMap, AgentError> {
    let mut app_map = AppMap::new();
    let model = analyzer.analyze(screen)?;

    let node = PageNode {
        url: screen
            .url
            .clone()
            .unwrap_or_else(|| config.start_url.clone()),
        title: screen.title.clone(),
        depth: 0,
        page_model: model,
    };
    app_map.add_page(node);

    Ok(app_map)
}

/// Convenience: single-page explore with `MockPageAnalyzer` (for testing).
pub fn explore(config: &ExplorerConfig, screen: &ScreenState) -> Result<AppMap, AgentError> {
    explore_with_analyzer(config, &MockPageAnalyzer, screen)
}

// ============================================================================
// Live multi-page BFS exploration (requires BrowserSession)
// ============================================================================

/// Multi-page BFS exploration using a live `BrowserSession`.
///
/// Starting from `config.start_url`, navigates to each page, extracts DOM,
/// analyzes with `PageAnalyzer`, discovers links from `NavigationTargets`,
/// and follows them up to `max_pages` / `max_depth`.
///
/// Uses `snapshot_session()` from `crate` to extract and classify each page.
pub fn explore_live(
    config: &ExplorerConfig,
    session: &mut BrowserSession,
    analyzer: &dyn PageAnalyzer,
) -> Result<AppMap, Box<dyn std::error::Error>> {
    let mut app_map = AppMap::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    queue.push_back((config.start_url.clone(), 0));

    while let Some((url, depth)) = queue.pop_front() {
        // Respect page limit
        if app_map.page_count() >= config.max_pages {
            break;
        }

        // Respect depth limit
        if depth > config.max_depth {
            continue;
        }

        // Skip already-visited pages
        if app_map.has_page(&url) {
            continue;
        }

        // Same-origin check
        if config.same_origin_only && !is_same_origin(&config.start_url, &url) {
            continue;
        }

        // Navigate and snapshot
        session.navigate(&url)?;
        let (screen_state, _canonical) = crate::snapshot_session(session)?;

        // Analyze page
        let model = analyzer.analyze(&screen_state)?;

        // Queue discovered navigation targets
        for target in &model.navigation_targets {
            if let Some(resolved) = resolve_url(&url, &target.label) {
                app_map.add_transition(Transition {
                    from_url: url.clone(),
                    to_url: resolved.clone(),
                    label: target.label.clone(),
                    kind: TransitionKind::Link,
                });
                if !app_map.has_page(&resolved) {
                    queue.push_back((resolved, depth + 1));
                }
            }
        }

        // --- Form-aware exploration ---
        if config.explore_forms {
            let forms_to_explore: Vec<_> = model
                .forms
                .iter()
                .take(config.max_forms_per_page)
                .filter(|f| !f.fields.is_empty())
                .cloned()
                .collect();

            for form in &forms_to_explore {
                if app_map.page_count() >= config.max_pages {
                    break;
                }

                // Navigate back (form submit may have changed the page)
                session.navigate(&url)?;

                // Build values from suggested test values
                let values: HashMap<String, String> = form
                    .fields
                    .iter()
                    .map(|f| (f.label.clone(), f.suggested_test_value.clone()))
                    .collect();

                // Fill and submit — graceful: skip on failure
                if submit_form_in_session(session, form).is_ok() {
                    let result_url = session
                        .current_url()
                        .unwrap_or_else(|_| url.clone());

                    if let Ok((result_screen, _canonical)) = crate::snapshot_session(session) {
                        let result_page_url = result_screen
                            .url
                            .clone()
                            .unwrap_or_else(|| result_url.clone());

                        // Record FormSubmission transition
                        app_map.add_transition(Transition {
                            from_url: url.clone(),
                            to_url: result_page_url.clone(),
                            label: form
                                .submit_label
                                .clone()
                                .unwrap_or_else(|| "submit".into()),
                            kind: TransitionKind::FormSubmission {
                                form_id: form.form_id.clone(),
                                values,
                            },
                        });

                        // Add result page if new
                        if !app_map.has_page(&result_page_url) {
                            let result_model = analyzer.analyze(&result_screen)?;
                            let result_node = PageNode {
                                url: result_page_url.clone(),
                                title: result_screen.title.clone(),
                                depth: depth + 1,
                                page_model: result_model,
                            };
                            app_map.add_page(result_node);
                            queue.push_back((result_page_url, depth + 1));
                        }
                    }
                }
            }
        }

        // Store page
        let node = PageNode {
            url: screen_state.url.clone().unwrap_or_else(|| url.clone()),
            title: screen_state.title.clone(),
            depth,
            page_model: model,
        };
        app_map.add_page(node);
    }

    Ok(app_map)
}

// ============================================================================
// Form submission helper
// ============================================================================

/// Fill and submit a form via BrowserSession.
///
/// Fills each field using SelectorHint targeting, then clicks the submit button.
/// Returns Ok(()) on success, Err on browser action failure.
fn submit_form_in_session(
    session: &mut BrowserSession,
    form: &FormModel,
) -> Result<(), AgentError> {
    for field in &form.fields {
        let selector = SelectorHint {
            role: Some("textbox".into()),
            name: Some(field.label.clone()),
            tag: Some("input".into()),
            input_type: None,
            form_id: Some(form.form_id.clone()),
        };
        session.fill(&selector, &field.suggested_test_value)?;
    }

    if let Some(label) = &form.submit_label {
        let selector = SelectorHint {
            role: Some("button".into()),
            name: Some(label.clone()),
            tag: None,
            input_type: None,
            form_id: None,
        };
        session.click(&selector)?;
    }

    session.wait_idle(500)?;
    Ok(())
}

// ============================================================================
// URL utilities
// ============================================================================

/// Check if two URLs share the same origin (scheme + host).
pub fn is_same_origin(base: &str, candidate: &str) -> bool {
    match (extract_origin(base), extract_origin(candidate)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Extract the origin (scheme + host) from a URL.
fn extract_origin(url: &str) -> Option<&str> {
    let after_scheme = url.find("://").map(|i| i + 3)?;
    let end = url[after_scheme..]
        .find('/')
        .map(|i| after_scheme + i)
        .unwrap_or(url.len());
    Some(&url[..end])
}

/// Attempt to resolve a navigation target label into a URL.
///
/// Currently only handles absolute URLs (http:// or https://).
/// Future: could use href attributes from DOM extraction for relative URLs.
pub fn resolve_url(_base: &str, label: &str) -> Option<String> {
    if label.starts_with("http://") || label.starts_with("https://") {
        Some(label.to_string())
    } else {
        None
    }
}
