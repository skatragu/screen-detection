use std::collections::HashMap;
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

/// Classify a field's semantic type from its input_type, label, and HTML tag.
pub fn classify_field_type(input_type: Option<&str>, label: Option<&str>, tag: Option<&str>) -> FieldType {
    // Check HTML tag first (textarea has no type attribute)
    if let Some(t) = tag {
        if t == "textarea" {
            return FieldType::Textarea;
        }
        if t == "select" {
            return FieldType::Select;
        }
    }

    // Check input_type (most reliable signal for <input>)
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
            "search" => return FieldType::Search,
            "time" => return FieldType::Time,
            "hidden" => return FieldType::Hidden,
            "color" | "range" | "file" | "month" | "week" => return FieldType::Other,
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
        if lower.contains("search") || lower.contains("query") {
            return FieldType::Search;
        }
        if lower.contains("comment") || lower.contains("message") || lower.contains("description")
            || lower.contains("bio") || lower.contains("notes") {
            return FieldType::Textarea;
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
                            classify_field_type(input.input_type.as_deref(), Some(&label), input.tag.as_deref());

                        // For selects with options, use first option value; otherwise guess_value
                        let suggested_test_value = if field_type == FieldType::Select {
                            input.options.as_ref()
                                .and_then(|opts| opts.first())
                                .map(|o| o.value.clone())
                                .unwrap_or_else(|| guess_value(&label, input.input_type.as_deref(), Some(&category)))
                        } else {
                            guess_value(&label, input.input_type.as_deref(), Some(&category))
                        };

                        FieldModel {
                            label,
                            field_type,
                            required: input.required,
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
// LLM response parsing — robust recovery for small models
// ============================================================================

/// Lightweight response structure for LLM output parsing.
/// Only purpose and category — the LLM is not asked for form details.
#[derive(serde::Deserialize)]
pub struct LlmPageResponse {
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub field_values: Option<HashMap<String, String>>,
}

/// Attempt to fix common JSON issues from small LLM output.
/// Tries: raw parse → strip markdown fences → fix trailing commas → extract JSON substring.
pub fn try_parse_llm_response(raw: &str) -> Option<LlmPageResponse> {
    // Stage 1: Direct parse
    if let Ok(parsed) = serde_json::from_str::<LlmPageResponse>(raw) {
        return Some(parsed);
    }

    // Stage 2: Strip markdown code fences (```json ... ```)
    let stripped = raw
        .trim()
        .strip_prefix("```json")
        .or_else(|| raw.trim().strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(raw)
        .trim();

    if let Ok(parsed) = serde_json::from_str::<LlmPageResponse>(stripped) {
        return Some(parsed);
    }

    // Stage 3: Fix trailing commas (common small-model error)
    let no_trailing = stripped.replace(",}", "}").replace(",]", "]");
    if let Ok(parsed) = serde_json::from_str::<LlmPageResponse>(&no_trailing) {
        return Some(parsed);
    }

    // Stage 4: Extract first JSON object substring { ... }
    if let Some(start) = stripped.find('{') {
        if let Some(end) = stripped.rfind('}') {
            if end > start {
                let substring = &stripped[start..=end];
                let fixed = substring.replace(",}", "}").replace(",]", "]");
                if let Ok(parsed) = serde_json::from_str::<LlmPageResponse>(&fixed) {
                    return Some(parsed);
                }
            }
        }
    }

    None
}

/// Map LLM's simplified category string to PageCategory.
/// Handles case-insensitive matching and common variations.
pub fn map_llm_category(raw: &str) -> PageCategory {
    let lower = raw.to_lowercase();
    match lower.trim() {
        "login" | "signin" | "sign in" => PageCategory::Login,
        "registration" | "register" | "signup" | "sign up" => PageCategory::Registration,
        "search" => PageCategory::Search,
        "dashboard" | "home" => PageCategory::Dashboard,
        "settings" | "preferences" => PageCategory::Settings,
        "listing" | "list" => PageCategory::Listing,
        "detail" | "details" => PageCategory::Detail,
        "checkout" | "payment" => PageCategory::Checkout,
        "error" | "404" | "500" => PageCategory::Error,
        _ => PageCategory::Other,
    }
}

// ============================================================================
// LlmPageAnalyzer — hybrid enrichment (LLM semantics + deterministic details)
// ============================================================================

/// LLM-backed page analyzer using hybrid enrichment strategy.
///
/// Asks the LLM for only `purpose` and `category` (simple 2-field JSON),
/// then builds the full `PageModel` deterministically via `MockPageAnalyzer`.
/// The LLM's purpose and category override the deterministic defaults when
/// available and valid. This approach never fails — it always produces a
/// complete `PageModel`.
pub struct LlmPageAnalyzer {
    backend: Box<dyn TextInference>,
}

impl LlmPageAnalyzer {
    pub fn new(backend: Box<dyn TextInference>) -> Self {
        Self { backend }
    }

    /// Build a simplified prompt for small (1-1.5B) models.
    /// Only asks for purpose and category — 2 fields, ~120 words total.
    pub fn build_page_prompt(screen: &ScreenState) -> String {
        let forms_summary = screen
            .forms
            .iter()
            .map(|f| {
                let fields: Vec<String> = f
                    .inputs
                    .iter()
                    .map(|i| {
                        let label = i.label.clone().unwrap_or_else(|| "unlabeled".into());
                        let input_type = i.input_type.clone().unwrap_or_else(|| "text".into());
                        format!("{} ({})", label, input_type)
                    })
                    .collect();
                let actions: Vec<String> =
                    f.actions.iter().filter_map(|a| a.label.clone()).collect();
                format!(
                    "  Form '{}': fields=[{}], buttons=[{}]",
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
            .take(3)
            .collect::<Vec<_>>()
            .join("; ");

        format!(
            r#"Analyze this web page. Return JSON with "purpose" and "category".

Page:
- URL: {url}
- Title: {title}
- Forms:
{forms}
- Visible text: {outputs}

Categories: Login, Search, Dashboard, Error, Form, Other

If forms are present, suggest test values:
"field_values":{{"Email":"test@example.com","Password":"pass123"}}

Example:
Page with title "Sign In", form with email+password fields, "Log In" button.
{{"purpose":"User login page","category":"Login","field_values":{{"Email":"user@test.com","Password":"Test123!"}}}}

Return ONLY JSON:{{"purpose":"...","category":"...","field_values":{{...}}}}"#,
            url = screen.url.as_deref().unwrap_or("unknown"),
            title = screen.title,
            forms = if forms_summary.is_empty() {
                "  (none)".to_string()
            } else {
                forms_summary
            },
            outputs = if outputs_summary.is_empty() {
                "(none)"
            } else {
                &outputs_summary
            },
        )
    }
}

impl PageAnalyzer for LlmPageAnalyzer {
    fn analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError> {
        let prompt = Self::build_page_prompt(screen);

        // Try to get LLM-provided purpose, category, and field values
        let (llm_purpose, llm_category, llm_field_values) = match self.backend.infer_text(&prompt) {
            Some(response) => match try_parse_llm_response(&response) {
                Some(parsed) => {
                    let purpose = parsed.purpose;
                    let category = parsed.category.map(|c| map_llm_category(&c));
                    let field_values = parsed.field_values;
                    (purpose, category, field_values)
                }
                None => (None, None, None),
            },
            None => (None, None, None),
        };

        // Build the base model deterministically (always succeeds)
        let mut model = MockPageAnalyzer.analyze(screen)?;

        // Enrich with LLM insights where available
        if let Some(purpose) = llm_purpose {
            if !purpose.is_empty() {
                model.purpose = purpose;
            }
        }
        if let Some(category) = llm_category {
            model.category = category;
        }

        // Enrich field values from LLM if available
        if let Some(field_values) = llm_field_values {
            for form in &mut model.forms {
                for field in &mut form.fields {
                    if let Some(llm_value) = field_values.get(&field.label) {
                        if !llm_value.is_empty() {
                            field.suggested_test_value = llm_value.clone();
                        }
                    }
                }
            }
        }

        Ok(model)
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
