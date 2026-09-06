//! Bounded multiline editor document and widget contracts.
mod document;
pub use document::{
    MAX_TEXT_EDITOR_BYTES, MAX_TEXT_EDITOR_GRAPHEMES, TextEditorCompositionDelta, TextEditorDelta,
    TextEditorDocument, TextEditorEdit, TextEditorError, TextEditorRevision, TextEditorSelection,
    TextEditorSnapshot,
};
