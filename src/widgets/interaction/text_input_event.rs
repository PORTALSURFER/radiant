use std::fmt;

use super::{TextEditGrouping, TextInputMessage};

/// One opt-in transient grouping event from a single-line text input.
///
/// Applications retain durable history. This event only describes local edit
/// continuity and may carry the legacy message that changed the value.
#[derive(Clone)]
pub struct TextInputEditEvent {
    /// Legacy value/submission message, when this input produced one.
    pub legacy_message: Option<TextInputMessage>,
    /// Text-free grouping lifecycle metadata.
    pub grouping: TextEditGrouping,
    /// Current caret measured in Unicode scalar values.
    pub caret: usize,
    /// Current selection anchor measured in Unicode scalar values.
    pub selection_anchor: usize,
    /// Whether native composition is currently active.
    pub composing: bool,
}

impl fmt::Debug for TextInputEditEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextInputEditEvent")
            .field(
                "legacy_message",
                &self.legacy_message.as_ref().map(TextInputMessage::kind),
            )
            .field("grouping", &self.grouping)
            .field("caret", &self.caret)
            .field("selection_anchor", &self.selection_anchor)
            .field("composing", &self.composing)
            .finish()
    }
}
