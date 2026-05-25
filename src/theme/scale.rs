//! Viewport density and UI-scale helpers.

/// Global baseline size multiplier applied before host UI scale.
pub const DEFAULT_UI_SCALE: f32 = 1.0;

/// Sanitized native DPI scale used to convert between logical points and physical pixels.
///
/// Radiant layout, widget input, and paint plans use logical points. Native
/// renderers keep GPU surfaces in physical pixels and use this model at the
/// backend boundary so application code does not need to special-case monitor
/// scale changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DpiScale {
    factor: f32,
}

impl DpiScale {
    /// Baseline 1:1 scale.
    pub const ONE: Self = Self { factor: 1.0 };

    /// Build a DPI scale from a platform scale factor, rejecting non-finite and non-positive input.
    pub fn new(factor: f64) -> Self {
        let factor = factor as f32;
        if factor.is_finite() && factor > 0.0 {
            Self { factor }
        } else {
            Self::ONE
        }
    }

    /// Return the scale factor as physical pixels per logical point.
    pub fn factor(self) -> f32 {
        self.factor
    }

    /// Convert a physical-pixel coordinate or size component to logical points.
    pub fn physical_to_logical(self, value: f32) -> f32 {
        value / self.factor
    }

    /// Convert a logical-point coordinate or size component to physical pixels.
    pub fn logical_to_physical(self, value: f32) -> f32 {
        value * self.factor
    }
}

impl Default for DpiScale {
    fn default() -> Self {
        Self::ONE
    }
}

impl Eq for DpiScale {}

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
