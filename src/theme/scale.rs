//! Viewport density and UI-scale helpers.

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
