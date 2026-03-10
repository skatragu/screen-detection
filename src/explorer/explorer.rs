use std::collections::{HashMap, VecDeque};

use crate::agent::app_context::AppContext;
use crate::agent::data_generator::DataGenerator;
use crate::agent::error::AgentError;
use crate::agent::page_analyzer::{MockPageAnalyzer, PageAnalyzer};
use crate::agent::page_model::{FieldAnalysis, FieldType, FormModel};
use crate::browser::playwright::SelectorHint;
use crate::browser::session::BrowserSession;
use crate::cli::config::{AuthConfig, ExclusionConfig, ValueConfig};
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
/// Optional parameters:
/// - `auth`: if provided and has credentials, performs auto-login before BFS
/// - `exclusions`: URL patterns to skip + extra URLs to seed into the queue
/// - `value_overrides`: custom field values overriding suggested test values
///
/// Uses `snapshot_session()` from `crate` to extract and classify each page.
pub fn explore_live(
    config: &ExplorerConfig,
    session: &mut BrowserSession,
    analyzer: &dyn PageAnalyzer,
    auth: Option<&AuthConfig>,
    exclusions: Option<&ExclusionConfig>,
    value_overrides: Option<&ValueConfig>,
) -> Result<AppMap, Box<dyn std::error::Error>> {
    // Auto-login before BFS if credentials are configured
    if let Some(auth_cfg) = auth {
        if auth_cfg.has_credentials() {
            perform_login(session, auth_cfg, analyzer)?;
        }
    }
    let mut app_map = AppMap::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    // AppContext grows as pages are visited — provides cross-page data recall + domain tracking
    let mut app_context = AppContext::new();

    queue.push_back((config.start_url.clone(), 0));

    // Seed any force-include URLs into the initial queue
    if let Some(excl) = exclusions {
        for inc_url in &excl.include_urls {
            queue.push_back((inc_url.clone(), 0));
        }
    }

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

        // Skip excluded URLs
        if let Some(excl) = exclusions {
            if excl.should_skip(&url) {
                continue;
            }
        }

        // Navigate and snapshot
        session.navigate(&url)?;
        let (screen_state, _canonical) = crate::snapshot_session(session)?;

        // Analyze page — pass accumulated context for domain-aware LLM prompts
        let model = analyzer.analyze_with_context(&screen_state, &app_context)?;
        // Update application domain from this page's LLM analysis
        app_context.update_domain(Some(model.domain.clone()));

        // Queue discovered navigation targets
        for target in &model.navigation_targets {
            let candidate = target.href.as_deref().unwrap_or(&target.label);
            if let Some(resolved) = resolve_url(&url, candidate) {
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

                // Build field_analyses lookup: lowercase_label → FieldAnalysis
                let field_analyses: HashMap<String, &FieldAnalysis> = model
                    .field_analyses
                    .iter()
                    .map(|fa| (fa.label.to_lowercase(), fa))
                    .collect();

                // DataGenerator: cross-page recall → LLM suggestion → guess_value fallback
                let generator = DataGenerator::new(&app_context);

                // Build values: user overrides take highest priority, then DataGenerator
                let values: HashMap<String, String> = form
                    .fields
                    .iter()
                    .map(|f| {
                        let analysis = field_analyses.get(&f.label.to_lowercase()).copied();
                        let value = value_overrides
                            .and_then(|v| v.resolve(&f.label, Some(&model.domain)))
                            .cloned()
                            .unwrap_or_else(|| generator.generate(f, analysis));
                        (f.label.clone(), value)
                    })
                    .collect();

                // Fill and submit using the computed values — graceful: skip on failure
                if submit_form_in_session(session, form, &values).is_ok() {
                    let result_url = session
                        .current_url()
                        .unwrap_or_else(|_| url.clone());

                    if let Ok((result_screen, _canonical)) = crate::snapshot_session(session) {
                        let result_page_url = result_screen
                            .url
                            .clone()
                            .unwrap_or_else(|| result_url.clone());

                        // Update AppContext with what was entered on this page
                        app_context.record_page(&url, &model, values.clone());

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

                        // Add result page if new — use context-aware analysis
                        if !app_map.has_page(&result_page_url) {
                            let result_model = analyzer.analyze_with_context(&result_screen, &app_context)?;
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

/// Build a SelectorHint appropriate for a field's type.
///
/// Maps FieldType to the correct ARIA role, HTML tag, and input_type so
/// Playwright can locate the element correctly.
pub fn build_selector_for_field(field: &crate::agent::page_model::FieldModel, form_id: &str) -> SelectorHint {
    let (role, tag, input_type) = match field.field_type {
        FieldType::Select => (Some("combobox".into()), Some("select".into()), None),
        FieldType::Textarea => (Some("textbox".into()), Some("textarea".into()), None),
        FieldType::Checkbox => (Some("checkbox".into()), Some("input".into()), Some("checkbox".into())),
        FieldType::Radio => (Some("radio".into()), Some("input".into()), Some("radio".into())),
        FieldType::Email => (Some("textbox".into()), Some("input".into()), Some("email".into())),
        FieldType::Password => (Some("textbox".into()), Some("input".into()), Some("password".into())),
        FieldType::Number => (Some("spinbutton".into()), Some("input".into()), Some("number".into())),
        FieldType::Tel => (Some("textbox".into()), Some("input".into()), Some("tel".into())),
        FieldType::Search => (Some("searchbox".into()), Some("input".into()), Some("search".into())),
        FieldType::Date => (None, Some("input".into()), Some("date".into())),
        FieldType::Time => (None, Some("input".into()), Some("time".into())),
        FieldType::Url => (Some("textbox".into()), Some("input".into()), Some("url".into())),
        _ => (Some("textbox".into()), Some("input".into()), None),
    };

    SelectorHint {
        role,
        name: Some(field.label.clone()),
        tag,
        input_type,
        form_id: Some(form_id.to_string()),
    }
}

/// Fill and submit a form via BrowserSession using pre-computed field values.
///
/// `values` is a label→value map produced by `DataGenerator` (or value_overrides).
/// Falls back to `field.suggested_test_value` when a field's label is not in `values`.
/// Fills each field using field-type-aware SelectorHint targeting, then clicks
/// the submit button. Skips hidden fields. Returns Ok(()) on success, Err on
/// browser action failure.
fn submit_form_in_session(
    session: &mut BrowserSession,
    form: &FormModel,
    values: &HashMap<String, String>,
) -> Result<(), AgentError> {
    for field in &form.fields {
        // Skip hidden fields — agent can't interact with them
        if field.field_type == FieldType::Hidden {
            continue;
        }

        // Use pre-computed value (from DataGenerator/overrides), falling back to suggested
        let value = values
            .get(&field.label)
            .map(|v| v.as_str())
            .unwrap_or(&field.suggested_test_value);

        let selector = build_selector_for_field(field, &form.form_id);
        match field.field_type {
            FieldType::Select => session.select_option(&selector, value)?,
            FieldType::Checkbox | FieldType::Radio => session.check(&selector)?,
            _ => session.fill(&selector, value)?,
        }
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
// Auto-login
// ============================================================================

/// Perform automated login using the provided `AuthConfig`.
///
/// Navigates to the login URL, extracts the form, fills credentials from the
/// config, submits, and optionally verifies success via URL pattern check.
///
/// Returns `Ok(true)` if login was performed, `Ok(false)` if skipped
/// (no credentials or no login URL), `Err` on failure.
pub fn perform_login(
    session: &mut BrowserSession,
    auth: &AuthConfig,
    analyzer: &dyn PageAnalyzer,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !auth.has_credentials() {
        return Ok(false);
    }
    let login_url = match &auth.login_url {
        Some(url) => url.clone(),
        None => return Ok(false),
    };

    // Navigate to login page and analyze
    session.navigate(&login_url)?;
    let (screen, _) = crate::snapshot_session(session)?;
    let model = analyzer.analyze(&screen)?;

    let form = model.forms.first().ok_or_else(|| {
        AgentError::DomStructure("No form found on login page".into())
    })?;

    // Fill each field that has a matching credential
    for field in &form.fields {
        if field.field_type == FieldType::Hidden {
            continue;
        }
        if let Some(cred_value) = auth.credentials.get(&field.label) {
            let selector = build_selector_for_field(field, &form.form_id);
            session.fill(&selector, cred_value).map_err(|e| {
                AgentError::BrowserAction(format!("Login fill '{}' failed: {}", field.label, e))
            })?;
        }
    }

    // Submit — prefer configured label, fall back to form's submit label
    let submit_label = auth
        .submit_label
        .as_deref()
        .or_else(|| form.submit_label.as_deref());
    if let Some(label) = submit_label {
        let selector = SelectorHint {
            role: Some("button".into()),
            name: Some(label.into()),
            tag: None,
            input_type: None,
            form_id: None,
        };
        session
            .click(&selector)
            .map_err(|e| AgentError::BrowserAction(format!("Login submit failed: {}", e)))?;
    }

    session.wait_idle(1500).map_err(|e| {
        AgentError::BrowserAction(format!("Login wait failed: {}", e))
    })?;

    // Verify success via URL if configured
    if let Some(expected) = &auth.success_url_contains {
        let current = session.current_url().map_err(|e| {
            AgentError::BrowserAction(format!("Could not read URL after login: {}", e))
        })?;
        if !current.contains(expected.as_str()) {
            return Err(Box::new(AgentError::BrowserAction(format!(
                "Login failed: expected URL containing '{}', got '{}'",
                expected, current
            ))));
        }
    }

    Ok(true)
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
///
/// Example: `"https://example.com/path/page"` → `"https://example.com"`
pub fn extract_origin(url: &str) -> Option<String> {
    let after_scheme = url.find("://").map(|i| i + 3)?;
    let end = url[after_scheme..]
        .find('/')
        .map(|i| after_scheme + i)
        .unwrap_or(url.len());
    Some(url[..end].to_string())
}

/// Resolve a navigation candidate against a base URL.
///
/// Handles absolute URLs, protocol-relative URLs, root-relative paths,
/// and relative paths. Filters out `javascript:`, `mailto:`, `tel:`,
/// empty strings, and bare `#` fragments.
pub fn resolve_url(base: &str, candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty()
        || trimmed == "#"
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("tel:")
    {
        return None;
    }
    // Absolute URL — pass through
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Some(trimmed.to_string());
    }
    // Protocol-relative — inherit scheme from base
    if trimmed.starts_with("//") {
        let scheme = base.split("://").next().unwrap_or("https");
        return Some(format!("{}:{}", scheme, trimmed));
    }
    // Relative: resolve against base origin
    let origin = extract_origin(base)?;
    if trimmed.starts_with('/') {
        // Root-relative: /path
        Some(format!("{}{}", origin, trimmed))
    } else {
        // Relative path: resolve against base directory
        let base_dir = base.rfind('/').map(|i| &base[..i + 1]).unwrap_or(base);
        Some(format!("{}{}", base_dir, trimmed))
    }
}
