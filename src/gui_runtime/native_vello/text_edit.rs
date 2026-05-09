//! Single-line text editing helpers shared by native runtime text fields.

mod layout;
mod sanitize;
mod state;

pub(super) use layout::build_text_field_layout;
pub(super) use state::SingleLineTextEditorState;

#[cfg(test)]
mod tests;
