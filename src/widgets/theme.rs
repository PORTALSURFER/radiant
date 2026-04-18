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
/// states rather than to Sempal shell chrome names.
pub fn resolve_widget_visual_tokens(
    theme: &ThemeTokens,
    style: WidgetStyle,
    state: WidgetState,
) -> WidgetVisualTokens {
    let emphasis = tone_color(theme, style.tone);
    let fill = if state.disabled {
        theme.control_disabled_fill
    } else if state.pressed || state.active || state.selected {
        accent_fill(theme, emphasis)
    } else {
        prominence_fill(theme, style.prominence)
    };
    let foreground = if state.disabled {
        theme.text_muted
    } else if matches!(style.prominence, WidgetProminence::Strong)
        || state.pressed
        || state.active
        || state.selected
    {
        theme.text_primary
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
        WidgetTone::Danger => theme.accent_trash,
    }
}

fn prominence_fill(theme: &ThemeTokens, prominence: WidgetProminence) -> Rgba8 {
    match prominence {
        WidgetProminence::Subtle => theme.surface_base,
        WidgetProminence::Normal => theme.surface_raised,
        WidgetProminence::Strong => theme.surface_overlay,
    }
}

fn accent_fill(theme: &ThemeTokens, emphasis: Rgba8) -> Rgba8 {
    if emphasis == theme.text_primary {
        theme.surface_raised
    } else {
        emphasis
    }
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
}
