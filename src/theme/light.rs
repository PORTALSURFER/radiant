use super::ThemeTokens;
use crate::gui::types::Rgba8;

pub(super) fn light_palette() -> ThemeTokens {
    ThemeTokens {
        clear_color: rgba(247, 248, 247, 255),
        bg_primary: rgba(247, 248, 247, 255),
        bg_secondary: rgba(239, 241, 240, 255),
        bg_tertiary: rgba(232, 235, 234, 255),
        surface_base: rgba(247, 248, 247, 255),
        surface_raised: rgba(255, 255, 255, 255),
        surface_overlay: rgba(255, 255, 255, 255),
        border: rgba(181, 187, 184, 255),
        border_emphasis: rgba(99, 108, 104, 255),
        grid_strong: rgba(196, 202, 199, 255),
        grid_soft: rgba(221, 225, 223, 255),
        accent_mint: rgba(190, 62, 43, 255),
        accent_copper: rgba(172, 70, 48, 255),
        accent_danger: rgba(190, 45, 34, 255),
        accent_warning: rgba(157, 93, 30, 255),
        highlight_orange: rgba(190, 62, 43, 255),
        highlight_orange_soft: rgba(172, 70, 48, 255),
        highlight_blue: rgba(47, 83, 111, 255),
        highlight_blue_soft: rgba(75, 106, 130, 255),
        highlight_cyan: rgba(20, 105, 98, 255),
        highlight_cyan_soft: rgba(48, 127, 120, 255),
        text_primary: rgba(27, 30, 30, 255),
        text_muted: rgba(82, 91, 87, 255),
        control_disabled_fill: rgba(226, 230, 228, 255),
        state_hover_soft: 0.10,
        state_hover_strong: 0.18,
        state_selected_blend: 0.12,
        state_focus_pulse_blend: 0.20,
        scrim_soft_alpha: 96,
        scrim_modal_alpha: 128,
        motion_speed_transport: 2.6,
        motion_speed_idle: 1.2,
        motion_focus_wave_amp: 0.08,
        motion_focus_text_wave_amp: 0.04,
    }
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
