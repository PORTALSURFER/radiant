//! Graphical EQ editor sandbox for plugin-style GUI interaction.

#[path = "eq_editor/model.rs"]
mod model;
#[path = "eq_editor/widget.rs"]
mod widget;

use model::{EqBand, EqEditorState, EqMessage, selected_band, update};
use radiant::prelude::*;
use widget::EqEditorWidget;

const EQ_WIDGET_ID: u64 = 70;
const STATUS_WIDGET_ID: u64 = 71;

fn main() -> radiant::Result {
    radiant::app(EqEditorState::default())
        .title("Radiant Graphical EQ Editor")
        .size(920, 560)
        .min_size(680, 430)
        .view(project_surface)
        .update(update)
        .run()
}

fn project_surface(state: &mut EqEditorState) -> View<EqMessage> {
    let selected = selected_band(state).copied();

    column([
        row([
            text("Graphical EQ").height(30.0).fill_width(),
            toggle("Analyzer", state.analyzer)
                .message(|_| EqMessage::ToggleAnalyzer)
                .size(118.0, 30.0),
            toggle("Bypass", state.bypassed)
                .message(|_| EqMessage::ToggleBypass)
                .size(104.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        custom_widget_mapped(
            EqEditorWidget::new(state.bands.clone(), state.selected_band, state.analyzer),
            EqMessage::Editor,
        )
        .id(EQ_WIDGET_ID)
        .height(300.0)
        .fill_width(),
        row([
            selected_band_tile(selected),
            button("- Gain")
                .subtle()
                .message(EqMessage::NudgeGain(-0.5))
                .size(90.0, 30.0),
            button("+ Gain")
                .primary()
                .message(EqMessage::NudgeGain(0.5))
                .size(90.0, 30.0),
            button("- Q")
                .subtle()
                .message(EqMessage::NudgeQ(-0.05))
                .size(72.0, 30.0),
            button("+ Q")
                .primary()
                .message(EqMessage::NudgeQ(0.05))
                .size(72.0, 30.0),
            button("Band")
                .subtle()
                .message(EqMessage::ToggleSelectedBand)
                .size(78.0, 30.0),
            text(state.status.clone())
                .id(STATUS_WIDGET_ID)
                .height(30.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        grid_with_gaps(
            state
                .bands
                .iter()
                .map(|band| band_summary(*band, band.id == state.selected_band))
                .collect::<Vec<_>>(),
            4,
            10.0,
            10.0,
        )
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn selected_band_tile(selected: Option<EqBand>) -> View<EqMessage> {
    let label = selected
        .map(|band| {
            format!(
                "{} {:.0} Hz / {:+.1} dB",
                band.label, band.freq_hz, band.gain_db
            )
        })
        .unwrap_or_else(|| "No band".into());
    text(label).height(30.0).fill_width()
}

fn band_summary(band: EqBand, selected: bool) -> View<EqMessage> {
    let state = if band.enabled { "on" } else { "off" };
    let style = if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        }
    };

    column([
        text(format!("{} {}", band.id, band.label))
            .height(22.0)
            .fill_width(),
        text(format!("{:.0} Hz", band.freq_hz))
            .height(22.0)
            .fill_width(),
        text(format!(
            "{:+.1} dB / Q {:.2} / {state}",
            band.gain_db, band.q
        ))
        .height(22.0)
        .fill_width(),
    ])
    .style(style)
    .padding(10.0)
    .spacing(4.0)
    .height(92.0)
    .fill_width()
}

#[cfg(test)]
#[path = "eq_editor/tests.rs"]
mod tests;
