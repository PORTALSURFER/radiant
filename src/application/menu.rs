use crate::{
    application::{StateView, ViewNode, button, column, dismiss_layer, row, stack, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

mod model;

pub use model::{ContextMenuOverlayParts, MenuItem, MenuItemParts, MenuParts};
pub use model::{
    DismissibleContextMenuParts, MenuCommand, MenuCommandParts, MessageContextMenuOverlayParts,
    MessageMenuParts, MessageMenuWidthPolicy,
};

/// Height of the title line in compact Radiant menus.
pub const MENU_TITLE_HEIGHT: f32 = 22.0;
/// Height of one command row in compact Radiant menus.
pub const MENU_ITEM_HEIGHT: f32 = 28.0;
/// Outer padding applied by compact Radiant menus.
pub const MENU_PADDING: f32 = 8.0;
/// Gap between the title and the command list.
pub const MENU_SECTION_SPACING: f32 = 6.0;
/// Gap between command rows.
pub const MENU_ITEM_SPACING: f32 = 4.0;

/// Build a compact vertical menu.
pub fn menu<State: 'static>(
    title: impl Into<String>,
    items: impl IntoIterator<Item = MenuItem<State>>,
) -> StateView<State> {
    menu_from_parts(MenuParts {
        title: title.into(),
        items: items.into_iter().collect(),
    })
}

/// Build a compact vertical menu from named parts.
pub fn menu_from_parts<State: 'static>(parts: MenuParts<State>) -> StateView<State> {
    column([
        text(parts.title).fill_width().height(MENU_TITLE_HEIGHT),
        column(
            parts
                .items
                .into_iter()
                .enumerate()
                .map(|(index, item)| menu_item_button(index, item)),
        )
        .fill_width()
        .spacing(MENU_ITEM_SPACING),
    ])
    .style(WidgetStyle::new(
        WidgetTone::Accent,
        WidgetProminence::Strong,
    ))
    .fill_width()
    .padding(MENU_PADDING)
    .spacing(MENU_SECTION_SPACING)
}

/// Build a compact vertical menu that emits host messages.
pub fn message_menu<Message>(
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    message_menu_from_parts(MessageMenuParts {
        title: title.into(),
        style: WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Strong),
        commands: commands.into_iter().collect(),
    })
}

/// Build a compact message-emitting menu from named parts.
pub fn message_menu_from_parts<Message>(parts: MessageMenuParts<Message>) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    column([
        text(parts.title).fill_width().height(MENU_TITLE_HEIGHT),
        column(
            parts
                .commands
                .into_iter()
                .enumerate()
                .map(|(index, command)| menu_command_button(index, command)),
        )
        .fill_width()
        .spacing(MENU_ITEM_SPACING),
    ])
    .style(parts.style)
    .fill_width()
    .padding(MENU_PADDING)
    .spacing(MENU_SECTION_SPACING)
}

/// Return the normal compact menu height for a known number of items.
pub fn menu_height(item_count: usize) -> f32 {
    MENU_PADDING * 2.0
        + MENU_TITLE_HEIGHT
        + MENU_SECTION_SPACING
        + item_count as f32 * MENU_ITEM_HEIGHT
        + item_count.saturating_sub(1) as f32 * MENU_ITEM_SPACING
}

/// Return the normal compact message-menu height for a known number of commands.
pub fn message_menu_height(command_count: usize) -> f32 {
    menu_height(command_count)
}

/// Build a context menu overlaid at an anchored surface position.
pub fn context_menu_overlay<State: 'static>(
    bounds: Rect,
    anchor: Point,
    size: Vector2,
    title: impl Into<String>,
    items: impl IntoIterator<Item = MenuItem<State>>,
) -> StateView<State> {
    context_menu_overlay_from_parts(ContextMenuOverlayParts {
        bounds,
        anchor,
        size,
        title: title.into(),
        items: items.into_iter().collect(),
    })
}

/// Build a context menu overlay from named parts.
pub fn context_menu_overlay_from_parts<State: 'static>(
    parts: ContextMenuOverlayParts<State>,
) -> StateView<State> {
    let rect = crate::gui::panel::anchored_panel_rect_from_parts(
        crate::gui::panel::AnchoredPanelRectParts {
            bounds: parts.bounds,
            anchor: parts.anchor,
            size: parts.size,
            inset: 0.0,
        },
    );
    let top = (rect.min.y - parts.bounds.min.y).max(0.0);
    let left = (rect.min.x - parts.bounds.min.x).max(0.0);
    column([
        text("").fill_width().height(top),
        row([
            text("").size(left, 1.0),
            menu_from_parts(MenuParts {
                title: parts.title,
                items: parts.items,
            })
            .width(parts.size.x)
            .height(parts.size.y),
            text("").fill_width().height(1.0),
        ])
        .fill_width()
        .height(parts.size.y),
        text("").fill_width().fill_height(),
    ])
    .fill_width()
    .fill_height()
}

/// Build a full-surface context-menu layer with an input-only dismiss backing.
pub fn dismissible_context_menu<Message>(
    anchor: Point,
    size: Vector2,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dismissible_context_menu_from_parts(DismissibleContextMenuParts {
        anchor,
        size,
        title: title.into(),
        style: WidgetStyle::new(WidgetTone::Neutral, WidgetProminence::Strong),
        commands: commands.into_iter().collect(),
        dismiss_message,
    })
}

/// Build a full-surface context-menu layer with standard compact menu height.
///
/// Use this when the menu should use Radiant's standard compact menu row
/// heights and the caller only needs to choose the overlay width.
pub fn dismissible_context_menu_with_width<Message>(
    anchor: Point,
    width: f32,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = Vector2::new(width, message_menu_height(commands.len()));
    dismissible_context_menu(anchor, size, title, commands, dismiss_message)
}

/// Build a full-surface context-menu layer with Radiant's default compact menu
/// width and height policy.
pub fn dismissible_context_menu_auto_width<Message>(
    anchor: Point,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    dismissible_context_menu_with_width_policy(
        anchor,
        MessageMenuWidthPolicy::compact(),
        title,
        commands,
        dismiss_message,
    )
}

/// Build a full-surface context-menu layer with a deterministic menu width
/// derived from the title and command labels.
pub fn dismissible_context_menu_with_width_policy<Message>(
    anchor: Point,
    width_policy: MessageMenuWidthPolicy,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let title = title.into();
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = Vector2::new(
        width_policy.width_for_title_and_commands(&title, &commands),
        message_menu_height(commands.len()),
    );
    dismissible_context_menu(anchor, size, title, commands, dismiss_message)
}

/// Build a foreground-only message context-menu layer.
pub fn message_context_menu_overlay<Message>(
    anchor: Point,
    size: Vector2,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    message_context_menu_overlay_from_parts(MessageContextMenuOverlayParts {
        anchor,
        size,
        title: title.into(),
        style: WidgetStyle::new(WidgetTone::Neutral, WidgetProminence::Strong),
        commands: commands.into_iter().collect(),
    })
}

/// Build a foreground-only message context-menu layer with standard compact
/// menu height.
pub fn message_context_menu_overlay_with_width<Message>(
    anchor: Point,
    width: f32,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = Vector2::new(width, message_menu_height(commands.len()));
    message_context_menu_overlay(anchor, size, title, commands)
}

/// Build a foreground-only message context-menu layer using Radiant's default
/// compact width and height policy.
pub fn message_context_menu_overlay_auto_width<Message>(
    anchor: Point,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    message_context_menu_overlay_with_width_policy(
        anchor,
        MessageMenuWidthPolicy::compact(),
        title,
        commands,
    )
}

/// Build a foreground-only message context-menu layer with a deterministic menu
/// width derived from the title and command labels.
pub fn message_context_menu_overlay_with_width_policy<Message>(
    anchor: Point,
    width_policy: MessageMenuWidthPolicy,
    title: impl Into<String>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let title = title.into();
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = Vector2::new(
        width_policy.width_for_title_and_commands(&title, &commands),
        message_menu_height(commands.len()),
    );
    message_context_menu_overlay(anchor, size, title, commands)
}

/// Build a foreground-only message context-menu layer from named parts.
pub fn message_context_menu_overlay_from_parts<Message>(
    parts: MessageContextMenuOverlayParts<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let top = parts.anchor.y.max(0.0);
    let left = parts.anchor.x.max(0.0);
    column([
        text("").fill_width().height(top),
        row([
            text("").size(left, 1.0),
            message_menu_from_parts(MessageMenuParts {
                title: parts.title,
                style: parts.style,
                commands: parts.commands,
            })
            .width(parts.size.x)
            .height(parts.size.y),
            text("").fill_width().height(1.0),
        ])
        .fill_width()
        .height(parts.size.y),
        text("").fill_width().fill_height(),
    ])
    .fill()
}

/// Build a dismissible context-menu layer from named parts.
pub fn dismissible_context_menu_from_parts<Message>(
    parts: DismissibleContextMenuParts<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    stack([
        dismiss_layer(parts.dismiss_message).key("context-menu-dismiss"),
        message_context_menu_overlay_from_parts(MessageContextMenuOverlayParts {
            anchor: parts.anchor,
            size: parts.size,
            title: parts.title,
            style: parts.style,
            commands: parts.commands,
        }),
    ])
    .fill()
}

fn menu_item_button<State: 'static>(index: usize, item: MenuItem<State>) -> StateView<State> {
    let on_select = Arc::clone(&item.on_select);
    button(item.label)
        .on_click(move |state: &mut State| on_select(state))
        .key(format!("menu-item-{index}"))
        .style(item.style)
        .fill_width()
        .height(MENU_ITEM_HEIGHT)
}

fn menu_command_button<Message>(index: usize, command: MenuCommand<Message>) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button(command.label)
        .message(command.message)
        .key(format!("menu-command-{index}"))
        .style(command.style)
        .fill_width()
        .height(MENU_ITEM_HEIGHT)
}

#[cfg(test)]
mod tests;
