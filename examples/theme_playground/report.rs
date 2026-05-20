use radiant::{
    gui::types::Rgba8,
    prelude::{WidgetProminence, WidgetStyle, WidgetTone},
    theme::{ThemeTokens, ViewportScaleTier, effective_ui_scale},
    widgets::resolve_widget_visual_tokens,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ThemeReport {
    pub(super) tier: ViewportScaleTier,
    pub(super) scale: f32,
    pub(super) accent_fill: Rgba8,
    pub(super) danger_fill: Rgba8,
    pub(super) hover_blend: f32,
}

pub(super) fn theme_report(viewport_width: f32, requested_scale: f32) -> ThemeReport {
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

pub(super) fn primary_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    }
}

pub(super) fn danger_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Danger,
        prominence: WidgetProminence::Normal,
    }
}
