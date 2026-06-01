use crate::runtime::PaintText;

mod editing;
mod navigation;
mod selection;
mod word_boundary;

/// Immutable public properties for a reusable single-line text input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextInputProps {
    /// Optional placeholder shown when the current value is empty.
    pub placeholder: Option<PaintText>,
    /// Optional inline completion text painted after the current value.
    pub completion_suffix: Option<PaintText>,
    /// Whether Enter should emit a submit message instead of inserting text.
    pub submit_on_enter: bool,
    /// Optional maximum number of Unicode scalar values accepted by the field.
    pub character_limit: Option<usize>,
    /// Visual chrome treatment for the input bounds.
    pub chrome: TextInputChrome,
}

/// Visual chrome treatment for a reusable single-line text input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextInputChrome {
    /// Paint the normal input fill, border, and focus outline.
    #[default]
    Full,
    /// Paint only a subtle baseline and focus underline.
    Underline,
}

/// Mutable interaction state for a reusable single-line text input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextInputState {
    /// Current single-line text value.
    pub value: String,
    /// Caret position measured in Unicode scalar values from the start.
    pub caret: usize,
    /// Selection anchor measured in Unicode scalar values from the start.
    pub selection_anchor: usize,
}

/// Result of applying an editing command to [`TextInputState`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextInputEditResult {
    /// The text value changed and the host should publish a changed value.
    pub value_changed: bool,
    /// The caret or selection changed without necessarily changing the value.
    pub selection_changed: bool,
}

impl TextInputState {
    /// Create editable single-line text state with a collapsed caret at the end.
    pub fn from_value(value: String) -> Self {
        let caret = value.chars().count();
        Self {
            value,
            caret,
            selection_anchor: caret,
        }
    }

    /// Return the current value length in Unicode scalar values.
    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }
}
