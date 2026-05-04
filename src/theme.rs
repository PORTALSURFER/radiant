//! Generic theme tokens for reusable Radiant widgets, containers, and runtimes.
//!
//! This surface intentionally avoids naming tied to any host application.
//! Adapter-specific chrome colors and shell layout sizing stay outside the
//! reusable token contract.

use crate::gui::types::Rgba8;

/// Global baseline size multiplier used by compatibility shells that want a
/// slightly larger default density than the raw host UI scale.
pub const DEFAULT_UI_SCALE: f32 = 1.06;

/// Viewport-width tier for host surfaces that choose density from available space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportScaleTier {
    /// Compact layout tuned for narrow windows or embedded panels.
    Compact,
    /// Baseline layout for common desktop window sizes.
    Standard,
    /// Spacious layout tuned for wide desktop windows.
    Wide,
}

impl ViewportScaleTier {
    /// Resolve a scale tier from a logical viewport width.
    pub fn from_viewport_width(viewport_width: f32) -> Self {
        if viewport_width < 980.0 {
            Self::Compact
        } else if viewport_width > 2100.0 {
            Self::Wide
        } else {
            Self::Standard
        }
    }
}

/// Clamp the requested UI scale to a safe range for geometry and typography.
pub fn clamp_ui_scale(scale: f32) -> f32 {
    scale.clamp(1.0, 3.0)
}

/// Apply Radiant's baseline density multiplier to a requested UI scale.
pub fn effective_ui_scale(ui_scale: f32) -> f32 {
    clamp_ui_scale(clamp_ui_scale(ui_scale) * DEFAULT_UI_SCALE)
}

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

/// Generic core theme bundle for reusable Radiant primitives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeTokens {
    /// Frame clear color for host backends that paint a root background.
    pub clear_color: Rgba8,
    /// Strongest background fill used behind the main content area.
    pub bg_primary: Rgba8,
    /// Secondary background fill used for recessed regions.
    pub bg_secondary: Rgba8,
    /// Tertiary background fill used for elevated rows and controls.
    pub bg_tertiary: Rgba8,
    /// Base surface fill for content-heavy containers.
    pub surface_base: Rgba8,
    /// Raised surface fill for controls and grouped chrome.
    pub surface_raised: Rgba8,
    /// Overlay surface fill for popovers, dialogs, and menus.
    pub surface_overlay: Rgba8,
    /// Default border color.
    pub border: Rgba8,
    /// Higher-contrast border color for emphasized boundaries.
    pub border_emphasis: Rgba8,
    /// Primary grid or separator line color.
    pub grid_strong: Rgba8,
    /// Secondary grid or separator line color.
    pub grid_soft: Rgba8,
    /// Primary accent color for active or selected emphasis.
    pub accent_mint: Rgba8,
    /// Secondary accent color for alternate active emphasis.
    pub accent_copper: Rgba8,
    /// Danger accent for destructive actions and warnings.
    pub accent_danger: Rgba8,
    /// Warning accent for cautionary actions and hover emphasis.
    pub accent_warning: Rgba8,
    /// Strong warm highlight used for emphasized focus or transport hints.
    pub highlight_orange: Rgba8,
    /// Softer warm highlight used for secondary emphasis.
    pub highlight_orange_soft: Rgba8,
    /// Strong cool highlight used for informational emphasis.
    pub highlight_blue: Rgba8,
    /// Softer cool highlight used for secondary informational emphasis.
    pub highlight_blue_soft: Rgba8,
    /// Strong success highlight used for positive active emphasis.
    pub highlight_cyan: Rgba8,
    /// Softer success highlight used for secondary positive emphasis.
    pub highlight_cyan_soft: Rgba8,
    /// High-contrast primary text color.
    pub text_primary: Rgba8,
    /// Secondary muted text color.
    pub text_muted: Rgba8,
    /// Disabled control fill used when widgets remain visible but inactive.
    pub control_disabled_fill: Rgba8,
    /// Blend amount for subtle hover states.
    pub state_hover_soft: f32,
    /// Blend amount for stronger hover states.
    pub state_hover_strong: f32,
    /// Blend amount for selected-state fills.
    pub state_selected_blend: f32,
    /// Blend amount for pulsing focused-state fills and borders.
    pub state_focus_pulse_blend: f32,
    /// Alpha used by non-modal background scrims.
    pub scrim_soft_alpha: u8,
    /// Alpha used by modal-blocking background scrims.
    pub scrim_modal_alpha: u8,
    /// Pulse speed used while transport or other active motion is running.
    pub motion_speed_transport: f32,
    /// Pulse speed used for idle focus emphasis.
    pub motion_speed_idle: f32,
    /// Additional blend amplitude injected into focused fills.
    pub motion_focus_wave_amp: f32,
    /// Additional blend amplitude injected into focused text emphasis.
    pub motion_focus_text_wave_amp: f32,
}

impl ThemeTokens {
    /// Return the baseline dark theme used by the generic Radiant surface.
    pub fn dark() -> Self {
        Self::dark_for_tier(ViewportScaleTier::Standard)
    }

    /// Return the dark theme adjusted for a viewport width tier.
    pub fn dark_for_tier(layout_tier: ViewportScaleTier) -> Self {
        let mut theme = Self::dark_base();
        theme.apply_visual_policy(visual_policy_for_tier(layout_tier));
        theme
    }

    /// Return the dark theme adjusted for a logical viewport width.
    pub fn dark_for_viewport_width(viewport_width: f32) -> Self {
        Self::dark_for_tier(ViewportScaleTier::from_viewport_width(viewport_width))
    }

    fn dark_base() -> Self {
        Self {
            clear_color: rgba(16, 16, 16, 255),
            bg_primary: rgba(10, 10, 10, 255),
            bg_secondary: rgba(18, 18, 18, 255),
            bg_tertiary: rgba(28, 28, 28, 255),
            surface_base: rgba(14, 14, 14, 255),
            surface_raised: rgba(24, 24, 24, 255),
            surface_overlay: rgba(34, 34, 34, 255),
            border: rgba(58, 58, 58, 255),
            border_emphasis: rgba(90, 90, 90, 255),
            grid_strong: rgba(74, 74, 74, 255),
            grid_soft: rgba(43, 43, 43, 255),
            accent_mint: rgba(196, 122, 63, 255),
            accent_copper: rgba(212, 170, 96, 255),
            accent_danger: rgba(171, 78, 57, 255),
            accent_warning: rgba(224, 153, 84, 255),
            highlight_orange: rgba(182, 104, 52, 255),
            highlight_orange_soft: rgba(198, 150, 103, 255),
            highlight_blue: rgba(150, 77, 52, 255),
            highlight_blue_soft: rgba(188, 128, 89, 255),
            highlight_cyan: rgba(214, 160, 83, 255),
            highlight_cyan_soft: rgba(224, 184, 126, 255),
            text_primary: rgba(234, 234, 234, 255),
            text_muted: rgba(169, 169, 169, 255),
            control_disabled_fill: rgba(38, 38, 38, 255),
            state_hover_soft: 0.12,
            state_hover_strong: 0.20,
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

    fn apply_visual_policy(&mut self, policy: TierVisualPolicy) {
        self.state_hover_soft = policy.state_hover_soft;
        self.state_hover_strong = policy.state_hover_strong;
        self.state_selected_blend = policy.state_selected_blend;
        self.state_focus_pulse_blend = policy.state_focus_pulse_blend;
        self.scrim_soft_alpha = policy.scrim_soft_alpha;
        self.scrim_modal_alpha = policy.scrim_modal_alpha;
        self.motion_speed_transport = policy.motion_speed_transport;
        self.motion_speed_idle = policy.motion_speed_idle;
        self.motion_focus_wave_amp = policy.motion_focus_wave_amp;
        self.motion_focus_text_wave_amp = policy.motion_focus_text_wave_amp;
    }
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::dark()
    }
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_UI_SCALE, ThemeTokens, ViewportScaleTier, effective_ui_scale};

    #[test]
    fn viewport_width_maps_to_scale_tiers() {
        assert_eq!(
            ViewportScaleTier::from_viewport_width(820.0),
            ViewportScaleTier::Compact
        );
        assert_eq!(
            ViewportScaleTier::from_viewport_width(1280.0),
            ViewportScaleTier::Standard
        );
        assert_eq!(
            ViewportScaleTier::from_viewport_width(2300.0),
            ViewportScaleTier::Wide
        );
    }

    #[test]
    fn effective_ui_scale_clamps_and_applies_default_multiplier() {
        assert_eq!(effective_ui_scale(0.5), DEFAULT_UI_SCALE);
        assert!((effective_ui_scale(1.5) - (1.5 * DEFAULT_UI_SCALE)).abs() < 0.0001);
        assert_eq!(effective_ui_scale(4.0), 3.0);
    }

    #[test]
    fn dark_theme_uses_distinct_primary_and_overlay_surfaces() {
        let theme = ThemeTokens::dark();
        assert_ne!(theme.surface_base, theme.surface_overlay);
        assert_ne!(theme.bg_primary, theme.bg_tertiary);
    }

    #[test]
    fn dark_theme_exposes_non_zero_motion_and_state_layers() {
        let theme = ThemeTokens::dark();
        assert!(theme.state_hover_soft > 0.0);
        assert!(theme.state_hover_strong >= theme.state_hover_soft);
        assert!(theme.motion_speed_transport > 0.0);
        assert!(theme.motion_focus_wave_amp > 0.0);
    }

    #[test]
    fn dark_theme_resolves_viewport_tier_motion_without_compatibility_shell() {
        let compact = ThemeTokens::dark_for_viewport_width(820.0);
        let wide = ThemeTokens::dark_for_viewport_width(2300.0);
        assert!(compact.motion_speed_transport < wide.motion_speed_transport);
        assert!(compact.scrim_modal_alpha < wide.scrim_modal_alpha);
    }
}
