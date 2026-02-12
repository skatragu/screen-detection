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
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            start_url: String::new(),
            max_pages: 10,
            max_depth: 3,
            same_origin_only: true,
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

/// A directed edge between two pages in the AppMap.
///
/// Records which link/button text caused the transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// Source page URL
    pub from_url: String,

    /// Destination page URL
    pub to_url: String,

    /// Label of the link or button that caused this transition
    pub label: String,
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
