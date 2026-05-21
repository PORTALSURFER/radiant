use crate::{
    application::{StateAction, ViewNode, button, column, row, stack, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

const DROPDOWN_MENU_PADDING: f32 = 4.0;

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
pub struct DropdownBuilder<Message> {
    selected_label: String,
    open: bool,
    toggle_message: Option<Message>,
    options: Vec<DropdownOption<Message>>,
}

impl<Message> DropdownBuilder<Message> {
    /// Emit the supplied host message when the collapsed control is activated.
    pub fn toggle_message(mut self, message: Message) -> Self {
        self.toggle_message = Some(message);
        self
    }

    /// Add one selectable option.
    pub fn option(mut self, label: impl Into<String>, selected: bool, message: Message) -> Self {
        self.options
            .push(DropdownOption::new(label, selected, message));
        self
    }

    /// Add several selectable options.
    pub fn options(mut self, options: impl IntoIterator<Item = DropdownOption<Message>>) -> Self {
        self.options.extend(options);
        self
    }

    /// Build this dropdown from the accumulated fields.
    pub fn build(self) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        dropdown_from_parts(DropdownParts {
            selected_label: self.selected_label,
            open: self.open,
            toggle_message: self
                .toggle_message
                .expect("dropdown toggle_message must be set before build"),
            options: self.options,
        })
    }
}

/// Build a generic dropdown.
pub fn dropdown<Message>(
    selected_label: impl Into<String>,
    open: bool,
) -> DropdownBuilder<Message> {
    DropdownBuilder {
        selected_label: selected_label.into(),
        open,
        toggle_message: None,
        options: Vec::new(),
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

/// Return the overlay menu height for a dropdown option list.
pub fn dropdown_menu_height(option_count: usize) -> f32 {
    option_count as f32 * 22.0
        + option_count.saturating_sub(1) as f32 * 3.0
        + DROPDOWN_MENU_PADDING * 2.0
}

/// Build only the expanded option menu for a dropdown.
pub fn dropdown_menu<Message>(options: Vec<DropdownOption<Message>>) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let option_count = options.len();
    column(
        options
            .into_iter()
            .enumerate()
            .map(|(index, option)| dropdown_option_button(index, option)),
    )
    .key("dropdown-menu")
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Strong,
    })
    .padding(DROPDOWN_MENU_PADDING)
    .spacing(3.0)
    .fill_width()
    .height(dropdown_menu_height(option_count))
}

/// Build a dropdown menu overlay positioned inside a caller-owned stack layer.
pub fn dropdown_menu_overlay<Message>(
    x: f32,
    y: f32,
    width: Option<f32>,
    options: Vec<DropdownOption<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let menu_height = dropdown_menu_height(options.len());
    let mut menu = dropdown_menu(options).height(menu_height);
    if let Some(width) = width {
        menu = menu.width(width);
    } else {
        menu = menu.fill_width();
    }
    column([
        dropdown_overlay_gap().height(y.max(0.0)).fill_width(),
        row([
            dropdown_overlay_gap().width(x.max(0.0)).height(1.0),
            menu,
            dropdown_overlay_gap().fill_width().height(1.0),
        ])
        .fill_width()
        .height(menu_height),
        dropdown_overlay_gap().fill_height().fill_width(),
    ])
    .key("dropdown-menu-overlay")
    .fill()
}

fn dropdown_overlay_gap<Message: 'static>() -> ViewNode<Message> {
    text("")
}

fn dropdown_option_button<Message>(
    index: usize,
    option: DropdownOption<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button(option.label)
        .message(option.message)
        .key(format!("dropdown-option-{index}"))
        .style(WidgetStyle {
            tone: if option.selected {
                WidgetTone::Accent
            } else {
                WidgetTone::Neutral
            },
            prominence: if option.selected {
                WidgetProminence::Strong
            } else {
                WidgetProminence::Subtle
            },
        })
        .fill_width()
        .height(22.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Toggle,
        Select(&'static str),
    }

    #[test]
    fn dropdown_height_tracks_expanded_options() {
        assert_eq!(dropdown_height(false, 3), 24.0);
        assert_eq!(dropdown_height(true, 3), 24.0);
        assert_eq!(dropdown_menu_height(3), 80.0);
    }

    #[test]
    fn dropdown_builder_accepts_toggle_and_options() {
        let _view = dropdown("WASAPI", true)
            .toggle_message(Message::Toggle)
            .option("System default", false, Message::Select("default"))
            .option("WASAPI", true, Message::Select("wasapi"))
            .build();
    }
}
