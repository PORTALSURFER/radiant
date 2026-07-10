use radiant::prelude::*;

use super::{
    AppMessage, DATA_SOURCE_NOTE, DESTINATION_COUNT, MATRIX_WIDGET_ID, MatrixMessage,
    ModulationMatrixState, ModulationMatrixWidget, SOURCE_COUNT, STATUS_WIDGET_ID,
};

pub(crate) fn project_surface(state: &ModulationMatrixState) -> View<AppMessage> {
    column([
        header_row(state),
        custom_widget_mapped(
            ModulationMatrixWidget::new(state.amounts, state.selected, state.activity_phase),
            AppMessage::Matrix,
        )
        .id(MATRIX_WIDGET_ID)
        .height(390.0)
        .fill_width(),
        status_row(state),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn header_row(state: &ModulationMatrixState) -> View<AppMessage> {
    row([
        text("Modulation Matrix").height(30.0).fill_width(),
        button(if state.running { "Pause" } else { "Run" })
            .primary()
            .message(AppMessage::ToggleRun)
            .size(88.0, 30.0),
        button("Clear")
            .subtle()
            .message(AppMessage::Matrix(MatrixMessage::ClearSelected))
            .size(82.0, 30.0),
        button("Reset")
            .subtle()
            .message(AppMessage::Reset)
            .size(82.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn status_row(state: &ModulationMatrixState) -> View<AppMessage> {
    row([
        stat_tile("Sources", SOURCE_COUNT.to_string()),
        stat_tile("Destinations", DESTINATION_COUNT.to_string()),
        stat_tile(
            "Selected",
            format!("{}%", (state.selected_amount() * 100.0).round()),
        ),
        stat_tile("Source", DATA_SOURCE_NOTE),
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(68.0)
            .fill_width(),
    ])
    .fill_width()
    .spacing(10.0)
}

fn stat_tile(label: impl Into<TextContent>, value: impl Into<TextContent>) -> View<AppMessage> {
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
