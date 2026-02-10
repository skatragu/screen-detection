use crate::agent::ai_model::{MockTextInference, TextInference, guess_value};
use crate::agent::error::AgentError;
use crate::agent::page_model::{
    FieldModel, FieldType, FormModel, NavigationTarget, OutputModel, PageCategory, PageModel,
    SuggestedAssertion,
};
use crate::state::state_model::ScreenState;

// ============================================================================
// PageAnalyzer trait — core abstraction for AI page understanding
// ============================================================================

/// Analyzes a `ScreenState` and produces a `PageModel` describing the page's
/// purpose, forms, suggested test values, assertions, and navigation targets.
pub trait PageAnalyzer {
    fn analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError>;
}

// ============================================================================
// Field type classification
// ============================================================================

/// Classify a field's semantic type from its input_type and label.
pub fn classify_field_type(input_type: Option<&str>, label: Option<&str>) -> FieldType {
    // First check input_type (most reliable signal)
    if let Some(t) = input_type {
        match t {
            "email" => return FieldType::Email,
            "password" => return FieldType::Password,
            "number" => return FieldType::Number,
            "date" | "datetime-local" | "datetime" => return FieldType::Date,
            "tel" => return FieldType::Tel,
            "url" => return FieldType::Url,
            "checkbox" => return FieldType::Checkbox,
            "radio" => return FieldType::Radio,
            "select" | "select-one" | "select-multiple" => return FieldType::Select,
            _ => {}
        }
    }

    // Fall back to label-based heuristics
    if let Some(l) = label {
        let lower = l.to_lowercase();
        if lower.contains("email") {
            return FieldType::Email;
        }
        if lower.contains("password") {
            return FieldType::Password;
        }
        if lower.contains("phone") || lower.contains("tel") {
            return FieldType::Tel;
        }
        if lower.contains("url") || lower.contains("website") {
            return FieldType::Url;
        }
        if lower.contains("date") {
            return FieldType::Date;
        }
        if lower.contains("number") || lower.contains("amount") || lower.contains("quantity") {
            return FieldType::Number;
        }
    }

    FieldType::Text
}

// ============================================================================
// Page category classification
// ============================================================================

/// Infer page category from form intents and page structure.
fn classify_page_category(screen: &ScreenState) -> PageCategory {
    // Check form intents first
    for form in &screen.forms {
        if let Some(intent) = &form.intent {
            let label_lower = intent.label.to_lowercase();
            if intent.confidence > 0.7 {
                if label_lower.contains("auth") || label_lower.contains("login") {
                    return PageCategory::Login;
                }
                if label_lower.contains("register") || label_lower.contains("signup") {
                    return PageCategory::Registration;
                }
            }
            if intent.confidence > 0.4 {
                if label_lower.contains("search") {
                    return PageCategory::Search;
                }
            }
        }
    }

    // Check title for clues
    let title_lower = screen.title.to_lowercase();
    if title_lower.contains("login") || title_lower.contains("sign in") {
        return PageCategory::Login;
    }
    if title_lower.contains("register") || title_lower.contains("sign up") {
        return PageCategory::Registration;
    }
    if title_lower.contains("search") {
        return PageCategory::Search;
    }
    if title_lower.contains("dashboard") {
        return PageCategory::Dashboard;
    }
    if title_lower.contains("settings") || title_lower.contains("preferences") {
        return PageCategory::Settings;
    }
    if title_lower.contains("checkout") || title_lower.contains("payment") {
        return PageCategory::Checkout;
    }
    if title_lower.contains("error") || title_lower.contains("404") || title_lower.contains("500")
    {
        return PageCategory::Error;
    }

    // Check for error outputs
    let has_error_output = screen.outputs.iter().any(|o| {
        o.label
            .as_ref()
            .is_some_and(|l| l.to_lowercase().contains("error"))
    });
    if has_error_output && screen.forms.is_empty() {
        return PageCategory::Error;
    }

    PageCategory::Other
}

// ============================================================================
// MockPageAnalyzer — deterministic, rule-based (for testing without LLM)
// ============================================================================

/// Rule-based page analyzer that uses existing heuristics to produce a `PageModel`
/// without any LLM calls. Uses `classify_field_type()` and `guess_value()` to
/// fill in form fields, and derives page category from `FormIntent` confidence.
pub struct MockPageAnalyzer;

impl PageAnalyzer for MockPageAnalyzer {
    fn analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError> {
        let category = classify_page_category(screen);

        let purpose = match &category {
            PageCategory::Login => "Login page for user authentication".to_string(),
            PageCategory::Registration => "Registration page for new user sign-up".to_string(),
            PageCategory::Search => "Search page for querying content".to_string(),
            PageCategory::Dashboard => "Dashboard page showing overview".to_string(),
            PageCategory::Settings => "Settings page for user preferences".to_string(),
            PageCategory::Checkout => "Checkout page for completing purchase".to_string(),
            PageCategory::Error => "Error page".to_string(),
            _ => format!("Web page: {}", screen.title),
        };

        // Build FormModels from screen forms
        let forms = screen
            .forms
            .iter()
            .map(|form| {
                let form_purpose = form
                    .intent
                    .as_ref()
                    .map(|i| i.label.clone())
                    .unwrap_or_else(|| "Form".to_string());

                let fields = form
                    .inputs
                    .iter()
                    .map(|input| {
                        let label = input.label.clone().unwrap_or_default();
                        let field_type =
                            classify_field_type(input.input_type.as_deref(), Some(&label));
                        let suggested_test_value =
                            guess_value(&label, input.input_type.as_deref());

                        FieldModel {
                            label,
                            field_type,
                            required: false, // Not tracked in ScreenElement currently
                            suggested_test_value,
                        }
                    })
                    .collect();

                let submit_label = form
                    .primary_action
                    .as_ref()
                    .or(form.actions.first())
                    .and_then(|a| a.label.clone());

                FormModel {
                    form_id: form.id.clone(),
                    purpose: form_purpose,
                    fields,
                    submit_label,
                }
            })
            .collect();

        // Build OutputModels
        let outputs = screen
            .outputs
            .iter()
            .filter_map(|o| {
                o.label.as_ref().map(|label| OutputModel {
                    description: label.clone(),
                    region: "main".to_string(),
                })
            })
            .collect();

        // Generate basic suggested assertions
        let mut suggested_assertions = Vec::new();
        if !screen.title.is_empty() {
            // Grab a meaningful keyword from the title for assertion
            let title_word = screen
                .title
                .split_whitespace()
                .next()
                .unwrap_or(&screen.title);
            suggested_assertions.push(SuggestedAssertion {
                assertion_type: "title_contains".to_string(),
                expected: title_word.to_string(),
                description: format!("Verify page title contains '{}'", title_word),
            });
        }

        // Capture navigation targets from standalone actions
        let navigation_targets = screen
            .standalone_actions
            .iter()
            .filter_map(|action| {
                action.label.as_ref().map(|label| NavigationTarget {
                    label: label.clone(),
                    likely_destination: format!("{} page", label),
                })
            })
            .collect();

        Ok(PageModel {
            purpose,
            category,
            forms,
            outputs,
            suggested_assertions,
            navigation_targets,
        })
    }
}

// ============================================================================
// LlmPageAnalyzer — uses TextInference to analyze pages via LLM
// ============================================================================

/// LLM-backed page analyzer. Sends a structured prompt to an LLM via
/// `TextInference`, parses the JSON response into a `PageModel`.
/// Falls back to `MockPageAnalyzer` if the LLM response can't be parsed.
pub struct LlmPageAnalyzer {
    backend: Box<dyn TextInference>,
}

impl LlmPageAnalyzer {
    pub fn new(backend: Box<dyn TextInference>) -> Self {
        Self { backend }
    }

    /// Build a structured prompt describing the page for LLM analysis.
    fn build_page_prompt(screen: &ScreenState) -> String {
        let forms_summary = screen
            .forms
            .iter()
            .map(|f| {
                let fields: Vec<String> = f
                    .inputs
                    .iter()
                    .map(|i| {
                        let label = i.label.clone().unwrap_or_else(|| "unlabeled".to_string());
                        let input_type =
                            i.input_type.clone().unwrap_or_else(|| "text".to_string());
                        format!("{} ({})", label, input_type)
                    })
                    .collect();
                let actions: Vec<String> = f
                    .actions
                    .iter()
                    .filter_map(|a| a.label.clone())
                    .collect();
                format!(
                    "    - Form '{}': fields=[{}], submit=[{}]",
                    f.id,
                    fields.join(", "),
                    actions.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let outputs_summary = screen
            .outputs
            .iter()
            .filter_map(|o| o.label.clone())
            .take(5)
            .collect::<Vec<_>>()
            .join("; ");

        let actions_summary = screen
            .standalone_actions
            .iter()
            .filter_map(|a| a.label.clone())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r##"Analyze this web page and return a JSON object describing it.

PAGE STATE:
- URL: {url}
- Title: {title}
- Forms:
{forms}
- Outputs: {outputs}
- Standalone actions: {actions}

Return ONLY valid JSON matching this exact schema:
{{
  "purpose": "Brief description of the page's purpose",
  "category": "Login|Registration|Search|Dashboard|Settings|Listing|Detail|Checkout|Error|Other",
  "forms": [
    {{
      "form_id": "form id string",
      "purpose": "what the form does",
      "fields": [
        {{
          "label": "field label",
          "field_type": "Text|Email|Password|Number|Date|Tel|Url|Select|Checkbox|Radio|Other",
          "required": true,
          "suggested_test_value": "a realistic test value"
        }}
      ],
      "submit_label": "button text or null"
    }}
  ],
  "outputs": [
    {{
      "description": "what the output shows",
      "region": "main|header|footer|results"
    }}
  ],
  "suggested_assertions": [
    {{
      "assertion_type": "title_contains|text_present|text_absent|element_visible|url_contains",
      "expected": "expected value",
      "description": "what this assertion verifies"
    }}
  ],
  "navigation_targets": [
    {{
      "label": "link or button text",
      "likely_destination": "where it goes"
    }}
  ]
}}

Respond with ONLY valid JSON, no explanation."##,
            url = screen.url.as_deref().unwrap_or("unknown"),
            title = screen.title,
            forms = if forms_summary.is_empty() {
                "    (none)".to_string()
            } else {
                forms_summary
            },
            outputs = if outputs_summary.is_empty() {
                "(none)"
            } else {
                &outputs_summary
            },
            actions = if actions_summary.is_empty() {
                "(none)"
            } else {
                &actions_summary
            },
        )
    }
}

impl PageAnalyzer for LlmPageAnalyzer {
    fn analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError> {
        let prompt = Self::build_page_prompt(screen);

        match self.backend.infer_text(&prompt) {
            Some(response) => {
                // Try to parse LLM response as PageModel
                match serde_json::from_str::<PageModel>(&response) {
                    Ok(model) => Ok(model),
                    Err(_) => {
                        // Fallback to deterministic analysis
                        MockPageAnalyzer.analyze(screen)
                    }
                }
            }
            None => {
                // LLM unavailable, fall back to deterministic
                MockPageAnalyzer.analyze(screen)
            }
        }
    }
}

// Convenience constructor for testing with mock responses
impl LlmPageAnalyzer {
    /// Create an LlmPageAnalyzer backed by a MockTextInference with a canned response.
    pub fn with_mock_response(response: &str) -> Self {
        Self {
            backend: Box::new(MockTextInference {
                response: response.to_string(),
            }),
        }
    }
}
