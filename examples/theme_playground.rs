//! Theme-token and density-policy playground.

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

    radiant::window("Radiant Theme Playground")
        .size(520, 260)
        .min_size(360, 220)
        .run(project_surface(report))
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

fn project_surface(report: ThemeReport) -> View {
    column([
        text("Theme Playground").height(30.0).fill_width(),
        row([
            theme_tile(
                "Compact",
                theme_report(720.0, 0.9),
                WidgetStyle {
                    tone: WidgetTone::Neutral,
                    prominence: WidgetProminence::Subtle,
                },
            ),
            theme_tile("Standard", report, primary_style()),
            theme_tile("Wide", theme_report(2400.0, 1.6), danger_style()),
        ])
        .spacing(10.0)
        .fill_width(),
        text(format!(
            "scale {:.2} hover blend {:.2}",
            report.scale, report.hover_blend
        ))
        .height(28.0)
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
}

fn theme_tile(label: &'static str, report: ThemeReport, style: WidgetStyle) -> View {
    column([
        text(label).height(24.0).fill_width(),
        badge(format!("{:?}", report.tier))
            .style(style)
            .message(())
            .size(108.0, 26.0),
        text(format!("scale {:.2}", report.scale))
            .height(24.0)
            .fill_width(),
    ])
    .style(style)
    .padding(10.0)
    .spacing(8.0)
    .height(118.0)
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

        let surface = project_surface(wide).into_surface();
        assert!(!surface.keyboard_focus_order().is_empty());
    }
}
