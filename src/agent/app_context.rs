use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::agent::page_model::PageModel;

// ============================================================================
// AppContext — growing cross-page understanding of the application under test
// ============================================================================

/// Accumulates domain knowledge and entered data as pages are visited during BFS.
///
/// `AppContext` does two inseparable things:
/// 1. **Cross-page data recall** — remembers every value entered across all pages so
///    the same label can be filled consistently on any subsequent page (e.g. an MSISDN
///    entered on step 1 is automatically reused on step 3 when asked again).
/// 2. **Domain tracking** — builds a growing free-form description of what kind of
///    application is being tested ("telecom SIM provisioning", "hospital patient intake",
///    "HR leave management"). This description is included in every subsequent LLM prompt
///    so the LLM generates domain-appropriate values.
///
/// `AppContext` is emergent — it starts empty and grows with every page visited.
/// It is created internally by `explore_live()` and is never passed as a parameter
/// by callers; the `explore_live()` signature remains unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppContext {
    /// Free-form application domain, updated by LLM after each page analysis.
    /// e.g. "telecom SIM provisioning", "hospital patient intake", "HR leave management"
    #[serde(default)]
    pub domain: Option<String>,

    /// Inferred multi-page flow description built from page sequence.
    /// e.g. "3-step SIM activation wizard", "2-page patient registration flow"
    #[serde(default)]
    pub inferred_flow: Option<String>,

    /// Cross-page data memory: lowercase_label → entered_value.
    /// Enables recall: "MSISDN entered on page 1 → reuse on page 3 when asked again".
    #[serde(default)]
    entered_values: HashMap<String, String>,

    /// Per-page summaries accumulated as BFS explores the application.
    /// Each entry records what the page does, what fields were seen, and what was filled.
    #[serde(default)]
    pub page_summaries: Vec<PageSummary>,
}

/// A single page's contribution to the application context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    /// URL of the visited page.
    pub url: String,
    /// Free-form purpose description from the LLM or deterministic analyzer.
    pub purpose: String,
    /// Field labels seen on this page (from all forms).
    pub fields_seen: Vec<String>,
    /// label → value map of what was actually filled on this page.
    pub values_entered: HashMap<String, String>,
}

impl AppContext {
    /// Create a fresh, empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a field was filled with a value.
    ///
    /// Stores the value under the lowercase label key for case-insensitive recall.
    /// Later pages can recall this value using `recall("MSISDN")` regardless of the
    /// original label casing.
    pub fn record_fill(&mut self, label: &str, value: &str) {
        self.entered_values
            .insert(label.to_lowercase(), value.to_string());
    }

    /// Recall a previously entered value by label (case-insensitive).
    ///
    /// Returns the most recently entered value for this label across all pages visited so far.
    /// Returns `None` if this label has never been filled.
    pub fn recall(&self, label: &str) -> Option<&str> {
        self.entered_values
            .get(&label.to_lowercase())
            .map(|s| s.as_str())
    }

    /// Record visiting a page and update cross-page memory with what was filled.
    ///
    /// - Updates `entered_values` with everything in `values`.
    /// - Appends a `PageSummary` capturing the page's purpose, all field labels, and
    ///   what was actually filled.
    pub fn record_page(
        &mut self,
        url: &str,
        model: &PageModel,
        values: HashMap<String, String>,
    ) {
        // Update cross-page data memory
        for (label, value) in &values {
            self.record_fill(label, value);
        }

        // Collect all field labels seen on this page
        let fields_seen = model
            .forms
            .iter()
            .flat_map(|f| f.fields.iter().map(|fld| fld.label.clone()))
            .collect();

        self.page_summaries.push(PageSummary {
            url: url.to_string(),
            purpose: model.purpose.clone(),
            fields_seen,
            values_entered: values,
        });
    }

    /// Update the application domain understanding from the LLM's free-form `domain` field.
    ///
    /// Called after each page's LLM analysis. Only updates if the new domain is non-empty.
    /// The domain is included in every subsequent page's prompt so the LLM generates
    /// domain-appropriate values consistently across all pages.
    pub fn update_domain(&mut self, domain: Option<String>) {
        if let Some(d) = domain {
            if !d.is_empty() {
                self.domain = Some(d);
            }
        }
    }

    /// Build a concise context summary to include in the next page's LLM prompt.
    ///
    /// Returns an empty string if no pages have been visited yet (first page has no prior context).
    /// Otherwise returns a multi-line summary of:
    /// - Application domain (if known)
    /// - Inferred flow (if known)
    /// - Pages visited so far with URLs and purposes
    /// - Data entered so far (label → value pairs)
    pub fn build_context_summary(&self) -> String {
        if self.page_summaries.is_empty() {
            return String::new();
        }

        let domain_line = self
            .domain
            .as_deref()
            .map(|d| format!("Application domain: {d}\n"))
            .unwrap_or_default();

        let flow_line = self
            .inferred_flow
            .as_deref()
            .map(|f| format!("Inferred flow: {f}\n"))
            .unwrap_or_default();

        let pages_seen = self
            .page_summaries
            .iter()
            .map(|p| format!("  - {} ({})", p.url, p.purpose))
            .collect::<Vec<_>>()
            .join("\n");

        // Sort entries for deterministic output (important for tests)
        let mut kv: Vec<_> = self.entered_values.iter().collect();
        kv.sort_by_key(|(k, _)| k.as_str());
        let data_entered = kv
            .iter()
            .map(|(k, v)| format!("  {k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "{domain_line}{flow_line}Pages visited so far:\n{pages_seen}\n\nData entered so far:\n{data_entered}"
        )
    }

    /// Returns true if no pages have been visited yet.
    pub fn is_empty(&self) -> bool {
        self.page_summaries.is_empty()
    }

    /// Returns the number of pages visited so far.
    pub fn pages_visited(&self) -> usize {
        self.page_summaries.len()
    }
}
