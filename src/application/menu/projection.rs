//! Menu-command row projection and shortcut-hint layout inputs.

use crate::{
    application::{ViewNode, button, row, stack, text},
    gui::text_layout::{TextWidthEstimate, estimated_text_width_for_char_count},
    widgets::{TextAlign, TextColorRole, WidgetProminence, WidgetStyle, WidgetTone},
};

use super::{MenuCommand, MessageMenuWidthPolicy, actions::MENU_ITEM_HEIGHT};

pub(super) const MENU_ROW_TEXT_PADDING_X: f32 = 8.0;
pub(super) const MENU_LABEL_HOTKEY_GAP: f32 = 16.0;
pub(super) const MENU_HOTKEY_HINT_HORIZONTAL_PADDING: f32 = 16.0;

#[derive(Clone, Copy)]
pub(super) struct MenuCommandTextColumns {
    hotkey_hint_width: f32,
}

impl MenuCommandTextColumns {
    pub(super) fn for_commands<Message>(commands: &[MenuCommand<Message>]) -> Self {
        let compact = MessageMenuWidthPolicy::compact();
        let metrics = TextWidthEstimate::new(
            compact.metrics.character_advance,
            MENU_HOTKEY_HINT_HORIZONTAL_PADDING,
        );
        let hotkey_hint_width = commands
            .iter()
            .filter_map(|command| command.hotkey_hint.as_ref())
            .map(|hint| estimated_text_width_for_char_count(hint.chars().count(), metrics))
            .fold(0.0, f32::max);
        Self { hotkey_hint_width }
    }
}

pub(super) fn menu_command_row<Message>(
    index: usize,
    command: MenuCommand<Message>,
    text_columns: MenuCommandTextColumns,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let label_color = menu_command_label_color(command.style);
    let hint_color = menu_command_hotkey_hint_color(command.style);
    let hotkey_hint = command.hotkey_hint.clone();
    let mut label_row = vec![
        text(command.label.clone())
            .align_text(TextAlign::Left)
            .text_color(label_color)
            .truncate()
            .fill_width()
            .height(MENU_ITEM_HEIGHT),
    ];
    let has_hotkey_hint = hotkey_hint.is_some() && text_columns.hotkey_hint_width > 0.0;
    if let Some(hotkey_hint) = hotkey_hint {
        label_row.push(
            text(hotkey_hint)
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
            .spacing(if has_hotkey_hint {
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
