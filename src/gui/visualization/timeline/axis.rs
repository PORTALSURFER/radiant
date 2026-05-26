use crate::gui::types::{Point, Rect};

/// Named fields for constructing a reusable timeline axis projector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineAxisParts {
    /// Rect used as the horizontal projection area.
    pub rect: Rect,
    /// First visible timeline value.
    pub start: f32,
    /// Last visible timeline value.
    pub end: f32,
    /// Logical pixels reserved at the trailing edge of the projection area.
    pub trailing_padding: f32,
}

impl TimelineAxisParts {
    /// Build axis parts for a visible timeline span.
    pub const fn new(rect: Rect, start: f32, end: f32) -> Self {
        Self {
            rect,
            start,
            end,
            trailing_padding: 0.0,
        }
    }
}

/// Reusable horizontal projector for beat, frame, time, or sample timelines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineAxis {
    /// Rect used as the horizontal projection area.
    pub rect: Rect,
    /// First visible timeline value.
    pub start: f32,
    /// Last visible timeline value.
    pub end: f32,
    /// Logical pixels reserved at the trailing edge of the projection area.
    pub trailing_padding: f32,
}

impl TimelineAxis {
    /// Build a timeline axis from named parts.
    pub const fn from_parts(parts: TimelineAxisParts) -> Self {
        Self {
            rect: parts.rect,
            start: parts.start,
            end: parts.end,
            trailing_padding: parts.trailing_padding,
        }
    }

    /// Build a timeline axis for a visible value span.
    pub const fn new(rect: Rect, start: f32, end: f32) -> Self {
        Self::from_parts(TimelineAxisParts::new(rect, start, end))
    }

    /// Return this axis with trailing projection padding.
    pub const fn with_trailing_padding(mut self, padding: f32) -> Self {
        self.trailing_padding = padding;
        self
    }

    /// Return the usable projection rect after trailing padding is applied.
    pub fn projection_rect(self) -> Rect {
        let mut rect = self.rect;
        if self.trailing_padding.is_finite() && self.trailing_padding > 0.0 {
            rect.max.x = (rect.max.x - self.trailing_padding).max(rect.min.x);
        }
        rect
    }

    /// Project a timeline value into x coordinates, clamped to the visible span.
    pub fn x_for_value(self, value: f32) -> f32 {
        self.x_for_value_unclamped(self.clamp_value(value))
    }

    /// Project a timeline value into x coordinates without clamping the value.
    pub fn x_for_value_unclamped(self, value: f32) -> f32 {
        let rect = self.projection_rect();
        rect.min.x + rect.width().max(1.0) * self.ratio_for_value_unclamped(value)
    }

    /// Project a timeline value range into a full-height rect, clamped to the visible span.
    pub fn range_rect(self, start: f32, end: f32) -> Rect {
        self.range_rect_from_x(self.x_for_value(start), self.x_for_value(end))
    }

    /// Project a timeline value range into a full-height rect without clamping values.
    pub fn range_rect_unclamped(self, start: f32, end: f32) -> Rect {
        self.range_rect_from_x(
            self.x_for_value_unclamped(start),
            self.x_for_value_unclamped(end),
        )
    }

    /// Convert an x coordinate into a clamped timeline value.
    pub fn value_for_x(self, x: f32) -> f32 {
        self.clamp_value(self.value_for_x_unclamped(x))
    }

    /// Convert an x coordinate into a timeline value without clamping the x coordinate.
    pub fn value_for_x_unclamped(self, x: f32) -> f32 {
        let rect = self.projection_rect();
        let width = rect.width();
        if !x.is_finite() || !width.is_finite() || width <= f32::EPSILON {
            return self.start_or_zero();
        }
        self.start_or_zero() + ((x - rect.min.x) / width) * self.value_span()
    }

    /// Clamp an x coordinate to the projection rect.
    pub fn clamp_x(self, x: f32) -> f32 {
        let rect = self.projection_rect();
        if !x.is_finite() {
            return rect.min.x;
        }
        x.clamp(rect.min.x, rect.max.x)
    }

    /// Return the clamped x coordinate for a point inside the projection span.
    pub fn cursor_x_at(self, position: Point) -> Option<f32> {
        let rect = self.projection_rect();
        if !position.x.is_finite() || position.x < rect.min.x || position.x > rect.max.x {
            return None;
        }
        Some(self.clamp_x(position.x))
    }

    /// Return the clamped timeline value for a point inside the projection span.
    pub fn value_at_point(self, position: Point) -> Option<f32> {
        self.cursor_x_at(position)?;
        Some(self.value_for_x(position.x))
    }

    /// Return the visible value span, using a safe minimum for degenerate spans.
    pub fn value_span(self) -> f32 {
        let span = self.end - self.start;
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
        (value - self.start_or_zero()) / self.value_span()
    }

    fn clamp_value(self, value: f32) -> f32 {
        if !value.is_finite() {
            return self.start_or_zero();
        }
        value.clamp(self.start.min(self.end), self.start.max(self.end))
    }

    fn start_or_zero(self) -> f32 {
        if self.start.is_finite() {
            self.start
        } else {
            0.0
        }
    }

    fn range_rect_from_x(self, x0: f32, x1: f32) -> Rect {
        let rect = self.projection_rect();
        Rect::from_min_max(
            Point::new(x0.min(x1), rect.min.y),
            Point::new(x0.max(x1), rect.max.y),
        )
    }
}
