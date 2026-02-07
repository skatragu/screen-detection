use std::collections::HashMap;

use crate::{
    screen::screen_model::{Form, ScreenElement},
    state::identity::IdentifiedElement,
};

#[derive(Debug, Clone)]
pub struct ScreenState {
    pub url: Option<String>,
    pub title: String,

    pub forms: Vec<Form>,
    pub standalone_actions: Vec<ScreenElement>,
    pub outputs: Vec<ScreenElement>,
    pub identities: HashMap<String, IdentifiedElement>,
}

#[derive(Debug)]
pub enum Outcome {
    NoChange,
    FormSubmissionSucceeded,
    FormSubmissionFailed { error: Option<String> },
    Navigation,
    Unknown,
}
