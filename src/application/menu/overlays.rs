//! Anchored and dismissible menu overlays plus compatibility entry points.

use crate::{
    application::{
        AnchoredPopoverAnchor, AnchoredPopoverParts, TextContent, ViewNode,
        anchored_popover_from_parts, dismiss_layer, stack,
    },
    gui::types::Point,
    layout::Vector2,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

use super::{
    DismissibleContextMenuParts, MenuCommand, MessageContextMenuOverlayParts, MessageMenuParts,
    MessageMenuWidthPolicy, message_menu_from_parts, message_menu_height,
};

/// Build a full-surface context-menu layer with an input-only dismiss backing.
pub fn dismissible_context_menu<Message>(
    anchor: Point,
    size: Vector2,
    title: impl Into<TextContent>,
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
pub fn dismissible_context_menu_with_width<Message>(
    anchor: Point,
    width: f32,
    title: impl Into<TextContent>,
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

/// Build a full-surface context-menu layer with Radiant's default compact menu policy.
pub fn dismissible_context_menu_auto_width<Message>(
    anchor: Point,
    title: impl Into<TextContent>,
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

/// Build a full-surface context-menu layer with a deterministic width policy.
pub fn dismissible_context_menu_with_width_policy<Message>(
    anchor: Point,
    width_policy: MessageMenuWidthPolicy,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
    dismiss_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let title = title.into();
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = menu_size(width_policy, &title, &commands);
    dismissible_context_menu(anchor, size, title, commands, dismiss_message)
}

/// Build a foreground-only message context-menu layer.
pub fn message_context_menu_overlay<Message>(
    anchor: Point,
    size: Vector2,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay(anchor, size, title, commands)
}

/// Build a foreground-only message context-menu layer with standard compact height.
pub fn message_context_menu_overlay_with_width<Message>(
    anchor: Point,
    width: f32,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay_with_width(anchor, width, title, commands)
}

/// Build a foreground-only message context-menu layer using the compact policy.
pub fn message_context_menu_overlay_auto_width<Message>(
    anchor: Point,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay_auto_width(anchor, title, commands)
}

/// Build a foreground-only message context-menu layer with a width policy.
pub fn message_context_menu_overlay_with_width_policy<Message>(
    anchor: Point,
    width_policy: MessageMenuWidthPolicy,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay_with_width_policy(anchor, width_policy, title, commands)
}

/// Build a foreground-only message context-menu layer from named parts.
pub fn message_context_menu_overlay_from_parts<Message>(
    parts: MessageContextMenuOverlayParts<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay_from_parts(parts)
}

/// Build a foreground-only anchored message-menu layer.
pub fn anchored_message_menu_overlay<Message>(
    anchor: Point,
    size: Vector2,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay_from_parts(MessageContextMenuOverlayParts {
        anchor,
        size,
        title: title.into(),
        style: WidgetStyle::new(WidgetTone::Neutral, WidgetProminence::Strong),
        commands: commands.into_iter().collect(),
    })
}

fn anchored_message_menu_overlay_with_width<Message>(
    anchor: Point,
    width: f32,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = Vector2::new(width, message_menu_height(commands.len()));
    anchored_message_menu_overlay(anchor, size, title, commands)
}

/// Build an anchored message-menu layer using the standard compact policy.
pub fn anchored_message_menu_overlay_auto_width<Message>(
    anchor: Point,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    anchored_message_menu_overlay_with_width_policy(
        anchor,
        MessageMenuWidthPolicy::compact(),
        title,
        commands,
    )
}

/// Build an anchored message-menu layer with a deterministic width policy.
pub fn anchored_message_menu_overlay_with_width_policy<Message>(
    anchor: Point,
    width_policy: MessageMenuWidthPolicy,
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let title = title.into();
    let commands = commands.into_iter().collect::<Vec<_>>();
    let size = menu_size(width_policy, &title, &commands);
    anchored_message_menu_overlay(anchor, size, title, commands)
}

/// Build a foreground-only anchored message-menu layer from named parts.
pub fn anchored_message_menu_overlay_from_parts<Message>(
    parts: MessageContextMenuOverlayParts<Message>,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let size = Vector2::new(parts.size.x.max(1.0), parts.size.y.max(1.0));
    anchored_popover_from_parts(AnchoredPopoverParts::below(
        message_menu_from_parts(MessageMenuParts {
            title: parts.title,
            style: parts.style,
            commands: parts.commands,
        }),
        AnchoredPopoverAnchor::pointer(parts.anchor),
        size,
    ))
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
        anchored_message_menu_overlay_from_parts(MessageContextMenuOverlayParts {
            anchor: parts.anchor,
            size: parts.size,
            title: parts.title,
            style: parts.style,
            commands: parts.commands,
        }),
    ])
    .fill()
}

fn menu_size<Message>(
    width_policy: MessageMenuWidthPolicy,
    title: &str,
    commands: &[MenuCommand<Message>],
) -> Vector2 {
    Vector2::new(
        width_policy.width_for_title_and_commands(title, commands),
        message_menu_height(commands.len()),
    )
}
