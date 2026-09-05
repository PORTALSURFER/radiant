//! Constraint primitives for the slot-based layout engine.

#[cfg(test)]
#[path = "constraints/tests.rs"]
mod tests;

/// Explicit min/max bounds used to build layout constraints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstraintsParts {
    /// Minimum allowed width in logical pixels.
    pub min_w: f32,
    /// Maximum allowed width in logical pixels.
    pub max_w: f32,
    /// Minimum allowed height in logical pixels.
    pub min_h: f32,
    /// Maximum allowed height in logical pixels.
    pub max_h: f32,
}

/// Axis-aligned min/max bounds used during measurement and layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
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
    pub fn unconstrained() -> Self {
        Self::from_parts(ConstraintsParts {
            min_w: 0.0,
            max_w: f32::INFINITY,
            min_h: 0.0,
            max_h: f32::INFINITY,
        })
    }

    /// Build normalized constraints from named raw bounds.
    pub fn from_parts(parts: ConstraintsParts) -> Self {
        Self {
            min_w: parts.min_w,
            max_w: parts.max_w,
            min_h: parts.min_h,
            max_h: parts.max_h,
        }
        .normalized()
    }

    /// Build normalized constraints from raw values for internal layout helpers.
    pub(crate) fn new(min_w: f32, max_w: f32, min_h: f32, max_h: f32) -> Self {
        Self::from_parts(ConstraintsParts {
            min_w,
            max_w,
            min_h,
            max_h,
        })
    }

    /// Return a copy with normalized and clamped ranges.
    pub fn normalized(self) -> Self {
        let (min_w, max_w) =
            super::validated_geometry::normalize_constraint_axis(self.min_w, self.max_w);
        let (min_h, max_h) =
            super::validated_geometry::normalize_constraint_axis(self.min_h, self.max_h);
        Self {
            min_w,
            max_w,
            min_h,
            max_h,
        }
    }

    /// Clamp a width after normalizing directly supplied range bounds.
    pub fn clamp_w(self, width: f32) -> f32 {
        let (minimum, maximum) =
            super::validated_geometry::normalize_constraint_axis(self.min_w, self.max_w);
        width.clamp(minimum, maximum)
    }

    /// Clamp a height after normalizing directly supplied range bounds.
    pub fn clamp_h(self, height: f32) -> f32 {
        let (minimum, maximum) =
            super::validated_geometry::normalize_constraint_axis(self.min_h, self.max_h);
        height.clamp(minimum, maximum)
    }

    /// Shrink available space by insets while preserving min <= max.
    pub fn inset(self, inset_x: f32, inset_y: f32) -> Self {
        let reduced_w = inset_maximum(self.max_w, inset_x);
        let reduced_h = inset_maximum(self.max_h, inset_y);
        Self::new(0.0, reduced_w, 0.0, reduced_h)
    }
}

fn inset_maximum(maximum: f32, inset: f32) -> f32 {
    if !inset.is_finite() {
        return 0.0;
    }

    if maximum == f32::INFINITY {
        return f32::INFINITY;
    }
    if !maximum.is_finite() {
        return 0.0;
    }

    let doubled = inset * 2.0;
    if !doubled.is_finite() {
        return if maximum.is_finite() && inset.is_sign_negative() {
            f32::MAX
        } else {
            0.0
        };
    }

    let reduced = maximum - doubled;
    if reduced.is_finite() {
        reduced.max(0.0)
    } else if doubled.is_sign_negative() {
        f32::MAX
    } else {
        0.0
    }
}
