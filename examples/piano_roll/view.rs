use radiant::prelude::*;

use super::{
    AppMessage, DATA_SOURCE_NOTE, PIANO_ROLL_WIDGET_ID, PianoRollState, PianoRollWidget,
    STATUS_WIDGET_ID,
};

pub(crate) fn project_surface(state: &mut PianoRollState) -> View<AppMessage> {
    column([
        header_row(state),
        custom_widget_mapped(
            PianoRollWidget::new(
                state.notes.clone(),
                state.selected_note,
                state.playhead_beat,
            ),
            AppMessage::Roll,
        )
        .id(PIANO_ROLL_WIDGET_ID)
        .height(390.0)
        .fill_width(),
        status_row(state),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn header_row(state: &PianoRollState) -> View<AppMessage> {
    row([
        text("Piano Roll").height(30.0).fill_width(),
        button(if state.running { "Pause" } else { "Run" })
            .primary()
            .message(AppMessage::ToggleRun)
            .size(88.0, 30.0),
        button("Reset")
            .subtle()
            .message(AppMessage::Reset)
            .size(82.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn status_row(state: &PianoRollState) -> View<AppMessage> {
    row([
        stat_tile("Notes", state.notes.len().to_string()),
        stat_tile("Grid", "1/4 beat"),
        stat_tile("Range", "C3 - B4"),
        stat_tile("Source", DATA_SOURCE_NOTE),
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(68.0)
            .fill_width(),
    ])
    .fill_width()
    .spacing(10.0)
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<AppMessage> {
    column([
        text(label.into()).height(22.0).fill_width(),
        text(value.into()).height(24.0).fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(10.0)
    .spacing(4.0)
    .height(68.0)
    .fill_width()
}
