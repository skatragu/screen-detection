use std::collections::HashMap;
use crate::agent::ai_model::{MockTextInference, TextInference, constrained_value, guess_value};
use crate::agent::app_context::AppContext;
use crate::agent::error::AgentError;
use crate::agent::page_model::{
    ExpectedOutcome, FieldModel, FieldType, FormModel, NavigationTarget,
    OutputModel, OutputSemantic, PageModel, SuggestedAssertion,
};
use crate::screen::screen_model::SelectOption;
use crate::state::state_model::ScreenState;

// ============================================================================
// PageAnalyzer trait — core abstraction for AI page understanding
// ============================================================================

/// Analyzes a `ScreenState` and produces a `PageModel` describing the page's
/// purpose, forms, suggested test values, assertions, and navigation targets.
pub trait PageAnalyzer {
    fn analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError>;

    /// Analyze with accumulated application context from previous pages.
    ///
    /// The default implementation ignores the context (backward compat).
    /// `LlmPageAnalyzer` overrides this to include the context in the prompt,
    /// enabling cross-page domain understanding and data consistency.
    fn analyze_with_context(
        &self,
        screen: &ScreenState,
        _ctx: &AppContext,
    ) -> Result<PageModel, AgentError> {
        self.analyze(screen)
    }
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
// Smart dropdown option selection
// ============================================================================

/// Select the best option from a dropdown, skipping common placeholder values.
///
/// Placeholder patterns: empty value, text starting with "select"/"choose"/"--"/"please",
/// or text that is "none" or "n/a". Falls back to first option if all options look
/// like placeholders.
pub fn smart_select_option(options: &[SelectOption]) -> Option<&SelectOption> {
    let is_placeholder = |opt: &SelectOption| -> bool {
        let text_lower = opt.text.trim().to_lowercase();
        let value = opt.value.trim();

        value.is_empty()
            || text_lower.is_empty()
            || text_lower.starts_with("select")
            || text_lower.starts_with("choose")
            || text_lower.starts_with("--")
            || text_lower.starts_with("please")
            || text_lower == "none"
            || text_lower == "n/a"
    };

    // Try to find a non-placeholder option
    options
        .iter()
        .find(|opt| !is_placeholder(opt))
        .or_else(|| options.first()) // fallback to first if all are placeholders
}

// ============================================================================
// Output semantic classification
// ============================================================================

/// Classify the semantic meaning of an output element from its text content.
pub fn classify_output_semantic(text: &str) -> OutputSemantic {
    let lower = text.to_lowercase();

    // Error patterns (check first — most specific)
    if lower.contains("error")
        || lower.contains("invalid")
        || lower.contains("failed")
        || lower.contains("not found")
        || lower.contains("denied")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("incorrect")
        || lower.contains("expired")
        || lower.contains("required")
    {
        return OutputSemantic::Error;
    }

    // Success patterns
    if lower.contains("success")
        || lower.contains("welcome")
        || lower.contains("created")
        || lower.contains("saved")
        || lower.contains("updated")
        || lower.contains("confirmed")
        || lower.contains("thank you")
        || lower.contains("logged in")
        || lower.contains("completed")
    {
        return OutputSemantic::Success;
    }

    // Warning patterns
    if lower.contains("warning")
        || lower.contains("caution")
        || lower.contains("expires")
        || lower.contains("limited")
    {
        return OutputSemantic::Warning;
    }

    // Navigation patterns
    if (lower.contains("home") && lower.len() < 20)
        || (lower.contains("next") && lower.len() < 20)
        || (lower.contains("previous") && lower.len() < 20)
        || (lower.contains("page ") && lower.contains(" of "))
    {
        return OutputSemantic::Navigation;
    }

    OutputSemantic::Info
}

// ============================================================================
// MockPageAnalyzer — deterministic, rule-based (for testing without LLM)
// ============================================================================

/// Derive a plain-text domain description from page structure.
///
/// Uses form intents and title keywords to produce a free-form domain string,
/// without assuming any particular application domain.
fn infer_domain_from_screen(screen: &ScreenState) -> String {
    // Check form intents first for high-confidence signals
    for form in &screen.forms {
        if let Some(intent) = &form.intent {
            if intent.confidence > 0.4 && !intent.label.is_empty() {
                return intent.label.clone();
            }
        }
    }

    // Derive from title
    if !screen.title.is_empty() {
        return screen.title.clone();
    }

    // Derive from URL
    if let Some(url) = &screen.url {
        return url.clone();
    }

    "Web page".to_string()
}

/// Rule-based page analyzer that uses existing heuristics to produce a `PageModel`
/// without any LLM calls. Uses `classify_field_type()` and `guess_value()` to
/// fill in form fields, and derives domain from `FormIntent` confidence + title.
pub struct MockPageAnalyzer;

impl PageAnalyzer for MockPageAnalyzer {
    fn analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError> {
        let domain = infer_domain_from_screen(screen);

        // Purpose derived from the primary form intent or page title
        let purpose = screen
            .forms
            .iter()
            .find_map(|f| f.intent.as_ref().filter(|i| i.confidence > 0.4).map(|i| i.label.clone()))
            .unwrap_or_else(|| format!("Web page: {}", screen.title));

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
                    .filter(|input| !input.readonly)
                    .map(|input| {
                        let label = input.label.clone().unwrap_or_default();
                        let field_type =
                            classify_field_type(input.input_type.as_deref(), Some(&label), input.tag.as_deref());

                        // For selects with options, use smart selection; otherwise constrained_value
                        let suggested_test_value = if field_type == FieldType::Select {
                            input.options.as_ref()
                                .and_then(|opts| smart_select_option(opts))
                                .map(|o| o.value.clone())
                                .unwrap_or_else(|| guess_value(&label, input.input_type.as_deref()))
                        } else {
                            constrained_value(&label, input.input_type.as_deref(), input.maxlength, input.minlength)
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
                    expected_outcome: ExpectedOutcome::default(),
                }
            })
            .collect();

        // Build OutputModels with semantic classification
        let outputs: Vec<OutputModel> = screen
            .outputs
            .iter()
            .filter_map(|o| {
                o.label.as_ref().map(|label| OutputModel {
                    description: label.clone(),
                    region: "main".to_string(),
                    semantic: classify_output_semantic(label),
                })
            })
            .collect();

        // Build page-level expected outcome from semantic output signals
        let page_expected_outcome = {
            let error_indicators: Vec<String> = outputs
                .iter()
                .filter(|o| o.semantic == OutputSemantic::Error)
                .map(|o| o.description.clone())
                .collect();
            let success_text: Vec<String> = outputs
                .iter()
                .filter(|o| o.semantic == OutputSemantic::Success)
                .map(|o| o.description.clone())
                .collect();
            ExpectedOutcome {
                error_indicators,
                success_text,
                ..Default::default()
            }
        };

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
        // Add text_absent assertions for error indicators from page-level outcome
        for text in &page_expected_outcome.error_indicators {
            suggested_assertions.push(SuggestedAssertion {
                assertion_type: "text_absent".to_string(),
                expected: text.clone(),
                description: format!("No error text: '{}'", text),
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
                    href: action.href.clone(),
                })
            })
            .collect();

        Ok(PageModel {
            purpose,
            domain,
            forms,
            outputs,
            suggested_assertions,
            navigation_targets,
            expected_outcome: page_expected_outcome,
            layout_description: None,
            field_analyses: vec![],
            suggested_test_scenarios: vec![],
        })
    }
}

// ============================================================================
// Outcome inference — LLM provides indicators; default is empty
// ============================================================================

/// Return an empty expected outcome — indicators are now provided by the LLM
/// via `success_indicators` / `error_indicators` in the LLM response.
///
/// The rule-based category-specific outcomes have been removed: they were
/// biased toward specific application domains (login/checkout/settings).
/// The LLM, given rich DOM context, generates appropriate indicators for any domain.
pub fn infer_form_outcome() -> ExpectedOutcome {
    ExpectedOutcome::default()
}

// ============================================================================
// LLM response parsing — robust recovery for small models
// ============================================================================

/// Lightweight response structure for LLM output parsing.
/// LLM response fields — purpose, domain, field values, analyses, and outcome indicators.
#[derive(serde::Deserialize)]
pub struct LlmPageResponse {
    #[serde(default)]
    pub purpose: Option<String>,
    /// Kept for backward compat with old prompts that returned category.
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub field_values: Option<HashMap<String, String>>,
    /// Text expected on page after successful action (e.g., "welcome", "confirmed")
    #[serde(default)]
    pub success_indicators: Option<Vec<String>>,
    /// Text seen on page when action failed (e.g., "invalid", "error", "declined")
    #[serde(default)]
    pub error_indicators: Option<Vec<String>>,
    // New fields from rich prompt (Phase 15 Step 2):
    /// Free-form domain description returned by rich prompt.
    #[serde(default)]
    pub domain: Option<String>,
    /// Brief layout description from structural outline analysis.
    #[serde(default)]
    pub layout_description: Option<String>,
    /// Per-field deep analyses from rich DOM context.
    #[serde(default)]
    pub field_analyses: Option<Vec<serde_json::Value>>,
    /// Test scenarios suggested by the LLM.
    #[serde(default)]
    pub test_scenarios: Option<Vec<serde_json::Value>>,
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

// ============================================================================
// Rich DOM prompt builder — replaces ~120-word build_page_prompt for Step 2+
// ============================================================================

/// Build a comprehensive page prompt using rich DOM context.
///
/// Includes the structural outline (headings/landmarks), form fields with
/// help text / fieldset groups / ARIA descriptions, and all visible outputs.
/// When `context` is `Some`, also includes an accumulated context section
/// ("what we've seen and entered so far") so the LLM generates consistent,
/// domain-appropriate values across pages.
/// Asks the LLM for domain, layout_description, field_analyses, test_scenarios,
/// field_values, success_indicators, and error_indicators.
///
/// Pass `None` as context on the first page (no prior context yet).
pub fn build_rich_page_prompt(screen: &ScreenState, context: Option<&AppContext>) -> String {
    let url = screen.url.as_deref().unwrap_or("unknown");
    let title = &screen.title;

    // Structural outline: headings and landmarks
    let outline = if screen.structural_outline.headings.is_empty()
        && screen.structural_outline.landmarks.is_empty()
    {
        String::new()
    } else {
        let headings = screen
            .structural_outline
            .headings
            .iter()
            .map(|h| format!("H{}: {}", h.level, h.text))
            .collect::<Vec<_>>()
            .join(", ");
        let landmarks = screen
            .structural_outline
            .landmarks
            .iter()
            .filter(|l| !l.label.is_empty())
            .map(|l| format!("<{}> {}", l.tag, l.label))
            .collect::<Vec<_>>()
            .join(", ");
        let mut parts = Vec::new();
        if !headings.is_empty() {
            parts.push(format!("Headings: {}", headings));
        }
        if !landmarks.is_empty() {
            parts.push(format!("Landmarks: {}", landmarks));
        }
        parts.join("\n")
    };

    // Forms with rich per-field context
    let forms_section = screen
        .forms
        .iter()
        .map(|form| {
            let fields = form
                .inputs
                .iter()
                .map(|input| {
                    let label = input.label.clone().unwrap_or_else(|| "unlabeled".into());
                    let type_ = input.input_type.clone().unwrap_or_else(|| "text".into());
                    let mut parts = vec![format!("  • {label} [{type_}]")];
                    if let Some(legend) = &input.fieldset_legend {
                        parts.push(format!("    group: {legend}"));
                    }
                    if let Some(section) = &input.section_heading {
                        parts.push(format!("    section: {section}"));
                    }
                    if let Some(help) = &input.nearby_help_text {
                        parts.push(format!("    help: {help}"));
                    }
                    if let Some(desc) = &input.aria_describedby_text {
                        parts.push(format!("    description: {desc}"));
                    }
                    if let Some(ac) = &input.autocomplete {
                        if !ac.is_empty() && ac != "off" {
                            parts.push(format!("    autocomplete: {ac}"));
                        }
                    }
                    if input.required {
                        parts.push("    required: yes".into());
                    }
                    if let Some(max) = input.maxlength {
                        parts.push(format!("    maxlength: {max}"));
                    }
                    if let Some(min) = input.minlength {
                        parts.push(format!("    minlength: {min}"));
                    }
                    parts.join("\n")
                })
                .collect::<Vec<_>>()
                .join("\n");

            let buttons = form
                .actions
                .iter()
                .filter_map(|a| a.label.as_deref())
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "Form '{id}':\n{fields}\n  Submit: [{buttons}]",
                id = form.id
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // All visible outputs (not just first 3)
    let outputs = screen
        .outputs
        .iter()
        .filter_map(|o| o.label.as_deref())
        .take(10)
        .collect::<Vec<_>>()
        .join("\n  ");

    let outline_section = if outline.is_empty() {
        String::new()
    } else {
        format!("\nPAGE STRUCTURE:\n{outline}\n")
    };

    let forms_section_str = if forms_section.is_empty() {
        "  (none)".to_string()
    } else {
        forms_section
    };

    let outputs_str = if outputs.is_empty() {
        "(none)".to_string()
    } else {
        format!("  {outputs}")
    };

    // Build accumulated context section (empty string on first page)
    let context_section = match context {
        Some(ctx) => {
            let summary = ctx.build_context_summary();
            if summary.is_empty() {
                String::new()
            } else {
                format!("\nACCUMULATED CONTEXT FROM PREVIOUS PAGES:\n{summary}\n")
            }
        }
        None => String::new(),
    };

    format!(
        r#"Analyze this web page DOM structure. Use structural context to understand the page deeply.

URL: {url}
Title: {title}
{outline_section}{context_section}
FORMS:
{forms_section_str}

VISIBLE TEXT / OUTPUTS:
{outputs_str}

Return JSON only:
{{
  "purpose": "one sentence: what this page does",
  "domain": "free-form application domain (e.g. 'SIM card provisioning step 2', 'patient intake form', 'HR leave request')",
  "layout_description": "brief description of page structure from headings/sections",
  "field_values": {{"FieldLabel": "suggested_value"}},
  "field_analyses": [
    {{
      "label": "field label",
      "context_clues": "what section/group/help text reveals about this field",
      "validation_hint": "any visible validation rule",
      "suggested_value": "realistic domain-appropriate value",
      "negative_values": ["value that triggers error"]
    }}
  ],
  "success_indicators": ["text expected after success"],
  "error_indicators": ["text expected after failure"],
  "test_scenarios": [
    {{"name": "scenario", "type": "happy_path|negative|boundary", "description": "what to test"}}
  ]
}}
Return ONLY valid JSON. Do NOT include category enum — use domain as free-form string."#
    )
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

If forms are present, suggest test values and outcome indicators:
"field_values":{{"Email":"test@example.com","Password":"pass123"}}
"success_indicators":["welcome","dashboard"]
"error_indicators":["invalid","incorrect","failed"]

Example:
Page with title "Sign In", form with email+password fields, "Log In" button.
{{"purpose":"User login page","category":"Login","field_values":{{"Email":"user@test.com","Password":"Test123!"}},"success_indicators":["welcome"],"error_indicators":["invalid password"]}}

Return ONLY JSON:{{"purpose":"...","category":"...","field_values":{{...}},"success_indicators":[...],"error_indicators":[...]}}"#,
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
        self.analyze_with_context(screen, &AppContext::default())
    }

    fn analyze_with_context(
        &self,
        screen: &ScreenState,
        ctx: &AppContext,
    ) -> Result<PageModel, AgentError> {
        // Use rich prompt with accumulated context (Phase 15) for deeper DOM understanding
        let context_arg = if ctx.is_empty() { None } else { Some(ctx) };
        let prompt = build_rich_page_prompt(screen, context_arg);

        // Try to get LLM-provided enrichments
        let llm_parsed = match self.backend.infer_text(&prompt) {
            Some(response) => try_parse_llm_response(&response),
            None => None,
        };

        let (llm_purpose, llm_domain, llm_field_values, llm_success, llm_errors,
             llm_layout_desc, llm_field_analyses, llm_test_scenarios) =
            match llm_parsed {
                Some(parsed) => {
                    let purpose = parsed.purpose;
                    // Prefer explicit domain field; fall back to category for old-format responses
                    let domain = parsed
                        .domain
                        .filter(|d| !d.is_empty())
                        .or_else(|| parsed.category.filter(|c| !c.is_empty()));
                    let field_values = parsed.field_values;
                    let success = parsed.success_indicators;
                    let errors = parsed.error_indicators;
                    let layout = parsed.layout_description;
                    let fa = parsed.field_analyses;
                    let ts = parsed.test_scenarios;
                    (purpose, domain, field_values, success, errors, layout, fa, ts)
                }
                None => (None, None, None, None, None, None, None, None),
            };

        // Build the base model deterministically (always succeeds)
        let mut model = MockPageAnalyzer.analyze(screen)?;

        // Enrich with LLM insights where available
        if let Some(purpose) = llm_purpose {
            if !purpose.is_empty() {
                model.purpose = purpose;
            }
        }
        // Update domain from LLM if provided
        if let Some(domain) = llm_domain {
            model.domain = domain;
        }
        // Populate new rich prompt fields
        if let Some(layout) = llm_layout_desc {
            if !layout.is_empty() {
                model.layout_description = Some(layout);
            }
        }
        // Parse field_analyses from raw JSON values
        if let Some(fa_values) = llm_field_analyses {
            model.field_analyses = fa_values
                .into_iter()
                .filter_map(|v| serde_json::from_value::<crate::agent::page_model::FieldAnalysis>(v).ok())
                .collect();
        }
        // Parse test_scenarios from raw JSON values
        if let Some(ts_values) = llm_test_scenarios {
            model.suggested_test_scenarios = ts_values
                .into_iter()
                .filter_map(|v| serde_json::from_value::<crate::agent::page_model::TestScenario>(v).ok())
                .collect();
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

        // Merge LLM-provided outcome indicators into ExpectedOutcome
        // LLM indicators extend (not replace) the deterministically-inferred ones
        if let Some(success) = llm_success {
            if !success.is_empty() {
                for form in &mut model.forms {
                    for s in &success {
                        if !form.expected_outcome.success_text.contains(s) {
                            form.expected_outcome.success_text.push(s.clone());
                        }
                    }
                }
                for s in &success {
                    if !model.expected_outcome.success_text.contains(s) {
                        model.expected_outcome.success_text.push(s.clone());
                    }
                }
            }
        }
        if let Some(errors) = llm_errors {
            if !errors.is_empty() {
                for form in &mut model.forms {
                    for e in &errors {
                        if !form.expected_outcome.error_indicators.contains(e) {
                            form.expected_outcome.error_indicators.push(e.clone());
                        }
                    }
                }
                for e in &errors {
                    if !model.expected_outcome.error_indicators.contains(e) {
                        model.expected_outcome.error_indicators.push(e.clone());
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
