/// Backend-neutral single-line text editing commands.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextEditCommand {
    /// Move the caret one logical character left.
    MoveLeft {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret one logical character right.
    MoveRight {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret one word boundary left.
    MoveWordLeft {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret one word boundary right.
    MoveWordRight {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret to the start of the value.
    MoveHome {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret to the end of the value.
    MoveEnd {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Select the full text value.
    SelectAll,
    /// Insert or paste a text payload at the current selection.
    InsertText(String),
    /// Delete the selected range or previous character.
    Backspace,
    /// Delete the selected range or next character.
    Delete,
    /// Delete the selected range or previous word boundary span.
    DeleteWordLeft,
    /// Delete the selected range or next word boundary span.
    DeleteWordRight,
    /// Delete the selected range for a cut operation.
    CutSelection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_edit_commands_preserve_selection_intent() {
        assert_ne!(
            TextEditCommand::MoveLeft {
                extend_selection: true
            },
            TextEditCommand::MoveLeft {
                extend_selection: false
            }
        );
    }
}
