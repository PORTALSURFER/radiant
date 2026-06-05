use crate::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintPrimitive, PaintStrokePolyline},
    widgets::WidgetId,
};
use std::sync::Arc;

/// Named fields for appending a sampled visual curve to a paint plan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampledCurveStrokeParts {
    /// Widget that owns the generated stroke primitive.
    pub widget_id: WidgetId,
    /// Bounds used to clamp sampled curve points before painting.
    pub bounds: Rect,
    /// Number of intervals to sample. The generated curve has `steps + 1`
    /// points when every callback result is valid.
    pub steps: usize,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub stroke_width: f32,
}

impl SampledCurveStrokeParts {
    /// Build sampled curve stroke parts.
    pub const fn new(
        widget_id: WidgetId,
        bounds: Rect,
        steps: usize,
        color: Rgba8,
        stroke_width: f32,
    ) -> Self {
        Self {
            widget_id,
            bounds,
            steps,
            color,
            stroke_width,
        }
    }
}

/// Sample a visual curve into clamped paint points.
///
/// The callback receives a normalized sample position in `0.0..=1.0` and may
/// return `None` to skip invalid or hidden points. Returned points are clamped
/// to `bounds`, and non-finite points are ignored. This keeps waveform,
/// timeline, automation, EQ, and other editor-style widgets from repeating
/// point-buffer and finite-geometry guard code while leaving domain curve math
/// in the host.
pub fn sampled_curve_points(
    bounds: Rect,
    steps: usize,
    mut point_at: impl FnMut(f32) -> Option<Point>,
) -> Vec<Point> {
    if !bounds.has_finite_positive_area() {
        return Vec::new();
    }
    let steps = steps.max(1);
    let mut points = Vec::with_capacity(steps + 1);
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let Some(point) = point_at(t) else {
            continue;
        };
        if !point.x.is_finite() || !point.y.is_finite() {
            continue;
        }
        points.push(Point::new(
            point.x.clamp(bounds.min.x, bounds.max.x),
            point.y.clamp(bounds.min.y, bounds.max.y),
        ));
    }
    points
}

/// Append a sampled visual curve stroke to a paint primitive buffer.
///
/// Returns `true` when at least two visible points were sampled and a stroke was
/// appended. Degenerate bounds, invalid sample points, and non-positive stroke
/// widths append no primitive.
pub fn push_sampled_curve_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    parts: SampledCurveStrokeParts,
    point_at: impl FnMut(f32) -> Option<Point>,
) -> bool {
    if !parts.stroke_width.is_finite() || parts.stroke_width <= 0.0 {
        return false;
    }
    let points = sampled_curve_points(parts.bounds, parts.steps, point_at);
    if points.len() < 2 {
        return false;
    }
    primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
        widget_id: parts.widget_id,
        points: Arc::from(points),
        color: parts.color,
        width: parts.stroke_width,
    }));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Vector2;

    fn bounds() -> Rect {
        Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 40.0))
    }

    #[test]
    fn sampled_curve_points_clamps_to_bounds_and_skips_invalid_points() {
        let points = sampled_curve_points(bounds(), 3, |t| {
            if (t - (1.0 / 3.0)).abs() < 0.001 {
                return None;
            }
            Some(Point::new(10.0 + t * 120.0, 10.0 + t * 80.0))
        });

        assert_eq!(
            points,
            vec![
                Point::new(10.0, 20.0),
                Point::new(90.0, 60.0),
                Point::new(110.0, 60.0),
            ]
        );
    }

    #[test]
    fn push_sampled_curve_stroke_appends_polyline_for_visible_points() {
        let mut primitives = Vec::new();

        assert!(push_sampled_curve_stroke(
            &mut primitives,
            SampledCurveStrokeParts::new(42, bounds(), 2, Rgba8::new(1, 2, 3, 4), 2.0),
            |t| Some(Point::new(10.0 + t * 100.0, 60.0 - t * 40.0)),
        ));

        assert_eq!(primitives.len(), 1);
        let stroke = primitives[0].stroke_polyline().expect("stroke polyline");
        assert_eq!(stroke.widget_id, 42);
        assert_eq!(stroke.points.len(), 3);
        assert_eq!(
            stroke.points.as_ref(),
            [
                Point::new(10.0, 60.0),
                Point::new(60.0, 40.0),
                Point::new(110.0, 20.0)
            ]
        );
    }

    #[test]
    fn push_sampled_curve_stroke_skips_degenerate_curves() {
        let mut primitives = Vec::new();

        assert!(!push_sampled_curve_stroke(
            &mut primitives,
            SampledCurveStrokeParts::new(42, bounds(), 2, Rgba8::new(1, 2, 3, 4), 2.0),
            |_| None,
        ));

        assert!(primitives.is_empty());
    }
}
