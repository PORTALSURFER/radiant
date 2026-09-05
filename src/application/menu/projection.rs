//! Menu-command row projection and shortcut-hint layout inputs.

use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    gui::text_layout::{TextWidthEstimate, estimated_text_width_for_char_count},
    runtime::WidgetMessageMapper,
    widgets::{TextColorRole, WidgetProminence, WidgetStyle, WidgetTone},
};

use super::{MenuCommand, MessageMenuWidthPolicy, widget::MenuCommandWidget};

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
    Message: Clone + 'static,
{
    view_node_from_widget(MappedWidget::new(
        MenuCommandWidget::new(
            command.label,
            command.hotkey_hint,
            text_columns.hotkey_hint_width,
        ),
        WidgetMessageMapper::button_message(command.message),
    ))
    .key(format!("menu-command-{index}"))
    .style(command.style)
    .fill_width()
}

pub(super) fn menu_command_label_color(style: WidgetStyle) -> TextColorRole {
    if matches!(
        (style.prominence, style.tone),
        (WidgetProminence::Subtle, WidgetTone::Neutral)
    ) {
        TextColorRole::Muted
    } else {
        TextColorRole::Primary
    }
}

pub(super) fn menu_command_hotkey_hint_color(style: WidgetStyle) -> TextColorRole {
    if matches!(style.prominence, WidgetProminence::Strong)
        && !matches!(style.tone, WidgetTone::Neutral)
    {
        TextColorRole::Primary
    } else {
        TextColorRole::Muted
    }
}
