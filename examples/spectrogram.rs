//! Retained heatmap visualization backed by deterministic synthetic data.
//!
//! This example validates Radiant frame-driven updates, custom heatmap paint,
//! hover readout, and paint-only widget-local state. DSP and audio processing
//! behavior are intentionally non-authoritative host-domain behavior.

#[path = "spectrogram/model.rs"]
mod model;

#[path = "spectrogram/widget.rs"]
mod widget;

#[cfg(test)]
#[path = "spectrogram/tests.rs"]
mod tests;

use radiant::prelude::*;

use model::{
    DATA_SOURCE_NOTE, MAX_FREQ_HZ, MIN_FREQ_HZ, SpectrogramMessage, SpectrogramState, update,
};
use widget::SpectrogramWidget;

const SPECTROGRAM_WIDGET_ID: u64 = 80;
const STATUS_WIDGET_ID: u64 = 81;

fn main() -> radiant::Result {
    radiant::app(SpectrogramState::default())
        .title("Radiant Realtime Spectrogram")
        .size(980, 560)
        .min_size(720, 420)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| SpectrogramMessage::Frame)
        .update(update)
        .run()
}

fn project_surface(state: &mut SpectrogramState) -> View<SpectrogramMessage> {
    column([
        header_row(state),
        spectrogram_view(state),
        controls_row(state),
        stats_grid(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn header_row(state: &SpectrogramState) -> View<SpectrogramMessage> {
    row([
        text("Realtime Spectrogram").height(30.0).fill_width(),
        button(if state.running { "Pause" } else { "Run" })
            .primary()
            .message(SpectrogramMessage::ToggleRun)
            .size(88.0, 30.0),
        button("Reset")
            .subtle()
            .message(SpectrogramMessage::Reset)
            .size(82.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn spectrogram_view(state: &SpectrogramState) -> View<SpectrogramMessage> {
    custom_widget_mapped(
        SpectrogramWidget::new(state.columns.iter().cloned().collect(), state.frame),
        |message| message,
    )
    .id(SPECTROGRAM_WIDGET_ID)
    .height(320.0)
    .fill_width()
}

fn controls_row(state: &SpectrogramState) -> View<SpectrogramMessage> {
    row([
        button("- Energy")
            .subtle()
            .message(SpectrogramMessage::DecreaseIntensity)
            .size(104.0, 30.0),
        button("+ Energy")
            .primary()
            .message(SpectrogramMessage::IncreaseIntensity)
            .size(104.0, 30.0),
        button(format!("Speed {}x", state.speed))
            .subtle()
            .message(SpectrogramMessage::CycleSpeed)
            .size(112.0, 30.0),
        text(format!("{:.0} Hz", MIN_FREQ_HZ))
            .height(30.0)
            .fill_width(),
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(30.0)
            .fill_width(),
    ])
    .fill_width()
    .spacing(10.0)
}

fn stats_grid() -> View<SpectrogramMessage> {
    grid_with_gaps(
        [
            stat_tile("Bins", model::BINS.to_string()),
            stat_tile("History", format!("{} columns", model::COLUMNS)),
            stat_tile("Source", DATA_SOURCE_NOTE),
            stat_tile(
                "Range",
                format!("{:.0} Hz - {:.0} kHz", MIN_FREQ_HZ, MAX_FREQ_HZ / 1_000.0),
            ),
        ],
        4,
        10.0,
        10.0,
    )
    .fill_width()
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<SpectrogramMessage> {
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
    .height(76.0)
    .fill_width()
}
