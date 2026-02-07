use serde::Deserialize;

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
