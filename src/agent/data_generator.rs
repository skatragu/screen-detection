use crate::agent::ai_model::guess_value;
use crate::agent::app_context::AppContext;
use crate::agent::page_model::{FieldAnalysis, FieldModel};

// ============================================================================
// DataGenerator — intelligent priority chain for test data generation
// ============================================================================

/// Generates test data using a three-level priority chain.
///
/// Priority order (highest to lowest):
/// 1. **AppContext cross-page recall** — if this field label was entered on a previous
///    page, reuse the same value for consistency (e.g. an MSISDN entered on step 1 is
///    automatically recalled on step 3 when asked again).
/// 2. **LLM FieldAnalysis suggestion** — if the LLM analyzed the rich DOM and provided
///    a `suggested_value` for this field, use it (domain-appropriate, help-text-aware).
/// 3. **`guess_value()` deterministic fallback** — the existing 26-pattern label matcher
///    always produces a value; never fails.
///
/// This ensures that values are: (1) consistent across pages, (2) domain-appropriate
/// when the LLM has analyzed the page, and (3) always present even without an LLM.
pub struct DataGenerator<'a> {
    /// The accumulated cross-page context for recall.
    pub context: &'a AppContext,
}

impl<'a> DataGenerator<'a> {
    /// Create a new DataGenerator backed by the given AppContext.
    pub fn new(context: &'a AppContext) -> Self {
        Self { context }
    }

    /// Generate a value for `field` using the priority chain.
    ///
    /// `field_analysis` is the LLM's deep analysis of this specific field (may be `None`
    /// if the LLM did not provide one, or if using `MockPageAnalyzer`).
    ///
    /// The returned value is always a non-empty string. The priority chain ensures
    /// `guess_value()` is always available as a final fallback.
    pub fn generate(&self, field: &FieldModel, field_analysis: Option<&FieldAnalysis>) -> String {
        // 1. Cross-page recall: reuse what was entered on a previous page
        if let Some(recalled) = self.context.recall(&field.label) {
            return recalled.to_string();
        }

        // 2. LLM FieldAnalysis suggested value (domain-aware, help-text-informed)
        if let Some(analysis) = field_analysis {
            if let Some(suggested) = &analysis.suggested_value {
                if !suggested.is_empty() {
                    return suggested.clone();
                }
            }
        }

        // 3. Deterministic fallback: label-pattern matching + input_type hints
        guess_value(&field.label, field.input_type_str())
    }

    /// Generate values for all non-hidden fields in a list.
    ///
    /// Returns a `Vec<(label, value)>` in field order, ready for form filling.
    /// Hidden fields are skipped (they cannot be interacted with).
    ///
    /// `analyses` maps lowercase field labels to their LLM `FieldAnalysis`. Pass an empty
    /// map when no LLM analysis is available.
    pub fn generate_all(
        &self,
        fields: &[FieldModel],
        analyses: &std::collections::HashMap<String, &FieldAnalysis>,
    ) -> Vec<(String, String)> {
        use crate::agent::page_model::FieldType;

        fields
            .iter()
            .filter(|f| f.field_type != FieldType::Hidden)
            .map(|field| {
                let analysis = analyses.get(&field.label.to_lowercase()).copied();
                let value = self.generate(field, analysis);
                (field.label.clone(), value)
            })
            .collect()
    }
}
