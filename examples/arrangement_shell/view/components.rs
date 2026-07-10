use radiant::prelude::*;

use super::super::{AppMessage, TRACKS, model::TrackMeter};

pub(super) fn collapsed_panel(label: &'static str, width: f32) -> View<AppMessage> {
    column([text(label).height(24.0).fill_width()])
        .style(panel_style())
        .padding(10.0)
        .size(width, 400.0)
}

pub(super) fn meter_tile(meter: TrackMeter) -> View<AppMessage> {
    column([
        text(TRACKS[meter.track]).height(20.0).fill_width(),
        text(format!(
            "lvl {:>3}% pk {:>3}%",
            (meter.level * 100.0) as u32,
            (meter.peak * 100.0) as u32
        ))
        .height(22.0)
        .fill_width(),
    ])
    .style(panel_style())
    .padding(10.0)
    .height(64.0)
    .fill_width()
}

pub(super) fn stat_tile(
    label: impl Into<TextContent>,
    value: impl Into<TextContent>,
) -> View<AppMessage> {
    column([
        text(label.into()).height(20.0).fill_width(),
        text(value.into()).height(22.0).fill_width(),
    ])
    .style(subtle_style())
    .padding(8.0)
    .height(58.0)
    .fill_width()
}

pub(super) fn panel_toggle_label(open: bool, name: &'static str) -> String {
    format!("{} {name}", if open { "Hide" } else { "Show" })
}

pub(super) fn track_button_style(selected: bool) -> WidgetStyle {
    if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        subtle_style()
    }
}

pub(super) fn panel_style() -> WidgetStyle {
    subtle_style()
}

pub(super) fn subtle_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    }
}
