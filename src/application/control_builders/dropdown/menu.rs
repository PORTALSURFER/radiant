use crate::{
    application::{ViewNode, button, column, row, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

use super::DropdownOption;

const DROPDOWN_MENU_PADDING: f32 = 4.0;

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
