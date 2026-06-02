/// Explicit selection state for one generic dropdown option.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DropdownOptionSelection {
    /// The option is not the current value.
    #[default]
    Unselected,
    /// The option represents the current value.
    Selected,
}

impl DropdownOptionSelection {
    /// Build selection state from compatibility flags.
    pub const fn from_selected(selected: bool) -> Self {
        match selected {
            true => Self::Selected,
            false => Self::Unselected,
        }
    }

    pub(super) const fn is_selected(self) -> bool {
        matches!(self, Self::Selected)
    }
}

/// One selectable item in a generic dropdown control.
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownOption<Message> {
    /// Visible option label.
    pub label: String,
    /// Whether this option represents the current value.
    pub selected: bool,
    /// Host message emitted when the option is selected.
    pub message: Message,
}

/// Named construction fields for one generic dropdown option.
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownOptionParts<Message> {
    /// Visible option label.
    pub label: String,
    /// Whether this option represents the current value.
    pub selection: DropdownOptionSelection,
    /// Host message emitted when the option is selected.
    pub message: Message,
}

impl<Message> DropdownOption<Message> {
    /// Build one dropdown option from named parts.
    pub fn from_parts(parts: DropdownOptionParts<Message>) -> Self {
        Self {
            label: parts.label,
            selected: parts.selection.is_selected(),
            message: parts.message,
        }
    }

    /// Build one dropdown option from an explicit selection state.
    pub fn from_selection(
        label: impl Into<String>,
        selection: DropdownOptionSelection,
        message: Message,
    ) -> Self {
        Self::from_parts(DropdownOptionParts {
            label: label.into(),
            selection,
            message,
        })
    }

    /// Build one dropdown option that represents the current value.
    pub fn selected(label: impl Into<String>, message: Message) -> Self {
        Self::from_selection(label, DropdownOptionSelection::Selected, message)
    }

    /// Build one dropdown option that does not represent the current value.
    pub fn unselected(label: impl Into<String>, message: Message) -> Self {
        Self::from_selection(label, DropdownOptionSelection::Unselected, message)
    }

    /// Build one dropdown option from a concrete value and current selection.
    pub fn for_value<Value>(
        label: impl Into<String>,
        value: Value,
        selected: &Value,
        message: impl FnOnce(Value) -> Message,
    ) -> Self
    where
        Value: PartialEq,
    {
        let selection = DropdownOptionSelection::from_selected(value == *selected);
        Self::from_selection(label, selection, message(value))
    }

    /// Build one dropdown option from an optional value and current optional selection.
    pub fn for_optional_value<Value>(
        label: impl Into<String>,
        value: Option<Value>,
        selected: Option<&Value>,
        message: impl FnOnce(Option<Value>) -> Message,
    ) -> Self
    where
        Value: PartialEq,
    {
        let selection = DropdownOptionSelection::from_selected(match (value.as_ref(), selected) {
            (Some(value), Some(selected)) => value == selected,
            (None, None) => true,
            _ => false,
        });
        Self::from_selection(label, selection, message(value))
    }

    /// Build one dropdown option.
    pub fn new(label: impl Into<String>, selected: bool, message: Message) -> Self {
        Self::from_selection(
            label,
            DropdownOptionSelection::from_selected(selected),
            message,
        )
    }
}

/// Named construction fields for a generic dropdown.
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownParts<Message> {
    /// Visible label for the currently selected value.
    pub selected_label: String,
    /// Whether the option list is expanded over the toggle.
    pub open: bool,
    /// Host message emitted when the collapsed control is activated.
    pub toggle_message: Message,
    /// Ordered selectable options.
    pub options: Vec<DropdownOption<Message>>,
}

/// Named construction fields for a dropdown trigger whose menu is rendered
/// elsewhere, such as in a stack-level overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownTriggerParts<Message> {
    /// Visible label for the currently selected value.
    pub selected_label: String,
    /// Whether the menu owned by the host is currently open.
    pub open: bool,
    /// Host message emitted when the collapsed control is activated.
    pub toggle_message: Message,
}
