//! Message-menu composition and command activation projection.

use crate::{
    application::{TextContent, ViewNode, column, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

use super::{MenuCommand, model::MessageMenuParts, projection::menu_command_row};

/// Height of the title line in compact Radiant menus.
pub(super) const MENU_TITLE_HEIGHT: f32 = 22.0;
/// Height of one command row in compact Radiant menus.
pub(super) const MENU_ITEM_HEIGHT: f32 = 28.0;
/// Outer padding applied by compact Radiant menus.
pub(super) const MENU_PADDING: f32 = 8.0;
/// Gap between the title and the command list.
pub(super) const MENU_SECTION_SPACING: f32 = 6.0;
/// Gap between command rows.
pub(super) const MENU_ITEM_SPACING: f32 = 4.0;

/// Build a compact vertical menu that emits host messages.
pub fn message_menu<Message>(
    title: impl Into<TextContent>,
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
    let command_text = super::projection::MenuCommandTextColumns::for_commands(&parts.commands);
    column([
        text(parts.title).fill_width().height(MENU_TITLE_HEIGHT),
        column(
            parts
                .commands
                .into_iter()
                .enumerate()
                .map(|(index, command)| menu_command_row(index, command, command_text)),
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
