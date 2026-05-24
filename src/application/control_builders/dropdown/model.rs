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
    pub selected: bool,
    /// Host message emitted when the option is selected.
    pub message: Message,
}

impl<Message> DropdownOption<Message> {
    /// Build one dropdown option from named parts.
    pub fn from_parts(parts: DropdownOptionParts<Message>) -> Self {
        Self {
            label: parts.label,
            selected: parts.selected,
            message: parts.message,
        }
    }

    /// Build one dropdown option.
    pub fn new(label: impl Into<String>, selected: bool, message: Message) -> Self {
        Self::from_parts(DropdownOptionParts {
            label: label.into(),
            selected,
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
