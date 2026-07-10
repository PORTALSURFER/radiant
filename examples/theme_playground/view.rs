use radiant::prelude::*;

use crate::{
    model::{PlaygroundMessage, PlaygroundState},
    report::{ThemeReport, primary_style, theme_report},
};

pub(super) fn project_surface(state: &PlaygroundState) -> View<PlaygroundMessage> {
    let selected_style = WidgetStyle {
        tone: state.selected_tone,
        prominence: WidgetProminence::Strong,
    };

    column([
        text("Theme Playground").height(32.0).fill_width(),
        section(
            "Density",
            row([
                density_tile("Compact", theme_report(720.0, 0.9)),
                density_tile("Standard", theme_report(1280.0, 1.25)),
                density_tile("Wide", theme_report(2400.0, 1.6)),
            ])
            .spacing(12.0)
            .fill_width(),
        ),
        section(
            "Tone",
            row([
                tone_button("Neutral", WidgetTone::Neutral, state.selected_tone),
                tone_button("Accent", WidgetTone::Accent, state.selected_tone),
                tone_button("Success", WidgetTone::Success, state.selected_tone),
                tone_button("Warning", WidgetTone::Warning, state.selected_tone),
                tone_button("Danger", WidgetTone::Danger, state.selected_tone),
            ])
            .spacing(8.0)
            .fill_width(),
        ),
        section(
            "Prominence",
            row([
                prominence_tile("Subtle", selected_style, WidgetProminence::Subtle),
                prominence_tile("Normal", selected_style, WidgetProminence::Normal),
                prominence_tile("Strong", selected_style, WidgetProminence::Strong),
            ])
            .spacing(12.0)
            .fill_width(),
        ),
        section(
            "State",
            row([
                button("Action")
                    .style(selected_style)
                    .message(PlaygroundMessage::SelectTone(state.selected_tone))
                    .height(36.0)
                    .fill_width(),
                toggle("Active preview", state.active_preview)
                    .style(selected_style)
                    .message(PlaygroundMessage::ToggleActive)
                    .height(36.0)
                    .fill_width(),
                badge(if state.active_preview {
                    "active"
                } else {
                    "idle"
                })
                .style(WidgetStyle {
                    tone: state.selected_tone,
                    prominence: if state.active_preview {
                        WidgetProminence::Strong
                    } else {
                        WidgetProminence::Subtle
                    },
                })
                .message(PlaygroundMessage::SelectTone(state.selected_tone))
                .height(30.0)
                .fill_width(),
            ])
            .spacing(12.0)
            .fill_width(),
        ),
    ])
    .style(WidgetStyle::default())
    .padding(18.0)
    .spacing(14.0)
}

fn section(label: &'static str, content: View<PlaygroundMessage>) -> View<PlaygroundMessage> {
    column([text(label).height(22.0).fill_width(), content])
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        })
        .padding(12.0)
        .spacing(8.0)
        .fill_width()
}

fn density_tile(label: &'static str, report: ThemeReport) -> View<PlaygroundMessage> {
    let height = (62.0 * report.scale).clamp(56.0, 118.0);
    column([
        text(label).height(22.0).fill_width(),
        badge(format!("{:?}", report.tier))
            .style(primary_style())
            .message(PlaygroundMessage::SelectTone(WidgetTone::Accent))
            .height((22.0 * report.scale).clamp(20.0, 34.0))
            .fill_width(),
        text(format!("scale {:.2}", report.scale))
            .height(20.0)
            .fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Normal,
    })
    .padding((8.0 * report.scale).clamp(6.0, 18.0))
    .spacing((6.0 * report.scale).clamp(4.0, 12.0))
    .height(height)
    .fill_width()
}

fn tone_button(
    label: &'static str,
    tone: WidgetTone,
    selected_tone: WidgetTone,
) -> View<PlaygroundMessage> {
    button(label)
        .style(WidgetStyle {
            tone,
            prominence: if tone == selected_tone {
                WidgetProminence::Strong
            } else {
                WidgetProminence::Normal
            },
        })
        .message(PlaygroundMessage::SelectTone(tone))
        .height(36.0)
        .fill_width()
}

fn prominence_tile(
    label: &'static str,
    style: WidgetStyle,
    prominence: WidgetProminence,
) -> View<PlaygroundMessage> {
    let style = WidgetStyle {
        prominence,
        ..style
    };
    column([
        text(label).height(22.0).fill_width(),
        button("Button")
            .style(style)
            .message(PlaygroundMessage::SelectTone(style.tone))
            .height(34.0)
            .fill_width(),
        badge(format!("{:?}", style.prominence))
            .style(style)
            .message(PlaygroundMessage::SelectTone(style.tone))
            .height(26.0)
            .fill_width(),
    ])
    .style(style)
    .padding(10.0)
    .spacing(8.0)
    .height(120.0)
    .fill_width()
}
