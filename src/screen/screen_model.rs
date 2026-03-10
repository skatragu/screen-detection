use serde::{Deserialize, Serialize};

/// Option value for `<select>` elements.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomElement {
    pub tag: String,
    pub text: Option<String>,
    pub role: Option<String>,
    pub r#type: Option<String>,
    #[serde(rename = "ariaLabel")]
    pub aria_label: Option<String>,
    pub disabled: bool,
    pub required: bool,
    #[serde(rename = "formId")]
    pub form_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<SelectOption>>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub minlength: Option<u32>,
    #[serde(default)]
    pub maxlength: Option<u32>,
    #[serde(default)]
    pub min: Option<String>,
    #[serde(default)]
    pub max: Option<String>,
    #[serde(default)]
    pub readonly: bool,
    // Rich DOM context fields added in Phase 15
    #[serde(default)]
    pub heading_level: Option<u8>,
    #[serde(default)]
    pub autocomplete: Option<String>,
    #[serde(default)]
    pub inputmode: Option<String>,
    #[serde(default)]
    pub title_attr: Option<String>,
    #[serde(default)]
    pub form_action: Option<String>,
    #[serde(default)]
    pub form_method: Option<String>,
    #[serde(rename = "aria_describedby_text", default)]
    pub aria_describedby_text: Option<String>,
    #[serde(default)]
    pub aria_invalid: bool,
    #[serde(default)]
    pub aria_required: bool,
    #[serde(default)]
    pub associated_label_text: Option<String>,
    #[serde(default)]
    pub fieldset_legend: Option<String>,
    #[serde(default)]
    pub section_heading: Option<String>,
    #[serde(default)]
    pub nearby_help_text: Option<String>,
    #[serde(default)]
    pub semantic_section: Option<String>,
    #[serde(default)]
    pub visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HeadingEntry {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LandmarkEntry {
    pub tag: String,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StructuralOutline {
    #[serde(default)]
    pub headings: Vec<HeadingEntry>,
    #[serde(default)]
    pub landmarks: Vec<LandmarkEntry>,
}

#[derive(Debug)]
pub struct ScreenSemantics {
    pub forms: Vec<Form>,
    pub standalone_actions: Vec<ScreenElement>,
    pub outputs: Vec<ScreenElement>,
    pub primary_action: Option<ScreenElement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenElement {
    pub label: Option<String>,
    pub kind: ElementKind,
    pub tag: Option<String>,
    pub role: Option<String>,
    pub input_type: Option<String>,
    pub required: bool,
    pub placeholder: Option<String>,
    pub id: Option<String>,
    pub href: Option<String>,
    pub options: Option<Vec<SelectOption>>,
    pub name: Option<String>,
    pub value: Option<String>,
    pub maxlength: Option<u32>,
    pub minlength: Option<u32>,
    pub readonly: bool,
    // Rich DOM context fields for AI prompt building
    pub fieldset_legend: Option<String>,
    pub section_heading: Option<String>,
    pub nearby_help_text: Option<String>,
    pub autocomplete: Option<String>,
    pub aria_describedby_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Form {
    pub id: String,
    pub inputs: Vec<ScreenElement>,
    pub actions: Vec<ScreenElement>,
    pub primary_action: Option<ScreenElement>,
    pub intent: Option<FormIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementKind {
    Input,
    Action,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormIntent {
    pub label: String,
    pub confidence: f32,
    pub signals: Vec<IntentSignal>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntentSignal {
    Keyword(String),
    InputType(String),
    ActionLabel(String),
    Structure(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputRegion {
    Header,
    Main,
    Results,
    Footer,
    Modal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Volatility {
    Stable,
    Volatile,
}
