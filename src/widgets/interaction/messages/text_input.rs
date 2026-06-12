/// Message emitted by a reusable text-input primitive.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextInputMessage {
    /// The visible text value changed immediately.
    Changed {
        /// Updated single-line text value.
        value: String,
    },
    /// The current text value was committed by submit intent.
    Submitted {
        /// Submitted single-line text value.
        value: String,
    },
    /// The current text value requested host-defined completion.
    CompletionRequested {
        /// Current single-line text value at completion time.
        value: String,
    },
}

/// High-level kind for a reusable text-input message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextInputMessageKind {
    /// The visible text value changed immediately.
    Changed,
    /// The current text value was committed by submit intent.
    Submitted,
    /// The current text value requested host-defined completion.
    CompletionRequested,
}

/// Borrowed parts of a text-input message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextInputMessageParts<'a> {
    /// High-level message kind.
    pub kind: TextInputMessageKind,
    /// Text value carried by the message.
    pub value: &'a str,
}

impl TextInputMessage {
    /// Return the high-level kind of this input event.
    pub fn kind(&self) -> TextInputMessageKind {
        match self {
            Self::Changed { .. } => TextInputMessageKind::Changed,
            Self::Submitted { .. } => TextInputMessageKind::Submitted,
            Self::CompletionRequested { .. } => TextInputMessageKind::CompletionRequested,
        }
    }

    /// Return borrowed parts for reducers that need both kind and value.
    pub fn parts(&self) -> TextInputMessageParts<'_> {
        TextInputMessageParts {
            kind: self.kind(),
            value: self.value(),
        }
    }

    /// Return the text value carried by this input event.
    pub fn value(&self) -> &str {
        match self {
            Self::Changed { value }
            | Self::Submitted { value }
            | Self::CompletionRequested { value } => value.as_str(),
        }
    }

    /// Consume this input event and return its text value.
    pub fn into_value(self) -> String {
        match self {
            Self::Changed { value }
            | Self::Submitted { value }
            | Self::CompletionRequested { value } => value,
        }
    }

    /// Return whether this event is an immediate edit.
    pub fn is_changed(&self) -> bool {
        matches!(self, Self::Changed { .. })
    }

    /// Return whether this event is a submit/commit intent.
    pub fn is_submitted(&self) -> bool {
        matches!(self, Self::Submitted { .. })
    }

    /// Return whether this event requests host-defined completion.
    pub fn is_completion_requested(&self) -> bool {
        matches!(self, Self::CompletionRequested { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::{TextInputMessage, TextInputMessageKind};

    #[test]
    fn text_input_message_parts_expose_kind_and_borrowed_value() {
        let message = TextInputMessage::Submitted {
            value: String::from("crate"),
        };

        let parts = message.parts();

        assert_eq!(parts.kind, TextInputMessageKind::Submitted);
        assert_eq!(parts.value, "crate");
        assert_eq!(message.value(), "crate");
    }

    #[test]
    fn text_input_message_kind_classifies_each_variant() {
        assert_eq!(
            TextInputMessage::Changed {
                value: String::from("a")
            }
            .kind(),
            TextInputMessageKind::Changed
        );
        assert_eq!(
            TextInputMessage::CompletionRequested {
                value: String::from("ab")
            }
            .kind(),
            TextInputMessageKind::CompletionRequested
        );
    }
}
