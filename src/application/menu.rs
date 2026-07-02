use crate::{
    application::{
        ViewNode, button, column, dismiss_layer, floating_layer_with_input_and_vertical_overflow,
        row, stack, text,
    },
    gui::{
        text_layout::{TextWidthEstimate, estimated_text_width_for_char_count},
        types::Point,
    },
    layout::{FloatingLayerVerticalOverflow, Vector2},
    widgets::{TextAlign, TextColorRole, WidgetProminence, WidgetStyle, WidgetTone},
};

mod model;

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

const MENU_ROW_TEXT_PADDING_X: f32 = 8.0;
const MENU_LABEL_HOTKEY_GAP: f32 = 16.0;

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
    let command_text = menu_command_text_columns(&parts.commands);
    column([
        text(parts.title).fill_width().height(MENU_TITLE_HEIGHT),
        column(
            parts
                .commands
                .into_iter()
                .enumerate()
                .map(|(index, command)| menu_command_button(index, command, command_text)),
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
    let anchor = Point::new(parts.anchor.x.max(0.0), parts.anchor.y.max(0.0));
    floating_layer_with_input_and_vertical_overflow(
        anchor,
        parts.size,
        message_menu_from_parts(MessageMenuParts {
            title: parts.title,
            style: parts.style,
            commands: parts.commands,
        })
        .width(parts.size.x)
        .height(parts.size.y),
        true,
        FloatingLayerVerticalOverflow::FlipUpWhenClipped,
    )
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

#[derive(Clone, Copy)]
struct MenuCommandTextColumns {
    hotkey_hint_width: f32,
}

fn menu_command_text_columns<Message>(commands: &[MenuCommand<Message>]) -> MenuCommandTextColumns {
    let compact = MessageMenuWidthPolicy::compact();
    let metrics = TextWidthEstimate::new(compact.metrics.character_advance, 0.0);
    let hotkey_hint_width = commands
        .iter()
        .filter_map(|command| command.hotkey_hint.as_ref())
        .map(|hint| estimated_text_width_for_char_count(hint.chars().count(), metrics))
        .fold(0.0, f32::max);
    MenuCommandTextColumns { hotkey_hint_width }
}

fn menu_command_button<Message>(
    index: usize,
    command: MenuCommand<Message>,
    text_columns: MenuCommandTextColumns,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let label_color = menu_command_label_color(command.style);
    let hint_color = menu_command_hotkey_hint_color(command.style);
    let mut label_row = vec![
        text(command.label.clone())
            .align_text(TextAlign::Left)
            .text_color(label_color)
            .truncate()
            .fill_width()
            .height(MENU_ITEM_HEIGHT),
    ];
    if text_columns.hotkey_hint_width > 0.0 {
        label_row.push(
            text(command.hotkey_hint.clone().unwrap_or_default())
                .align_text(TextAlign::Right)
                .text_color(hint_color)
                .truncate()
                .width(text_columns.hotkey_hint_width)
                .height(MENU_ITEM_HEIGHT),
        );
    }
    stack([
        button("")
            .message(command.message)
            .key(format!("menu-command-{index}"))
            .style(command.style)
            .fill_width()
            .height(MENU_ITEM_HEIGHT),
        row(label_row)
            .fill_width()
            .height(MENU_ITEM_HEIGHT)
            .padding_x(MENU_ROW_TEXT_PADDING_X)
            .spacing(if text_columns.hotkey_hint_width > 0.0 {
                MENU_LABEL_HOTKEY_GAP
            } else {
                0.0
            }),
    ])
    .fill_width()
    .height(MENU_ITEM_HEIGHT)
}

fn menu_command_label_color(style: WidgetStyle) -> TextColorRole {
    if matches!(
        (style.prominence, style.tone),
        (WidgetProminence::Subtle, WidgetTone::Neutral)
    ) {
        TextColorRole::Muted
    } else {
        TextColorRole::Primary
    }
}

fn menu_command_hotkey_hint_color(style: WidgetStyle) -> TextColorRole {
    if matches!(style.prominence, WidgetProminence::Strong)
        && !matches!(style.tone, WidgetTone::Neutral)
    {
        TextColorRole::Primary
    } else {
        TextColorRole::Muted
    }
}

#[cfg(test)]
mod tests;
