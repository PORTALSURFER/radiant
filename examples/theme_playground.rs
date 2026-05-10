//! Theme-token, tone, prominence, and density-policy playground.

use radiant::prelude::*;
use radiant::{
    theme::{ThemeTokens, ViewportScaleTier, effective_ui_scale},
    widgets::resolve_widget_visual_tokens,
};

fn main() -> radiant::Result {
    let report = theme_report(1280.0, 1.25);
    println!(
        "radiant_theme_playground tier={:?} scale={:.3} accent_fill={:?} danger_fill={:?}",
        report.tier, report.scale, report.accent_fill, report.danger_fill
    );

    radiant::app(PlaygroundState::default())
        .title("Radiant Theme Playground")
        .size(760, 560)
        .min_size(520, 420)
        .view(project_surface)
        .update(|state, message| match message {
            PlaygroundMessage::SelectTone(tone) => state.selected_tone = tone,
            PlaygroundMessage::ToggleActive(active) => state.active_preview = active,
        })
        .run()
}

#[derive(Clone, Debug, PartialEq)]
enum PlaygroundMessage {
    SelectTone(WidgetTone),
    ToggleActive(bool),
}

#[derive(Clone, Debug, PartialEq)]
struct PlaygroundState {
    selected_tone: WidgetTone,
    active_preview: bool,
}

impl Default for PlaygroundState {
    fn default() -> Self {
        Self {
            selected_tone: WidgetTone::Accent,
            active_preview: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ThemeReport {
    tier: ViewportScaleTier,
    scale: f32,
    accent_fill: radiant::gui::types::Rgba8,
    danger_fill: radiant::gui::types::Rgba8,
    hover_blend: f32,
}

fn theme_report(viewport_width: f32, requested_scale: f32) -> ThemeReport {
    let tier = ViewportScaleTier::from_viewport_width(viewport_width);
    let theme = ThemeTokens::dark_for_viewport_width(viewport_width);
    let accent = resolve_widget_visual_tokens(&theme, primary_style(), Default::default());
    let danger = resolve_widget_visual_tokens(&theme, danger_style(), Default::default());

    ThemeReport {
        tier,
        scale: effective_ui_scale(requested_scale),
        accent_fill: accent.fill,
        danger_fill: danger.fill,
        hover_blend: theme.state_hover_strong,
    }
}

fn project_surface(state: &mut PlaygroundState) -> View<PlaygroundMessage> {
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

fn primary_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    }
}

fn danger_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Danger,
        prominence: WidgetProminence::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn theme_playground_resolves_density_and_visual_tokens() {
        let compact = theme_report(720.0, 0.5);
        let wide = theme_report(2400.0, 1.6);

        assert_eq!(compact.tier, ViewportScaleTier::Compact);
        assert_eq!(wide.tier, ViewportScaleTier::Wide);
        assert!(wide.scale > compact.scale);
        assert_ne!(wide.accent_fill, wide.danger_fill);

        let mut state = PlaygroundState::default();
        let surface = project_surface(&mut state).into_surface();
        assert!(surface.keyboard_focus_order().len() >= 8);
    }

    #[test]
    fn theme_playground_projects_distinct_tone_and_state_controls() {
        let mut state = PlaygroundState {
            selected_tone: WidgetTone::Danger,
            active_preview: false,
        };

        let surface = project_surface(&mut state).into_surface();

        assert!(
            surface.keyboard_focus_order().len() >= 8,
            "tone buttons, prominence controls, and state controls should all be interactive"
        );
    }
}
