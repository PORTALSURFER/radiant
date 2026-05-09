use super::ThemeTokens;
use crate::gui::types::Rgba8;

pub(super) fn dark_palette() -> ThemeTokens {
    ThemeTokens {
        clear_color: rgba(24, 24, 24, 255),
        bg_primary: rgba(17, 17, 17, 255),
        bg_secondary: rgba(24, 24, 24, 255),
        bg_tertiary: rgba(32, 32, 32, 255),
        surface_base: rgba(23, 23, 23, 255),
        surface_raised: rgba(35, 35, 35, 255),
        surface_overlay: rgba(47, 47, 47, 255),
        border: rgba(54, 54, 54, 255),
        border_emphasis: rgba(88, 88, 88, 255),
        grid_strong: rgba(68, 68, 68, 255),
        grid_soft: rgba(40, 40, 40, 255),
        accent_mint: rgba(246, 82, 58, 255),
        accent_copper: rgba(255, 119, 82, 255),
        accent_danger: rgba(255, 82, 52, 255),
        accent_warning: rgba(255, 160, 82, 255),
        highlight_orange: rgba(255, 94, 62, 255),
        highlight_orange_soft: rgba(255, 143, 108, 255),
        highlight_blue: rgba(130, 130, 130, 255),
        highlight_blue_soft: rgba(102, 102, 102, 255),
        highlight_cyan: rgba(142, 142, 142, 255),
        highlight_cyan_soft: rgba(112, 112, 112, 255),
        text_primary: rgba(238, 238, 238, 255),
        text_muted: rgba(145, 145, 145, 255),
        control_disabled_fill: rgba(36, 36, 36, 255),
        state_hover_soft: 0.14,
        state_hover_strong: 0.28,
        state_selected_blend: 0.12,
        state_focus_pulse_blend: 0.24,
        scrim_soft_alpha: 172,
        scrim_modal_alpha: 188,
        motion_speed_transport: 2.6,
        motion_speed_idle: 1.2,
        motion_focus_wave_amp: 0.08,
        motion_focus_text_wave_amp: 0.04,
    }
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
