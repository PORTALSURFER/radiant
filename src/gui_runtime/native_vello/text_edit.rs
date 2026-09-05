//! Single-line text editing helpers shared by native runtime text fields.

mod boundary;
mod layout;
mod state;

#[cfg(test)]
pub(super) use layout::build_text_field_layout;
pub(super) use layout::{
    TextFieldLayoutState, build_text_field_layout_aligned, build_text_field_layout_from_snapshot,
};
pub(super) use state::SingleLineTextEditorState;

#[cfg(test)]
mod tests;
