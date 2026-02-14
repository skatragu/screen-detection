use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::agent::page_model::PageModel;

// ============================================================================
// Explorer configuration
// ============================================================================

/// Configuration for autonomous exploration.
///
/// Controls how the explorer traverses an application: starting URL,
/// page/depth limits, and origin restrictions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerConfig {
    /// URL to start exploring from
    pub start_url: String,

    /// Maximum number of pages to visit (default 10)
    pub max_pages: usize,

    /// Maximum depth from start URL (default 3)
    pub max_depth: usize,

    /// Only follow links to the same origin (default true)
    pub same_origin_only: bool,

    /// Whether to fill and submit forms during exploration (default true)
    #[serde(default = "default_true")]
    pub explore_forms: bool,

    /// Maximum number of forms to submit per page (default 3)
    #[serde(default = "default_three")]
    pub max_forms_per_page: usize,
}

fn default_true() -> bool {
    true
}

fn default_three() -> usize {
    3
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            start_url: String::new(),
            max_pages: 10,
            max_depth: 3,
            same_origin_only: true,
            explore_forms: true,
            max_forms_per_page: 3,
        }
    }
}

// ============================================================================
// Page graph data model
// ============================================================================

/// A single page discovered during exploration.
///
/// Stores the URL, title, depth from start, and the AI-generated
/// `PageModel` (from Phase 4) describing the page's structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageNode {
    /// Page URL
    pub url: String,

    /// Page title
    pub title: String,

    /// Number of hops from the start URL
    pub depth: usize,

    /// AI understanding of the page (forms, fields, assertions, navigation)
    pub page_model: PageModel,
}

/// How a transition between pages was triggered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransitionKind {
    /// Clicked a link or standalone button
    Link,
    /// Filled and submitted a form
    FormSubmission {
        form_id: String,
        values: HashMap<String, String>,
    },
}

impl Default for TransitionKind {
    fn default() -> Self {
        TransitionKind::Link
    }
}

/// A directed edge between two pages in the AppMap.
///
/// Records which link/button text caused the transition and how
/// the transition was triggered (link click or form submission).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// Source page URL
    pub from_url: String,

    /// Destination page URL
    pub to_url: String,

    /// Label of the link or button that caused this transition
    pub label: String,

    /// How this transition was triggered (defaults to Link for backward compat)
    #[serde(default)]
    pub kind: TransitionKind,
}

/// A single step in a detected multi-page flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FlowStep {
    /// Navigate to a URL
    Navigate { url: String },
    /// Fill a form and submit it
    FillAndSubmit {
        url: String,
        form_id: String,
        values: HashMap<String, String>,
        submit_label: Option<String>,
    },
}

/// A detected multi-step user flow (e.g., login -> dashboard).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Flow {
    /// Human-readable name for the flow
    pub name: String,
    /// Ordered steps in the flow
    pub steps: Vec<FlowStep>,
}

/// Graph of discovered pages and transitions.
///
/// Built during exploration and consumed by the test generator
/// to produce `TestSpec` entries for each discovered page and form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMap {
    /// Pages keyed by URL
    pub pages: HashMap<String, PageNode>,

    /// Directed edges between pages
    pub transitions: Vec<Transition>,
}

impl AppMap {
    /// Create an empty AppMap.
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
            transitions: Vec::new(),
        }
    }

    /// Add a discovered page. If the URL already exists, it is replaced.
    pub fn add_page(&mut self, node: PageNode) {
        self.pages.insert(node.url.clone(), node);
    }

    /// Record a transition between two pages.
    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    /// Number of discovered pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Check if a page has already been discovered.
    pub fn has_page(&self, url: &str) -> bool {
        self.pages.contains_key(url)
    }
}
