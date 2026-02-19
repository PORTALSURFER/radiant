//! Constraint primitives for the slot-based layout engine.

/// Axis-aligned min/max bounds used during measurement and layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Constraints {
    /// Minimum allowed width in logical pixels.
    pub min_w: f32,
    /// Maximum allowed width in logical pixels.
    pub max_w: f32,
    /// Minimum allowed height in logical pixels.
    pub min_h: f32,
    /// Maximum allowed height in logical pixels.
    pub max_h: f32,
}

impl Constraints {
    /// Build unconstrained bounds.
    pub(crate) fn unconstrained() -> Self {
        Self {
            min_w: 0.0,
            max_w: f32::INFINITY,
            min_h: 0.0,
            max_h: f32::INFINITY,
        }
    }

    /// Build normalized constraints from raw values.
    pub(crate) fn new(min_w: f32, max_w: f32, min_h: f32, max_h: f32) -> Self {
        Self {
            min_w,
            max_w,
            min_h,
            max_h,
        }
        .normalized()
    }

    /// Return a copy with normalized and clamped ranges.
    pub(crate) fn normalized(self) -> Self {
        let min_w = self.min_w.max(0.0);
        let min_h = self.min_h.max(0.0);
        let max_w = self.max_w.max(min_w);
        let max_h = self.max_h.max(min_h);
        Self {
            min_w,
            max_w,
            min_h,
            max_h,
        }
    }

    /// Clamp a width to this range.
    pub(crate) fn clamp_w(self, width: f32) -> f32 {
        width.clamp(self.min_w, self.max_w)
    }

    /// Clamp a height to this range.
    pub(crate) fn clamp_h(self, height: f32) -> f32 {
        height.clamp(self.min_h, self.max_h)
    }

    /// Shrink available space by insets while preserving min <= max.
    pub(crate) fn inset(self, inset_x: f32, inset_y: f32) -> Self {
        let reduced_w = (self.max_w - (inset_x * 2.0)).max(0.0);
        let reduced_h = (self.max_h - (inset_y * 2.0)).max(0.0);
        Self::new(0.0, reduced_w, 0.0, reduced_h)
    }
}

#[cfg(test)]
mod tests {
    use super::Constraints;

    #[test]
    fn constraints_normalize_invalid_ranges() {
        let normalized = Constraints::new(-10.0, 4.0, 8.0, 2.0);
        assert_eq!(normalized.min_w, 0.0);
        assert_eq!(normalized.max_w, 4.0);
        assert_eq!(normalized.min_h, 8.0);
        assert_eq!(normalized.max_h, 8.0);
    }
}
