//! Fluent construction for anchored and dismissible context menus.

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
    MenuCommand, MessageMenuWidthPolicy,
    actions::{message_menu_from_parts, message_menu_height},
    model::MessageMenuParts,
};

/// Fluent context-menu builder before its required surface anchor is supplied.
pub struct ContextMenuBuilder<Message> {
    title: TextContent,
    style: WidgetStyle,
    commands: Vec<MenuCommand<Message>>,
}

/// Fluent context-menu builder after its required surface anchor is supplied.
pub struct AnchoredContextMenuBuilder<Message> {
    anchor: Point,
    title: TextContent,
    style: WidgetStyle,
    commands: Vec<MenuCommand<Message>>,
    sizing: ContextMenuSizing,
    dismiss_message: Option<Message>,
}

enum ContextMenuSizing {
    WidthPolicy(MessageMenuWidthPolicy),
    Width(f32),
    Exact(Vector2),
}

/// Start a foreground-only context menu with Radiant's compact automatic-width policy.
///
/// Call [`ContextMenuBuilder::anchor`] to supply the required surface anchor.
/// The anchored builder can then opt into an exact size, a fixed width, a
/// different width policy, or an outside-click dismissal message before
/// producing the final view.
pub fn context_menu<Message>(
    title: impl Into<TextContent>,
    commands: impl IntoIterator<Item = MenuCommand<Message>>,
) -> ContextMenuBuilder<Message> {
    ContextMenuBuilder {
        title: title.into(),
        style: WidgetStyle::new(WidgetTone::Neutral, WidgetProminence::Strong),
        commands: commands.into_iter().collect(),
    }
}

impl<Message> ContextMenuBuilder<Message> {
    /// Anchor the menu at a point in surface coordinates.
    pub fn anchor(self, anchor: Point) -> AnchoredContextMenuBuilder<Message> {
        AnchoredContextMenuBuilder {
            anchor,
            title: self.title,
            style: self.style,
            commands: self.commands,
            sizing: ContextMenuSizing::WidthPolicy(MessageMenuWidthPolicy::compact()),
            dismiss_message: None,
        }
    }
}

impl<Message> AnchoredContextMenuBuilder<Message> {
    /// Use an exact logical size for the menu overlay.
    pub fn size(mut self, size: Vector2) -> Self {
        self.sizing = ContextMenuSizing::Exact(size);
        self
    }

    /// Use a fixed width and the standard compact height for the command count.
    pub fn width(mut self, width: f32) -> Self {
        self.sizing = ContextMenuSizing::Width(width);
        self
    }

    /// Size the menu from its title and commands with a deterministic policy.
    pub fn width_policy(mut self, width_policy: MessageMenuWidthPolicy) -> Self {
        self.sizing = ContextMenuSizing::WidthPolicy(width_policy);
        self
    }

    /// Apply visual styling to the menu surface.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    /// Emit a message when the user activates the full-surface dismiss backing.
    pub fn dismiss_on(mut self, message: Message) -> Self {
        self.dismiss_message = Some(message);
        self
    }

    /// Build the configured context-menu view.
    pub fn view(self) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        let size = self.size_for_content();
        let menu = anchored_popover_from_parts(AnchoredPopoverParts::below(
            message_menu_from_parts(MessageMenuParts {
                title: self.title,
                style: self.style,
                commands: self.commands,
            }),
            AnchoredPopoverAnchor::pointer(self.anchor),
            Vector2::new(size.x.max(1.0), size.y.max(1.0)),
        ));
        match self.dismiss_message {
            Some(message) => {
                stack([dismiss_layer(message).key("context-menu-dismiss"), menu]).fill()
            }
            None => menu,
        }
    }

    fn size_for_content(&self) -> Vector2 {
        match self.sizing {
            ContextMenuSizing::WidthPolicy(policy) => Vector2::new(
                policy.width_for_title_and_commands(&self.title, &self.commands),
                message_menu_height(self.commands.len()),
            ),
            ContextMenuSizing::Width(width) => {
                Vector2::new(width, message_menu_height(self.commands.len()))
            }
            ContextMenuSizing::Exact(size) => size,
        }
    }
}
