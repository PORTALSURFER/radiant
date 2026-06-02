use crate::{
    application::{ViewNode, button, column, labeled_control_control_offset, row, text},
    gui::layout_core::StackedLayoutCursor,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

use super::DropdownOption;

const DROPDOWN_MENU_PADDING: f32 = 4.0;
const DROPDOWN_TRIGGER_HEIGHT: f32 = 24.0;

/// Named construction fields for a dropdown menu overlay anchored below a trigger.
pub struct DropdownMenuOverlayBelowParts<Message> {
    /// Trigger left edge in the owning stack layer.
    pub x: f32,
    /// Trigger top edge in the owning stack layer.
    pub trigger_y: f32,
    /// Trigger height.
    pub trigger_height: f32,
    /// Gap between the trigger bottom and menu top.
    pub gap: f32,
    /// Optional fixed menu width. When omitted the menu fills remaining width.
    pub width: Option<f32>,
    /// Dropdown options shown in the overlay menu.
    pub options: Vec<DropdownOption<Message>>,
}

impl<Message> DropdownMenuOverlayBelowParts<Message> {
    /// Build named dropdown-overlay anchor parts.
    pub fn new(
        x: f32,
        trigger_y: f32,
        trigger_height: f32,
        gap: f32,
        options: Vec<DropdownOption<Message>>,
    ) -> Self {
        Self {
            x,
            trigger_y,
            trigger_height,
            gap,
            width: None,
            options,
        }
    }

    /// Set a fixed menu width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
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

/// Build a dropdown menu overlay positioned below a trigger rectangle.
pub fn dropdown_menu_overlay_below<Message>(
    x: f32,
    trigger_y: f32,
    trigger_height: f32,
    gap: f32,
    width: Option<f32>,
    options: Vec<DropdownOption<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dropdown_menu_overlay_below_from_parts(DropdownMenuOverlayBelowParts {
        x,
        trigger_y,
        trigger_height,
        gap,
        width,
        options,
    })
}

/// Build a dropdown menu overlay below Radiant's standard dropdown trigger.
pub fn dropdown_menu_overlay_below_trigger<Message>(
    x: f32,
    trigger_y: f32,
    gap: f32,
    width: Option<f32>,
    options: Vec<DropdownOption<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dropdown_menu_overlay_below(x, trigger_y, dropdown_trigger_height(), gap, width, options)
}

/// Build a dropdown menu overlay below a standard trigger inside a labeled control.
///
/// `labeled_control_y` is the top edge of the [`crate::application::labeled_control`]
/// row in the caller-owned stack layer.
pub fn dropdown_menu_overlay_below_labeled_control<Message>(
    x: f32,
    labeled_control_y: f32,
    gap: f32,
    width: Option<f32>,
    options: Vec<DropdownOption<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dropdown_menu_overlay_below_trigger(
        x,
        labeled_control_y + labeled_control_control_offset(),
        gap,
        width,
        options,
    )
}

/// Build a dropdown menu overlay below a standard trigger inside the current
/// item of a compact stacked labeled-control layout.
///
/// `stack_y` is the top edge of the stack layer containing the labeled-control
/// rows. `cursor` should point at the labeled-control item that owns the
/// trigger, before advancing past that item.
pub fn dropdown_menu_overlay_below_stacked_labeled_control<Message>(
    x: f32,
    stack_y: f32,
    cursor: StackedLayoutCursor,
    gap: f32,
    width: Option<f32>,
    options: Vec<DropdownOption<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dropdown_menu_overlay_below_labeled_control(x, stack_y + cursor.offset(), gap, width, options)
}

/// Build a dropdown menu overlay below a trigger from named parts.
pub fn dropdown_menu_overlay_below_from_parts<Message>(
    parts: DropdownMenuOverlayBelowParts<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dropdown_menu_overlay(
        parts.x,
        parts.trigger_y.max(0.0) + parts.trigger_height.max(0.0) + parts.gap.max(0.0),
        parts.width,
        parts.options,
    )
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

/// Return the normal-flow height for a standard dropdown trigger.
pub fn dropdown_trigger_height() -> f32 {
    DROPDOWN_TRIGGER_HEIGHT
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
