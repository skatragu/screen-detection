use serde::{Deserialize, Serialize};

// ============================================================================
// Page-level understanding produced by AI analysis of a ScreenState
// ============================================================================

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
    Textarea,
    Search,
    Hidden,
    Time,
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

impl FieldModel {
    /// Return the HTML `input_type` string that best represents this field's FieldType.
    ///
    /// Used by `DataGenerator` to pass the right input_type to `guess_value()` as a fallback,
    /// so label-pattern matching is combined with type-based fallbacks.
    /// Returns `None` for types that don't map directly to an input_type (Select, Textarea, Other).
    pub fn input_type_str(&self) -> Option<&'static str> {
        match self.field_type {
            FieldType::Email => Some("email"),
            FieldType::Password => Some("password"),
            FieldType::Number => Some("number"),
            FieldType::Date => Some("date"),
            FieldType::Tel => Some("tel"),
            FieldType::Url => Some("url"),
            FieldType::Checkbox => Some("checkbox"),
            FieldType::Radio => Some("radio"),
            FieldType::Search => Some("search"),
            FieldType::Time => Some("time"),
            FieldType::Hidden => Some("hidden"),
            FieldType::Text | FieldType::Textarea | FieldType::Select | FieldType::Other => None,
        }
    }
}

/// What the system expects to happen after a form submission or page action.
///
/// Used by the test generator to build meaningful post-submit assertions.
/// Defaults to empty (no assertions) — `MockPageAnalyzer` and `LlmPageAnalyzer`
/// fill this based on page category and LLM hints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExpectedOutcome {
    /// URL substring expected after a successful action (e.g. `"/dashboard"`)
    #[serde(default)]
    pub url_contains: Option<String>,
    /// URL substring expected to be ABSENT after success (e.g. `"/login"`)
    #[serde(default)]
    pub url_not_contains: Option<String>,
    /// Text fragments expected to appear on success (e.g. `["welcome", "dashboard"]`)
    #[serde(default)]
    pub success_text: Vec<String>,
    /// Text fragments that should be ABSENT on success (e.g. `["invalid", "error"]`)
    #[serde(default)]
    pub error_indicators: Vec<String>,
}

/// AI understanding of a single form on the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormModel {
    pub form_id: String,
    pub purpose: String,
    pub fields: Vec<FieldModel>,
    pub submit_label: Option<String>,
    /// What to expect after submitting this form.
    #[serde(default)]
    pub expected_outcome: ExpectedOutcome,
}

/// Semantic classification of an output element's meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputSemantic {
    Success,
    Error,
    Warning,
    Info,
    Data,
    Navigation,
}

fn default_output_semantic() -> OutputSemantic {
    OutputSemantic::Info
}

/// Description of an output region on the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputModel {
    pub description: String,
    pub region: String,
    #[serde(default = "default_output_semantic")]
    pub semantic: OutputSemantic,
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
    #[serde(default)]
    pub href: Option<String>,
}

/// Deep LLM analysis of a single form field (from rich DOM context).
///
/// Populated by `LlmPageAnalyzer` using the rich DOM prompt which includes
/// fieldset legends, section headings, nearby help text, and ARIA descriptions.
/// Contains context clues, validation hints, a suggested test value, and negative test values.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FieldAnalysis {
    pub label: String,
    /// What section headings, fieldset legends, help text, and ARIA descriptions reveal
    /// about this field's purpose in the application domain.
    #[serde(default)]
    pub context_clues: String,
    /// Any validation rule visible in help text, ARIA descriptions, or HTML constraints.
    #[serde(default)]
    pub validation_hint: Option<String>,
    /// A realistic test value appropriate for this specific application domain.
    #[serde(default)]
    pub suggested_value: Option<String>,
    /// Values that should trigger a validation error (for negative test scenarios).
    #[serde(default)]
    pub negative_values: Vec<String>,
}

/// A test scenario suggested by the LLM from its analysis of the page.
///
/// Populated by `LlmPageAnalyzer` based on form structure, help text, and domain context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    /// Scenario type: "happy_path", "negative", "boundary", or "edge_case"
    #[serde(rename = "type")]
    pub scenario_type: String,
    #[serde(default)]
    pub description: String,
}

/// Complete AI understanding of a web page.
///
/// Produced by `PageAnalyzer::analyze()` from a `ScreenState`.
/// Contains the page's purpose, domain description, form analysis with suggested
/// test values, output descriptions, suggested assertions, and
/// navigation targets for exploration.
///
/// `domain` is a free-form string that describes what kind of application/process
/// this page is part of — e.g. "SIM card provisioning step 2", "patient intake form",
/// "employee leave request". It is domain-neutral and discovered by the LLM at runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageModel {
    pub purpose: String,
    /// Free-form application domain description (e.g. "telecom SIM provisioning",
    /// "medical patient intake", "HR leave management").
    /// Set by LlmPageAnalyzer from LLM response; derived from title by MockPageAnalyzer.
    #[serde(default)]
    pub domain: String,
    pub forms: Vec<FormModel>,
    pub outputs: Vec<OutputModel>,
    pub suggested_assertions: Vec<SuggestedAssertion>,
    pub navigation_targets: Vec<NavigationTarget>,
    /// Page-level expectations (e.g. error indicators that should never appear).
    #[serde(default)]
    pub expected_outcome: ExpectedOutcome,
    /// Brief description of the page layout inferred from structural outline.
    /// Populated by LlmPageAnalyzer; None for MockPageAnalyzer.
    #[serde(default)]
    pub layout_description: Option<String>,
    /// Per-field deep analysis from the LLM (context clues, validation hints, suggested values).
    /// Only populated by LlmPageAnalyzer with rich DOM prompt; empty for MockPageAnalyzer.
    #[serde(default)]
    pub field_analyses: Vec<FieldAnalysis>,
    /// Test scenarios suggested by the LLM based on page structure and domain.
    /// Only populated by LlmPageAnalyzer; empty for MockPageAnalyzer.
    #[serde(default)]
    pub suggested_test_scenarios: Vec<TestScenario>,
}
