use crate::screen::screen_model::{Form, OutputRegion, ScreenElement, Volatility};

#[derive(Debug, Clone)]
pub struct IdentifiedElement {
    pub id: String,             // stable semantic identity
    pub element: ScreenElement, // original screen element
    pub scope: String,          // "form:<id>" or "screen"

    pub region: OutputRegion,
    pub volatility: Volatility,
}

pub fn form_key(form: &Form) -> String {
    format!(
        "{}::{:?}::{}::{}",
        form.id,
        form.intent,
        form.inputs.len(),
        form.actions.len()
    )
}

pub fn element_key(el: &ScreenElement) -> String {
    el.label.clone().unwrap_or_default()
}
