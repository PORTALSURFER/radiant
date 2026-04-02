//! Semantic color palette values for the native shell.

use crate::gui::types::Rgba8;

use super::tier::LayoutScaleTier;

/// Semantic palette values shared across shell layout tiers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PaletteTokens {
    pub clear_color: Rgba8,
    pub bg_primary: Rgba8,
    pub bg_secondary: Rgba8,
    pub bg_tertiary: Rgba8,
    pub surface_base: Rgba8,
    pub surface_raised: Rgba8,
    pub surface_overlay: Rgba8,
    pub border: Rgba8,
    pub border_emphasis: Rgba8,
    pub source_section_divider: Rgba8,
    pub source_recovery_badge_idle: Rgba8,
    pub source_recovery_badge_active: Rgba8,
    pub control_disabled_fill: Rgba8,
    pub grid_strong: Rgba8,
    pub grid_soft: Rgba8,
    pub accent_mint: Rgba8,
    pub accent_copper: Rgba8,
    pub accent_trash: Rgba8,
    pub accent_warning: Rgba8,
    pub highlight_orange: Rgba8,
    pub highlight_orange_soft: Rgba8,
    pub highlight_blue: Rgba8,
    pub highlight_blue_soft: Rgba8,
    pub highlight_cyan: Rgba8,
    pub highlight_cyan_soft: Rgba8,
    pub text_primary: Rgba8,
    pub text_muted: Rgba8,
}

/// Resolve the shell palette for the requested layout tier.
pub(super) fn palette_for_tier(layout_tier: LayoutScaleTier) -> PaletteTokens {
    let mut palette = PaletteTokens {
        clear_color: rgba(16, 16, 16, 255),
        bg_primary: rgba(10, 10, 10, 255),
        bg_secondary: rgba(18, 18, 18, 255),
        bg_tertiary: rgba(28, 28, 28, 255),
        surface_base: rgba(14, 14, 14, 255),
        surface_raised: rgba(24, 24, 24, 255),
        surface_overlay: rgba(34, 34, 34, 255),
        border: rgba(58, 58, 58, 255),
        border_emphasis: rgba(90, 90, 90, 255),
        source_section_divider: rgba(76, 76, 76, 255),
        source_recovery_badge_idle: rgba(48, 48, 48, 255),
        source_recovery_badge_active: rgba(255, 179, 71, 255),
        control_disabled_fill: rgba(38, 38, 38, 255),
        grid_strong: rgba(74, 74, 74, 255),
        grid_soft: rgba(43, 43, 43, 255),
        accent_mint: rgba(255, 122, 26, 255),
        accent_copper: rgba(255, 194, 71, 255),
        accent_trash: rgba(225, 75, 50, 255),
        accent_warning: rgba(255, 179, 71, 255),
        highlight_orange: rgba(255, 146, 43, 255),
        highlight_orange_soft: rgba(255, 208, 138, 255),
        highlight_blue: rgba(198, 60, 30, 255),
        highlight_blue_soft: rgba(255, 154, 92, 255),
        highlight_cyan: rgba(255, 200, 61, 255),
        highlight_cyan_soft: rgba(255, 224, 138, 255),
        text_primary: rgba(234, 234, 234, 255),
        text_muted: rgba(169, 169, 169, 255),
    };

    match layout_tier {
        LayoutScaleTier::Compact => {
            palette.surface_overlay = rgba(30, 30, 30, 255);
            palette.border_emphasis = rgba(84, 84, 84, 255);
            palette.source_section_divider = rgba(70, 70, 70, 255);
            palette.source_recovery_badge_idle = rgba(44, 44, 44, 255);
            palette.source_recovery_badge_active = palette.accent_warning;
            palette.control_disabled_fill = rgba(34, 34, 34, 255);
        }
        LayoutScaleTier::Wide => {
            palette.surface_overlay = rgba(38, 38, 38, 255);
            palette.border_emphasis = rgba(100, 100, 100, 255);
            palette.source_section_divider = rgba(82, 82, 82, 255);
            palette.source_recovery_badge_idle = rgba(56, 56, 56, 255);
            palette.source_recovery_badge_active = palette.accent_warning;
            palette.control_disabled_fill = rgba(42, 42, 42, 255);
        }
        LayoutScaleTier::Standard => {}
    }

    palette
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
