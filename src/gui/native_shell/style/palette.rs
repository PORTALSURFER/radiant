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
        clear_color: rgba(38, 38, 37, 255),
        bg_primary: rgba(19, 19, 19, 255),
        bg_secondary: rgba(26, 26, 25, 255),
        bg_tertiary: rgba(37, 37, 36, 255),
        surface_base: rgba(19, 19, 19, 255),
        surface_raised: rgba(31, 30, 29, 255),
        surface_overlay: rgba(43, 42, 40, 255),
        border: rgba(75, 74, 72, 255),
        border_emphasis: rgba(119, 118, 116, 255),
        source_section_divider: rgba(96, 94, 91, 255),
        source_recovery_badge_idle: rgba(63, 62, 60, 255),
        source_recovery_badge_active: rgba(223, 195, 121, 255),
        control_disabled_fill: rgba(50, 50, 48, 255),
        grid_strong: rgba(90, 89, 86, 255),
        grid_soft: rgba(60, 59, 57, 255),
        accent_mint: rgba(193, 193, 169, 255),
        accent_copper: rgba(227, 195, 122, 255),
        accent_trash: rgba(214, 110, 106, 255),
        accent_warning: rgba(239, 211, 149, 255),
        highlight_orange: rgba(223, 176, 92, 255),
        highlight_orange_soft: rgba(236, 204, 142, 255),
        highlight_blue: rgba(164, 164, 164, 255),
        highlight_blue_soft: rgba(184, 184, 184, 255),
        highlight_cyan: rgba(193, 193, 169, 255),
        highlight_cyan_soft: rgba(227, 226, 195, 255),
        text_primary: rgba(238, 234, 224, 255),
        text_muted: rgba(185, 180, 166, 255),
    };

    match layout_tier {
        LayoutScaleTier::Compact => {
            palette.surface_overlay = rgba(40, 39, 37, 255);
            palette.border_emphasis = rgba(112, 111, 108, 255);
            palette.source_section_divider = rgba(88, 86, 83, 255);
            palette.source_recovery_badge_idle = rgba(57, 56, 53, 255);
            palette.source_recovery_badge_active = palette.accent_warning;
            palette.control_disabled_fill = rgba(47, 46, 44, 255);
        }
        LayoutScaleTier::Wide => {
            palette.surface_overlay = rgba(47, 46, 44, 255);
            palette.border_emphasis = rgba(128, 126, 123, 255);
            palette.source_section_divider = rgba(102, 100, 96, 255);
            palette.source_recovery_badge_idle = rgba(74, 72, 69, 255);
            palette.source_recovery_badge_active = palette.accent_warning;
            palette.control_disabled_fill = rgba(53, 52, 49, 255);
        }
        LayoutScaleTier::Standard => {}
    }

    palette
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
