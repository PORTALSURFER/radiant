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

    /// Build one dropdown option.
    pub fn new(label: impl Into<String>, selected: bool, message: Message) -> Self {
        Self::from_parts(DropdownOptionParts {
            label: label.into(),
            selection: DropdownOptionSelection::from_selected(selected),
            message,
        })
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
