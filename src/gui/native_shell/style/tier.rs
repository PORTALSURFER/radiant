//! Viewport tier selection and tier-specific non-geometric motion policy.

/// Global baseline size multiplier used to keep default shell density slightly larger.
pub(super) const DEFAULT_UI_SCALE: f32 = 1.06;

/// Viewport density tier used to select a sizing token pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutScaleTier {
    /// Compact layout tuned for narrow plugin windows.
    Compact,
    /// Baseline layout for common desktop window sizes.
    Standard,
    /// Spacious layout tuned for wide desktop windows.
    Wide,
}

impl LayoutScaleTier {
    /// Resolve a scale tier from a logical viewport width.
    pub(crate) fn from_viewport_width(viewport_width: f32) -> Self {
        if viewport_width < 980.0 {
            Self::Compact
        } else if viewport_width > 2100.0 {
            Self::Wide
        } else {
            Self::Standard
        }
    }
}

/// Motion and state-blend policy selected per viewport tier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TierVisualPolicy {
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

/// Resolve per-tier motion and fill-blend policy values.
pub(super) fn visual_policy_for_tier(layout_tier: LayoutScaleTier) -> TierVisualPolicy {
    match layout_tier {
        LayoutScaleTier::Compact => TierVisualPolicy {
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
        LayoutScaleTier::Wide => TierVisualPolicy {
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
        LayoutScaleTier::Standard => TierVisualPolicy {
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

/// Clamp the requested UI scale to a safe range.
pub(super) fn clamp_ui_scale(scale: f32) -> f32 {
    scale.clamp(1.0, 3.0)
}

/// Apply the shell-wide baseline density multiplier to a requested UI scale.
pub(super) fn effective_ui_scale(ui_scale: f32) -> f32 {
    clamp_ui_scale(clamp_ui_scale(ui_scale) * DEFAULT_UI_SCALE)
}
