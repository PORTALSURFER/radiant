use crate::application::{StateAction, ViewNode, button, stack};
use std::sync::Arc;

mod menu;

#[cfg(test)]
#[path = "dropdown/tests.rs"]
mod tests;

pub use menu::{dropdown_menu, dropdown_menu_height, dropdown_menu_overlay};

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

impl<Message> DropdownOption<Message> {
    /// Build one dropdown option.
    pub fn new(label: impl Into<String>, selected: bool, message: Message) -> Self {
        Self {
            label: label.into(),
            selected,
            message,
        }
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

/// Builder for generic dropdown controls.
pub struct DropdownBuilderNeedsToggle<Message> {
    state: DropdownBuilderState<Message>,
}

struct DropdownBuilderState<Message> {
    selected_label: String,
    open: bool,
    options: Vec<DropdownOption<Message>>,
}

/// Builder for generic dropdown controls after the required toggle message is set.
pub struct DropdownBuilder<Message> {
    state: DropdownBuilderState<Message>,
    toggle_message: Message,
}

impl<Message> DropdownBuilderState<Message> {
    fn new(selected_label: impl Into<String>, open: bool) -> Self {
        Self {
            selected_label: selected_label.into(),
            open,
            options: Vec::new(),
        }
    }

    fn add_option(&mut self, label: impl Into<String>, selected: bool, message: Message) {
        self.options
            .push(DropdownOption::new(label, selected, message));
    }

    fn add_options(&mut self, options: impl IntoIterator<Item = DropdownOption<Message>>) {
        self.options.extend(options);
    }

    fn into_parts(self, toggle_message: Message) -> DropdownParts<Message> {
        DropdownParts {
            selected_label: self.selected_label,
            open: self.open,
            toggle_message,
            options: self.options,
        }
    }
}

impl<Message> DropdownBuilderNeedsToggle<Message> {
    /// Emit the supplied host message when the collapsed control is activated.
    pub fn toggle_message(self, message: Message) -> DropdownBuilder<Message> {
        DropdownBuilder {
            state: self.state,
            toggle_message: message,
        }
    }

    /// Add one selectable option before assigning the required toggle message.
    pub fn option(mut self, label: impl Into<String>, selected: bool, message: Message) -> Self {
        self.state.add_option(label, selected, message);
        self
    }

    /// Add several selectable options before assigning the required toggle message.
    pub fn options(mut self, options: impl IntoIterator<Item = DropdownOption<Message>>) -> Self {
        self.state.add_options(options);
        self
    }
}

impl<Message> DropdownBuilder<Message> {
    /// Add one selectable option.
    pub fn option(mut self, label: impl Into<String>, selected: bool, message: Message) -> Self {
        self.state.add_option(label, selected, message);
        self
    }

    /// Add several selectable options.
    pub fn options(mut self, options: impl IntoIterator<Item = DropdownOption<Message>>) -> Self {
        self.state.add_options(options);
        self
    }

    /// Build this dropdown from the accumulated fields.
    pub fn build(self) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        dropdown_from_parts(self.state.into_parts(self.toggle_message))
    }
}

/// Build a generic dropdown.
pub fn dropdown<Message>(
    selected_label: impl Into<String>,
    open: bool,
) -> DropdownBuilderNeedsToggle<Message> {
    DropdownBuilderNeedsToggle {
        state: DropdownBuilderState::new(selected_label, open),
    }
}

/// Build a generic dropdown from named parts.
pub fn dropdown_from_parts<Message>(parts: DropdownParts<Message>) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let toggle = button(format!("{}  v", parts.selected_label))
        .message(parts.toggle_message)
        .key("dropdown-toggle")
        .fill_width()
        .height(24.0);
    if parts.open {
        stack([
            toggle,
            dropdown_menu_overlay(0.0, 27.0, None, parts.options),
        ])
        .key("dropdown")
        .fill_width()
        .height(dropdown_height(parts.open, 0))
    } else {
        toggle.key("dropdown")
    }
}

/// Return the normal-flow height for a dropdown toggle.
pub fn dropdown_height(_open: bool, _option_count: usize) -> f32 {
    24.0
}

/// Build a state-mutating dropdown option.
pub fn dropdown_option<State: 'static>(
    label: impl Into<String>,
    selected: bool,
    apply: impl Fn(&mut State) + Send + Sync + 'static,
) -> DropdownOption<StateAction<State>> {
    DropdownOption::new(label, selected, StateAction::new(apply))
}

/// Build a state-mutating dropdown.
pub fn state_dropdown<State: 'static>(
    selected_label: impl Into<String>,
    open: bool,
    toggle: impl Fn(&mut State) + Send + Sync + 'static,
    options: impl IntoIterator<Item = DropdownOption<StateAction<State>>>,
) -> ViewNode<StateAction<State>> {
    let toggle = Arc::new(toggle);
    dropdown_from_parts(DropdownParts {
        selected_label: selected_label.into(),
        open,
        toggle_message: StateAction::new(move |state| toggle(state)),
        options: options.into_iter().collect(),
    })
}
