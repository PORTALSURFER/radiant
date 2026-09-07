use std::fmt;

/// Backend-neutral single-line text editing commands.
#[derive(Clone, PartialEq, Eq, Hash)]
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

impl fmt::Debug for TextEditCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MoveLeft { extend_selection } => formatter
                .debug_struct("MoveLeft")
                .field("extend_selection", extend_selection)
                .finish(),
            Self::MoveRight { extend_selection } => formatter
                .debug_struct("MoveRight")
                .field("extend_selection", extend_selection)
                .finish(),
            Self::MoveWordLeft { extend_selection } => formatter
                .debug_struct("MoveWordLeft")
                .field("extend_selection", extend_selection)
                .finish(),
            Self::MoveWordRight { extend_selection } => formatter
                .debug_struct("MoveWordRight")
                .field("extend_selection", extend_selection)
                .finish(),
            Self::MoveHome { extend_selection } => formatter
                .debug_struct("MoveHome")
                .field("extend_selection", extend_selection)
                .finish(),
            Self::MoveEnd { extend_selection } => formatter
                .debug_struct("MoveEnd")
                .field("extend_selection", extend_selection)
                .finish(),
            Self::SelectAll => formatter.write_str("SelectAll"),
            Self::InsertText(text) => formatter
                .debug_struct("InsertText")
                .field("text_bytes", &text.len())
                .finish(),
            Self::Backspace => formatter.write_str("Backspace"),
            Self::Delete => formatter.write_str("Delete"),
            Self::DeleteWordLeft => formatter.write_str("DeleteWordLeft"),
            Self::DeleteWordRight => formatter.write_str("DeleteWordRight"),
            Self::CutSelection => formatter.write_str("CutSelection"),
        }
    }
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

    #[test]
    fn debug_redacts_inserted_text_but_keeps_variant_identity() {
        let secret = "TEXT_EDIT_DEBUG_SECRET_6a5fb75e";
        let debug = format!("{:?}", TextEditCommand::InsertText(secret.into()));

        assert!(debug.contains("InsertText"));
        assert!(debug.contains("text_bytes"));
        assert!(!debug.contains(secret));
    }
}
