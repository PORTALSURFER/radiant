use crate::{
    application::{ViewNode, button, stack},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

mod menu;
mod model;

#[cfg(test)]
#[path = "dropdown/tests.rs"]
mod tests;

pub use menu::{
    DropdownMenuOverlayBelowParts, anchored_dropdown_menu_popover, dropdown_menu,
    dropdown_menu_height, dropdown_menu_overlay, dropdown_menu_overlay_below,
    dropdown_menu_overlay_below_from_parts, dropdown_menu_overlay_below_labeled_control,
    dropdown_menu_overlay_below_stacked_labeled_control, dropdown_menu_overlay_below_trigger,
    dropdown_trigger_height,
};
pub use model::{
    DropdownOption, DropdownOptionParts, DropdownOptionSelection, DropdownParts,
    DropdownTriggerParts,
};

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

/// Builder for a dropdown trigger whose menu is projected elsewhere.
pub struct DropdownTriggerBuilderNeedsToggle {
    selected_label: String,
    open: bool,
}

/// Builder for a dropdown trigger after the required toggle message is set.
pub struct DropdownTriggerBuilder<Message> {
    selected_label: String,
    open: bool,
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

    fn add_option_from_parts(&mut self, parts: DropdownOptionParts<Message>) {
        self.options.push(DropdownOption::from_parts(parts));
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
        self.state
            .options
            .push(DropdownOption::new(label, selected, message));
        self
    }

    /// Add one selectable option with an explicit selection state before
    /// assigning the required toggle message.
    pub fn option_with_selection(
        mut self,
        label: impl Into<String>,
        selection: DropdownOptionSelection,
        message: Message,
    ) -> Self {
        self.state
            .options
            .push(DropdownOption::from_selection(label, selection, message));
        self
    }

    /// Add one selectable option from named fields before assigning the required toggle message.
    pub fn option_from_parts(mut self, parts: DropdownOptionParts<Message>) -> Self {
        self.state.add_option_from_parts(parts);
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
        self.state
            .options
            .push(DropdownOption::new(label, selected, message));
        self
    }

    /// Add one selectable option with an explicit selection state.
    pub fn option_with_selection(
        mut self,
        label: impl Into<String>,
        selection: DropdownOptionSelection,
        message: Message,
    ) -> Self {
        self.state
            .options
            .push(DropdownOption::from_selection(label, selection, message));
        self
    }

    /// Add one selectable option from named fields.
    pub fn option_from_parts(mut self, parts: DropdownOptionParts<Message>) -> Self {
        self.state.add_option_from_parts(parts);
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

impl DropdownTriggerBuilderNeedsToggle {
    /// Emit the supplied host message when the trigger is activated.
    pub fn toggle_message<Message>(self, message: Message) -> DropdownTriggerBuilder<Message> {
        DropdownTriggerBuilder {
            selected_label: self.selected_label,
            open: self.open,
            toggle_message: message,
        }
    }
}

impl<Message> DropdownTriggerBuilder<Message> {
    /// Build this standalone dropdown trigger.
    pub fn build(self) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        dropdown_trigger_from_parts(DropdownTriggerParts {
            selected_label: self.selected_label,
            open: self.open,
            toggle_message: self.toggle_message,
        })
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

/// Build only the trigger for a dropdown whose menu is rendered by the host as
/// an external overlay.
pub fn dropdown_trigger(
    selected_label: impl Into<String>,
    open: bool,
) -> DropdownTriggerBuilderNeedsToggle {
    DropdownTriggerBuilderNeedsToggle {
        selected_label: selected_label.into(),
        open,
    }
}

/// Build a generic dropdown from named parts.
pub fn dropdown_from_parts<Message>(parts: DropdownParts<Message>) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let toggle = dropdown_trigger_from_parts(DropdownTriggerParts {
        selected_label: parts.selected_label,
        open: parts.open,
        toggle_message: parts.toggle_message,
    })
    .key("dropdown-toggle");
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

/// Build only a dropdown trigger from named parts.
pub fn dropdown_trigger_from_parts<Message>(
    parts: DropdownTriggerParts<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let mut trigger = button(format!("{}  v", parts.selected_label));
    if parts.open {
        trigger = trigger.style(WidgetStyle::new(
            WidgetTone::Accent,
            WidgetProminence::Subtle,
        ));
    }
    trigger
        .message(parts.toggle_message)
        .key("dropdown-trigger")
        .fill_width()
        .height(24.0)
}

/// Return the normal-flow height for a dropdown toggle.
pub fn dropdown_height(_open: bool, _option_count: usize) -> f32 {
    dropdown_trigger_height()
}
