use serde::{Deserialize, Serialize};

// ============================================================================
// Page-level understanding produced by AI analysis of a ScreenState
// ============================================================================

/// High-level page category inferred from page structure and content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageCategory {
    Login,
    Registration,
    Search,
    Dashboard,
    Settings,
    Listing,
    Detail,
    Checkout,
    Error,
    Other,
}

/// Classification of a form field's semantic type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Text,
    Email,
    Password,
    Number,
    Date,
    Tel,
    Url,
    Select,
    Checkbox,
    Radio,
    Other,
}

/// A single form field with its inferred type and suggested test value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldModel {
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub suggested_test_value: String,
}

/// AI understanding of a single form on the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormModel {
    pub form_id: String,
    pub purpose: String,
    pub fields: Vec<FieldModel>,
    pub submit_label: Option<String>,
}

/// Description of an output region on the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputModel {
    pub description: String,
    pub region: String,
}

/// An assertion the AI suggests for testing this page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuggestedAssertion {
    pub assertion_type: String,
    pub expected: String,
    pub description: String,
}

/// A link or action that navigates to another page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationTarget {
    pub label: String,
    pub likely_destination: String,
}

/// Complete AI understanding of a web page.
///
/// Produced by `PageAnalyzer::analyze()` from a `ScreenState`.
/// Contains the page's purpose, category, form analysis with suggested
/// test values, output descriptions, suggested assertions, and
/// navigation targets for exploration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageModel {
    pub purpose: String,
    pub category: PageCategory,
    pub forms: Vec<FormModel>,
    pub outputs: Vec<OutputModel>,
    pub suggested_assertions: Vec<SuggestedAssertion>,
    pub navigation_targets: Vec<NavigationTarget>,
}
