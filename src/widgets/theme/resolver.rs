//! Semantic widget-style resolution against core theme tokens.

use crate::{
    gui::types::Rgba8,
    theme::ThemeTokens,
    widgets::{WidgetProminence, WidgetState, WidgetStyle, WidgetTone},
};

use super::WidgetVisualTokens;

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
    } else if matches!(
        (style.prominence, style.tone),
        (WidgetProminence::Strong, WidgetTone::Warning)
    ) {
        theme.bg_primary
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
