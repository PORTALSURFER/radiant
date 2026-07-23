use super::ThemeTokens;
use crate::gui::types::Rgba8;

pub(super) fn dark_palette() -> ThemeTokens {
    ThemeTokens {
        clear_color: rgba(27, 30, 30, 255),
        bg_primary: rgba(27, 30, 30, 255),
        bg_secondary: rgba(27, 30, 30, 255),
        bg_tertiary: rgba(27, 30, 30, 255),
        surface_base: rgba(27, 30, 30, 255),
        surface_raised: rgba(27, 30, 30, 255),
        surface_overlay: rgba(42, 45, 45, 255),
        border: rgba(58, 61, 61, 255),
        border_emphasis: rgba(64, 67, 66, 255),
        grid_strong: rgba(54, 57, 57, 255),
        grid_soft: rgba(40, 43, 43, 255),
        accent_mint: rgba(233, 88, 67, 255),
        accent_copper: rgba(241, 108, 86, 255),
        accent_danger: rgba(239, 76, 61, 255),
        accent_warning: rgba(217, 151, 95, 255),
        highlight_orange: rgba(233, 88, 67, 255),
        highlight_orange_soft: rgba(241, 121, 98, 255),
        highlight_blue: rgba(153, 155, 154, 255),
        highlight_blue_soft: rgba(112, 115, 114, 255),
        highlight_cyan: rgba(174, 176, 173, 255),
        highlight_cyan_soft: rgba(126, 129, 127, 255),
        text_primary: rgba(216, 215, 211, 255),
        text_muted: rgba(153, 155, 154, 255),
        control_disabled_fill: rgba(36, 40, 41, 255),
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
