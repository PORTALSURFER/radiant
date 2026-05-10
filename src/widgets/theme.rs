//! Generic widget-theme helpers built on top of [`crate::theme`].
//!
//! These helpers let reusable widgets resolve a small visual treatment from the
//! core token surface without importing compatibility shell styling modules.

use crate::theme::ThemeTokens;

use super::{WidgetProminence, WidgetState, WidgetStyle, WidgetTone};
use crate::gui::types::Rgba8;

/// Resolved generic widget colors for a specific theme, style, and state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetVisualTokens {
    /// Background fill for the widget body.
    pub fill: Rgba8,
    /// Text or icon foreground color.
    pub foreground: Rgba8,
    /// Border color around the widget body.
    pub border: Rgba8,
    /// Optional focus ring or selected outline color.
    pub emphasis: Rgba8,
}

/// Resolve generic widget colors from the core theme surface.
///
/// This keeps reusable widget styling tied to semantic tones and interaction
/// states rather than to host-specific shell chrome names.
pub fn resolve_widget_visual_tokens(
    theme: &ThemeTokens,
    style: WidgetStyle,
    state: WidgetState,
) -> WidgetVisualTokens {
    let emphasis = tone_color(theme, style.tone);
    let base_fill = if state.disabled {
        theme.control_disabled_fill
    } else if matches!(style.prominence, WidgetProminence::Strong)
        && !matches!(style.tone, WidgetTone::Neutral)
    {
        emphasis
    } else if state.pressed || state.active || state.selected {
        active_fill(theme, emphasis)
    } else if matches!(style.prominence, WidgetProminence::Subtle)
        && !matches!(style.tone, WidgetTone::Neutral)
    {
        blend(theme.surface_raised, theme.surface_overlay, 0.45)
    } else {
        prominence_fill(theme, style.prominence)
    };
    let fill = if state.disabled {
        base_fill
    } else if state.pressed {
        blend(base_fill, theme.bg_primary, 0.18)
    } else if state.hovered {
        blend(
            base_fill,
            theme.surface_overlay,
            match style.prominence {
                WidgetProminence::Subtle => theme.state_hover_soft,
                WidgetProminence::Normal | WidgetProminence::Strong => theme.state_hover_strong,
            },
        )
    } else {
        base_fill
    };
    let foreground = if state.disabled {
        theme.text_muted
    } else if state.pressed
        || state.active
        || state.selected
        || (matches!(style.prominence, WidgetProminence::Strong)
            && !matches!(style.tone, WidgetTone::Neutral))
    {
        theme.text_primary
    } else if matches!(
        (style.prominence, style.tone),
        (WidgetProminence::Subtle, WidgetTone::Neutral)
    ) {
        theme.text_muted
    } else {
        match style.tone {
            WidgetTone::Neutral => theme.text_primary,
            _ => emphasis,
        }
    };
    let border = if state.disabled {
        theme.grid_soft
    } else if state.focused {
        theme.border_emphasis
    } else if state.pressed || state.hovered {
        blend(theme.border_emphasis, emphasis, theme.state_hover_soft)
    } else if matches!(style.prominence, WidgetProminence::Subtle) {
        theme.grid_soft
    } else {
        theme.border
    };

    WidgetVisualTokens {
        fill,
        foreground,
        border,
        emphasis,
    }
}

fn tone_color(theme: &ThemeTokens, tone: WidgetTone) -> Rgba8 {
    match tone {
        WidgetTone::Neutral => theme.text_primary,
        WidgetTone::Accent => theme.accent_mint,
        WidgetTone::Success => theme.highlight_cyan,
        WidgetTone::Warning => theme.accent_warning,
        WidgetTone::Danger => theme.accent_danger,
    }
}

fn prominence_fill(theme: &ThemeTokens, prominence: WidgetProminence) -> Rgba8 {
    match prominence {
        WidgetProminence::Subtle => theme.bg_primary,
        WidgetProminence::Normal => theme.surface_raised,
        WidgetProminence::Strong => theme.surface_overlay,
    }
}

fn active_fill(theme: &ThemeTokens, emphasis: Rgba8) -> Rgba8 {
    if emphasis == theme.text_primary {
        theme.surface_overlay
    } else {
        emphasis
    }
}

fn blend(from: Rgba8, to: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    Rgba8 {
        r: blend_channel(from.r, to.r, amount),
        g: blend_channel(from.g, to.g, amount),
        b: blend_channel(from.b, to.b, amount),
        a: blend_channel(from.a, to.a, amount),
    }
}

fn blend_channel(from: u8, to: u8, amount: f32) -> u8 {
    ((from as f32) + (((to as f32) - (from as f32)) * amount)).round() as u8
}

#[cfg(test)]
mod tests {
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
    fn neutral_active_controls_use_selected_surface() {
        let theme = ThemeTokens::default();
        let base =
            resolve_widget_visual_tokens(&theme, WidgetStyle::default(), WidgetState::default());
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
        let base =
            resolve_widget_visual_tokens(&theme, WidgetStyle::default(), WidgetState::default());
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
}
