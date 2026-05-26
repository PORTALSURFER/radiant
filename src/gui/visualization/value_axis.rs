use crate::gui::types::Rect;

/// Named fields for constructing a reusable horizontal logarithmic value axis projector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalLogValueAxisParts {
    /// Rect used as the horizontal projection area.
    pub rect: Rect,
    /// Positive value projected at the left edge of the rect.
    pub min: f32,
    /// Positive value projected at the right edge of the rect.
    pub max: f32,
}

impl HorizontalLogValueAxisParts {
    /// Build axis parts for a visible positive logarithmic value span.
    pub const fn new(rect: Rect, min: f32, max: f32) -> Self {
        Self { rect, min, max }
    }
}

/// Reusable horizontal projector for positive logarithmic editor values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalLogValueAxis {
    /// Rect used as the horizontal projection area.
    pub rect: Rect,
    /// Positive value projected at the left edge of the rect.
    pub min: f32,
    /// Positive value projected at the right edge of the rect.
    pub max: f32,
}

impl HorizontalLogValueAxis {
    /// Build a horizontal logarithmic value axis from named parts.
    pub const fn from_parts(parts: HorizontalLogValueAxisParts) -> Self {
        Self {
            rect: parts.rect,
            min: parts.min,
            max: parts.max,
        }
    }

    /// Build a horizontal logarithmic value axis for a visible positive value span.
    pub const fn new(rect: Rect, min: f32, max: f32) -> Self {
        Self::from_parts(HorizontalLogValueAxisParts::new(rect, min, max))
    }

    /// Project a positive value into x coordinates, clamped to the visible span.
    pub fn x_for_value(self, value: f32) -> f32 {
        self.x_for_value_unclamped(self.clamp_value(value))
    }

    /// Project a positive value into x coordinates without clamping the value.
    pub fn x_for_value_unclamped(self, value: f32) -> f32 {
        self.rect
            .x_for_ratio_unclamped(self.ratio_for_value_unclamped(value))
    }

    /// Convert an x coordinate into a clamped positive value.
    pub fn value_for_x(self, x: f32) -> f32 {
        self.clamp_value(self.value_for_x_unclamped(x))
    }

    /// Convert an x coordinate into a positive value without clamping the x coordinate.
    pub fn value_for_x_unclamped(self, x: f32) -> f32 {
        let width = self.rect.width();
        if !x.is_finite() || !width.is_finite() || width <= f32::EPSILON {
            return self.min_positive();
        }
        self.value_for_ratio_unclamped((x - self.rect.min.x) / width)
    }

    /// Convert a normalized horizontal ratio into a clamped positive logarithmic value.
    pub fn value_for_ratio(self, ratio: f32) -> f32 {
        self.value_for_ratio_unclamped(clamped_ratio(ratio))
    }

    /// Convert a normalized horizontal ratio into a positive logarithmic value without clamping.
    pub fn value_for_ratio_unclamped(self, ratio: f32) -> f32 {
        10.0_f32.powf(self.log_min() + self.log_span() * finite_or_zero(ratio))
    }

    /// Convert a positive value into a clamped normalized horizontal ratio.
    pub fn ratio_for_value(self, value: f32) -> f32 {
        clamped_ratio(self.ratio_for_value_unclamped(value))
    }

    /// Return the visible logarithmic span, using a safe minimum for degenerate spans.
    pub fn log_span(self) -> f32 {
        let span = self.log_max() - self.log_min();
        if span.is_finite() && span.abs() > f32::EPSILON {
            span
        } else {
            1.0
        }
    }

    fn ratio_for_value_unclamped(self, value: f32) -> f32 {
        let value = positive_or_one(value);
        (value.log10() - self.log_min()) / self.log_span()
    }

    fn clamp_value(self, value: f32) -> f32 {
        positive_or_one(value).clamp(
            self.min_positive().min(self.max_positive()),
            self.min_positive().max(self.max_positive()),
        )
    }

    fn log_min(self) -> f32 {
        self.min_positive().log10()
    }

    fn log_max(self) -> f32 {
        self.max_positive().log10()
    }

    fn min_positive(self) -> f32 {
        positive_or_one(self.min)
    }

    fn max_positive(self) -> f32 {
        positive_or_one(self.max)
    }
}

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

fn clamped_ratio(value: f32) -> f32 {
    finite_or_zero(value).clamp(0.0, 1.0)
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn positive_or_one(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
