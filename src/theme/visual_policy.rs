//! Tier-specific visual motion and state-layer policy.

use super::ViewportScaleTier;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TierVisualPolicy {
    pub state_hover_soft: f32,
    pub state_hover_strong: f32,
    pub state_selected_blend: f32,
    pub state_focus_pulse_blend: f32,
    pub motion_speed_transport: f32,
    pub motion_speed_idle: f32,
    pub motion_focus_wave_amp: f32,
    pub motion_focus_text_wave_amp: f32,
    pub scrim_soft_alpha: u8,
    pub scrim_modal_alpha: u8,
}

pub(crate) fn visual_policy_for_tier(layout_tier: ViewportScaleTier) -> TierVisualPolicy {
    match layout_tier {
        ViewportScaleTier::Compact => TierVisualPolicy {
            state_hover_soft: 0.10,
            state_hover_strong: 0.16,
            state_selected_blend: 0.10,
            state_focus_pulse_blend: 0.20,
            motion_speed_transport: 2.2,
            motion_speed_idle: 1.0,
            motion_focus_wave_amp: 0.06,
            motion_focus_text_wave_amp: 0.03,
            scrim_soft_alpha: 164,
            scrim_modal_alpha: 180,
        },
        ViewportScaleTier::Wide => TierVisualPolicy {
            state_hover_soft: 0.12,
            state_hover_strong: 0.20,
            state_selected_blend: 0.13,
            state_focus_pulse_blend: 0.25,
            motion_speed_transport: 2.8,
            motion_speed_idle: 1.2,
            motion_focus_wave_amp: 0.08,
            motion_focus_text_wave_amp: 0.04,
            scrim_soft_alpha: 180,
            scrim_modal_alpha: 196,
        },
        ViewportScaleTier::Standard => TierVisualPolicy {
            state_hover_soft: 0.12,
            state_hover_strong: 0.20,
            state_selected_blend: 0.12,
            state_focus_pulse_blend: 0.24,
            motion_speed_transport: 2.6,
            motion_speed_idle: 1.2,
            motion_focus_wave_amp: 0.08,
            motion_focus_text_wave_amp: 0.04,
            scrim_soft_alpha: 172,
            scrim_modal_alpha: 188,
        },
    }
}
