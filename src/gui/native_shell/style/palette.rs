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
        clear_color: rgba(20, 20, 20, 255),
        bg_primary: rgba(13, 13, 13, 255),
        bg_secondary: rgba(18, 18, 18, 255),
        bg_tertiary: rgba(25, 25, 25, 255),
        surface_base: rgba(15, 15, 15, 255),
        surface_raised: rgba(22, 22, 22, 255),
        surface_overlay: rgba(30, 30, 30, 255),
        border: rgba(62, 62, 62, 255),
        border_emphasis: rgba(95, 95, 95, 255),
        source_section_divider: rgba(78, 78, 78, 255),
        source_recovery_badge_idle: rgba(47, 47, 47, 255),
        source_recovery_badge_active: rgba(242, 182, 92, 255),
        control_disabled_fill: rgba(37, 37, 37, 255),
        grid_strong: rgba(74, 74, 74, 255),
        grid_soft: rgba(45, 45, 45, 255),
        accent_mint: rgba(102, 194, 255, 255),
        accent_copper: rgba(92, 157, 255, 255),
        accent_trash: rgba(232, 101, 101, 255),
        accent_warning: rgba(242, 182, 92, 255),
        highlight_orange: rgba(255, 168, 76, 255),
        highlight_orange_soft: rgba(255, 204, 125, 255),
        highlight_blue: rgba(71, 133, 255, 255),
        highlight_blue_soft: rgba(123, 180, 255, 255),
        highlight_cyan: rgba(84, 214, 255, 255),
        highlight_cyan_soft: rgba(156, 231, 255, 255),
        text_primary: rgba(232, 232, 232, 255),
        text_muted: rgba(166, 166, 166, 255),
    };

    match layout_tier {
        LayoutScaleTier::Compact => {
            palette.surface_overlay = rgba(28, 28, 28, 255);
            palette.border_emphasis = rgba(89, 89, 89, 255);
            palette.source_section_divider = rgba(73, 73, 73, 255);
            palette.source_recovery_badge_idle = rgba(42, 42, 42, 255);
            palette.source_recovery_badge_active = palette.accent_warning;
            palette.control_disabled_fill = rgba(34, 34, 34, 255);
        }
        LayoutScaleTier::Wide => {
            palette.surface_overlay = rgba(34, 34, 34, 255);
            palette.border_emphasis = rgba(104, 104, 104, 255);
            palette.source_section_divider = rgba(85, 85, 85, 255);
            palette.source_recovery_badge_idle = rgba(55, 55, 55, 255);
            palette.source_recovery_badge_active = palette.accent_warning;
            palette.control_disabled_fill = rgba(41, 41, 41, 255);
        }
        LayoutScaleTier::Standard => {}
    }

    palette
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
