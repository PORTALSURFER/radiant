//! Generic theme tokens for reusable Radiant widgets, containers, and runtimes.
//!
//! This surface intentionally avoids naming tied to any host application.
//! Adapter-specific chrome colors and shell layout sizing stay outside the
//! reusable token contract.

mod dark;
mod scale;
mod visual_policy;

use crate::gui::types::Rgba8;
use dark::dark_palette;
pub use scale::{DEFAULT_UI_SCALE, ViewportScaleTier, clamp_ui_scale, effective_ui_scale};
use visual_policy::{TierVisualPolicy, visual_policy_for_tier};

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
        let mut theme = dark_palette();
        theme.apply_visual_policy(visual_policy_for_tier(layout_tier));
        theme
    }

    /// Return the dark theme adjusted for a logical viewport width.
    pub fn dark_for_viewport_width(viewport_width: f32) -> Self {
        Self::dark_for_tier(ViewportScaleTier::from_viewport_width(viewport_width))
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

#[cfg(test)]
#[path = "theme/tests.rs"]
mod tests;
