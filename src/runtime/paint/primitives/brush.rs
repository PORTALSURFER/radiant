use crate::gui::types::{Point, Rect, Rgba8};

/// Backend-neutral brush used to fill vector paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaintBrush {
    /// Fill the complete path with one color.
    Solid(Rgba8),
    /// Fill the path with a two-stop linear gradient.
    LinearGradient(PaintLinearGradient),
}

impl PaintBrush {
    /// Build a solid-color path brush.
    pub const fn solid(color: Rgba8) -> Self {
        Self::Solid(color)
    }

    /// Build a linear-gradient path brush.
    pub const fn linear_gradient(gradient: PaintLinearGradient) -> Self {
        Self::LinearGradient(gradient)
    }
}

/// Two-stop linear gradient in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintLinearGradient {
    /// Gradient start point.
    pub start: Point,
    /// Gradient end point.
    pub end: Point,
    /// Color at the gradient start.
    pub start_color: Rgba8,
    /// Color at the gradient end.
    pub end_color: Rgba8,
}

impl PaintLinearGradient {
    /// Build a two-stop linear gradient.
    pub const fn new(start: Point, end: Point, start_color: Rgba8, end_color: Rgba8) -> Self {
        Self {
            start,
            end,
            start_color,
            end_color,
        }
    }

    /// Build a top-to-bottom gradient spanning `bounds`.
    pub fn vertical(bounds: Rect, top_color: Rgba8, bottom_color: Rgba8) -> Self {
        let x = bounds.min.x + bounds.width() * 0.5;
        Self::new(
            Point::new(x, bounds.min.y),
            Point::new(x, bounds.max.y),
            top_color,
            bottom_color,
        )
    }

    /// Return whether the gradient has finite, distinct endpoints.
    pub fn is_paintable(self) -> bool {
        self.start.is_finite()
            && self.end.is_finite()
            && (self.start.x != self.end.x || self.start.y != self.end.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Vector2;

    #[test]
    fn vertical_gradient_spans_bounds_and_preserves_alpha() {
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 40.0));
        let top = Rgba8::new(1, 2, 3, 90);
        let bottom = Rgba8::new(4, 5, 6, 0);
        let gradient = PaintLinearGradient::vertical(bounds, top, bottom);

        assert_eq!(gradient.start, Point::new(60.0, 20.0));
        assert_eq!(gradient.end, Point::new(60.0, 60.0));
        assert_eq!(gradient.start_color, top);
        assert_eq!(gradient.end_color, bottom);
        assert!(gradient.is_paintable());
    }

    #[test]
    fn gradient_rejects_degenerate_or_nonfinite_geometry() {
        let color = Rgba8::new(1, 2, 3, 4);
        assert!(
            !PaintLinearGradient::new(Point::new(1.0, 1.0), Point::new(1.0, 1.0), color, color,)
                .is_paintable()
        );
        assert!(
            !PaintLinearGradient::new(
                Point::new(f32::NAN, 1.0),
                Point::new(2.0, 1.0),
                color,
                color,
            )
            .is_paintable()
        );
    }
}
