use crate::gui::types::Rect;

/// Named fields for constructing a reusable vertical value axis projector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalValueAxisParts {
    /// Rect used as the vertical projection area.
    pub rect: Rect,
    /// Value projected at the bottom edge of the rect.
    pub min: f32,
    /// Value projected at the top edge of the rect.
    pub max: f32,
}

impl VerticalValueAxisParts {
    /// Build axis parts for a visible value span.
    pub const fn new(rect: Rect, min: f32, max: f32) -> Self {
        Self { rect, min, max }
    }
}

/// Reusable vertical projector for editor values where low values sit at the bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalValueAxis {
    /// Rect used as the vertical projection area.
    pub rect: Rect,
    /// Value projected at the bottom edge of the rect.
    pub min: f32,
    /// Value projected at the top edge of the rect.
    pub max: f32,
}

impl VerticalValueAxis {
    /// Build a vertical value axis from named parts.
    pub const fn from_parts(parts: VerticalValueAxisParts) -> Self {
        Self {
            rect: parts.rect,
            min: parts.min,
            max: parts.max,
        }
    }

    /// Build a vertical value axis for a visible value span.
    pub const fn new(rect: Rect, min: f32, max: f32) -> Self {
        Self::from_parts(VerticalValueAxisParts::new(rect, min, max))
    }

    /// Project a value into y coordinates, clamped to the visible span.
    pub fn y_for_value(self, value: f32) -> f32 {
        self.y_for_value_unclamped(self.clamp_value(value))
    }

    /// Project a value into y coordinates without clamping the value.
    pub fn y_for_value_unclamped(self, value: f32) -> f32 {
        self.rect
            .y_for_ratio_from_bottom_unclamped(self.ratio_for_value_unclamped(value))
    }

    /// Convert a y coordinate into a clamped value.
    pub fn value_for_y(self, y: f32) -> f32 {
        self.clamp_value(self.value_for_y_unclamped(y))
    }

    /// Convert a y coordinate into a value without clamping the y coordinate.
    pub fn value_for_y_unclamped(self, y: f32) -> f32 {
        let height = self.rect.height();
        if !y.is_finite() || !height.is_finite() || height <= f32::EPSILON {
            return self.min_or_zero();
        }
        self.min_or_zero() + ((self.rect.max.y - y) / height) * self.value_span()
    }

    /// Return the visible value span, using a safe minimum for degenerate spans.
    pub fn value_span(self) -> f32 {
        let span = self.max - self.min;
        if span.is_finite() && span.abs() > f32::EPSILON {
            span
        } else {
            1.0
        }
    }

    fn ratio_for_value_unclamped(self, value: f32) -> f32 {
        if !value.is_finite() {
            return 0.0;
        }
        (value - self.min_or_zero()) / self.value_span()
    }

    fn clamp_value(self, value: f32) -> f32 {
        if !value.is_finite() {
            return self.min_or_zero();
        }
        value.clamp(self.min.min(self.max), self.min.max(self.max))
    }

    fn min_or_zero(self) -> f32 {
        if self.min.is_finite() { self.min } else { 0.0 }
    }
}
