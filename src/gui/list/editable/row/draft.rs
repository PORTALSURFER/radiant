/// Focus policy for an inline editable tree draft input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditableTreeInputFocus {
    /// Leave the draft input unfocused.
    #[default]
    Blurred,
    /// Give the draft input keyboard focus.
    Focused,
}

impl EditableTreeInputFocus {
    /// Build focus policy from compatibility flags.
    pub const fn from_focused(focused: bool) -> Self {
        match focused {
            true => Self::Focused,
            false => Self::Blurred,
        }
    }

    const fn is_focused(self) -> bool {
        matches!(self, Self::Focused)
    }
}

pub(super) enum EditableTreeRowDraftSelection {
    KeepCaret,
    SelectAllOnFocus,
}

impl EditableTreeRowDraftSelection {
    fn select_all_on_focus(&self) -> bool {
        matches!(self, Self::SelectAllOnFocus)
    }
}

/// Explicit input parts used to build inline editable tree draft rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditableTreeDraftInputParts {
    /// Current input value.
    pub value: String,
    /// Placeholder shown while the input is empty.
    pub placeholder: String,
    /// Validation error shown for the draft input.
    pub error: Option<String>,
    /// Whether the draft input should own keyboard focus.
    pub focus: EditableTreeInputFocus,
}

/// Inline editor state for create and rename draft rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditableTreeRowInput {
    /// Editable input value for inline draft rows.
    pub value: Option<String>,
    /// Placeholder text for inline draft rows.
    pub placeholder: Option<String>,
    /// Validation error for inline draft rows.
    pub error: Option<String>,
    /// Whether the inline draft input should own keyboard focus.
    pub focused: bool,
    /// Whether the next focus transition should select the full input text once.
    pub select_all_on_focus: bool,
}

impl EditableTreeRowInput {
    pub(super) fn draft(
        parts: EditableTreeDraftInputParts,
        selection: EditableTreeRowDraftSelection,
    ) -> Self {
        Self {
            value: Some(parts.value),
            placeholder: Some(parts.placeholder),
            error: parts.error,
            focused: parts.focus.is_focused(),
            select_all_on_focus: selection.select_all_on_focus(),
        }
    }
}

pub(super) fn draft_input_parts(
    value: impl Into<String>,
    placeholder: impl Into<String>,
    error: Option<String>,
    focused: bool,
) -> EditableTreeDraftInputParts {
    EditableTreeDraftInputParts {
        value: value.into(),
        placeholder: placeholder.into(),
        error,
        focus: EditableTreeInputFocus::from_focused(focused),
    }
}
