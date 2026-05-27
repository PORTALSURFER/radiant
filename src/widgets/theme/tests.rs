use super::{WidgetVisualTokens, resolve_widget_visual_tokens};
use crate::{
    theme::ThemeTokens,
    widgets::{WidgetProminence, WidgetState, WidgetStyle, WidgetTone},
};

#[test]
fn disabled_widgets_use_muted_foreground_and_disabled_fill() {
    let theme = ThemeTokens::default();
    let tokens = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            disabled: true,
            ..WidgetState::default()
        },
    );
    assert_eq!(
        tokens,
        WidgetVisualTokens {
            fill: theme.control_disabled_fill,
            foreground: theme.text_muted,
            border: theme.grid_soft,
            emphasis: theme.text_primary,
        }
    );
}

#[test]
fn accent_widgets_use_accent_color_when_active() {
    let theme = ThemeTokens::default();
    let tokens = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Strong,
        },
        WidgetState {
            active: true,
            ..WidgetState::default()
        },
    );
    assert_eq!(tokens.fill, theme.accent_mint);
    assert_eq!(tokens.emphasis, theme.accent_mint);
}

#[test]
fn strong_warning_controls_keep_dark_text_on_warning_fill() {
    let theme = ThemeTokens::default();
    let tokens = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle {
            tone: WidgetTone::Warning,
            prominence: WidgetProminence::Strong,
        },
        WidgetState::default(),
    );

    assert_eq!(tokens.fill, theme.accent_warning);
    assert_eq!(tokens.foreground, theme.bg_primary);
    assert_eq!(tokens.emphasis, theme.accent_warning);
}

#[test]
fn neutral_active_controls_use_selected_surface() {
    let theme = ThemeTokens::default();
    let base = resolve_widget_visual_tokens(&theme, WidgetStyle::default(), WidgetState::default());
    let active = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            active: true,
            ..WidgetState::default()
        },
    );

    assert_eq!(base.fill, theme.surface_raised);
    assert_eq!(active.fill, theme.surface_overlay);
    assert_ne!(active.fill, base.fill);
    assert_eq!(active.foreground, theme.text_primary);
}

#[test]
fn hover_and_press_states_change_control_chrome() {
    let theme = ThemeTokens::default();
    let base = resolve_widget_visual_tokens(&theme, WidgetStyle::default(), WidgetState::default());
    let hovered = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            hovered: true,
            ..WidgetState::default()
        },
    );
    let pressed = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            pressed: true,
            ..WidgetState::default()
        },
    );

    assert_ne!(hovered.fill, base.fill);
    assert_ne!(hovered.border, base.border);
    assert_ne!(pressed.fill, hovered.fill);
    assert_ne!(pressed.border, base.border);
}

#[test]
fn subtle_controls_use_recessed_fill_and_toned_controls_lift() {
    let theme = ThemeTokens::default();
    let neutral = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        },
        WidgetState::default(),
    );
    let danger = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle {
            tone: WidgetTone::Danger,
            prominence: WidgetProminence::Subtle,
        },
        WidgetState::default(),
    );

    assert_eq!(neutral.fill, theme.bg_primary);
    assert_eq!(neutral.foreground, theme.text_muted);
    assert_eq!(neutral.border, theme.grid_soft);
    assert_ne!(danger.fill, theme.surface_base);
    assert_eq!(danger.foreground, theme.accent_danger);
}
