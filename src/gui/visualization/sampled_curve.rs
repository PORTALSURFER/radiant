use crate::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{
        PaintBrush, PaintFillPath, PaintPath, PaintPathCommand, PaintPrimitive, PaintStrokePolyline,
    },
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

/// Baseline used to close a sampled curve into a filled area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SampledCurveAreaBaseline {
    /// Close the area against the top edge of the curve bounds.
    Top,
    /// Close the area against the bottom edge of the curve bounds.
    Bottom,
    /// Close the area against a logical Y coordinate, clamped to the bounds.
    Y(f32),
}

impl SampledCurveAreaBaseline {
    fn y(self, bounds: Rect) -> Option<f32> {
        let y = match self {
            Self::Top => bounds.min.y,
            Self::Bottom => bounds.max.y,
            Self::Y(y) => y,
        };
        y.is_finite().then(|| y.clamp(bounds.min.y, bounds.max.y))
    }
}

/// Named fields for appending a sampled curve-area fill to a paint plan.
#[derive(Clone, Debug, PartialEq)]
pub struct SampledCurveAreaFillParts {
    /// Widget that owns the generated fill primitive.
    pub widget_id: WidgetId,
    /// Bounds used to clamp sampled curve points and the baseline.
    pub bounds: Rect,
    /// Number of intervals to sample.
    pub steps: usize,
    /// Baseline used to close the sampled curve area.
    pub baseline: SampledCurveAreaBaseline,
    /// Solid or gradient brush used to fill the area.
    pub brush: PaintBrush,
}

impl SampledCurveAreaFillParts {
    /// Build sampled curve-area fill parts.
    pub const fn new(
        widget_id: WidgetId,
        bounds: Rect,
        steps: usize,
        baseline: SampledCurveAreaBaseline,
        brush: PaintBrush,
    ) -> Self {
        Self {
            widget_id,
            bounds,
            steps,
            baseline,
            brush,
        }
    }
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

/// Sample a visual curve and close each contiguous segment against a baseline.
///
/// The result contains one compact backend-neutral path with one subpath per
/// contiguous visible curve segment. Invalid or hidden samples split segments
/// instead of bridging across missing data. Each emitted subpath contains at
/// least two curve points.
pub fn sampled_curve_area_path(
    bounds: Rect,
    steps: usize,
    baseline: SampledCurveAreaBaseline,
    mut point_at: impl FnMut(f32) -> Option<Point>,
) -> PaintPath {
    if !bounds.has_finite_positive_area() {
        return PaintPath::empty();
    }
    let Some(baseline_y) = baseline.y(bounds) else {
        return PaintPath::empty();
    };
    let steps = steps.max(1);
    let mut commands = Vec::with_capacity(steps + 5);
    let mut segment_start = None;
    let mut segment_points = 0usize;
    let mut last_x = 0.0;

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let point = point_at(t).filter(|point| point.is_finite()).map(|point| {
            Point::new(
                point.x.clamp(bounds.min.x, bounds.max.x),
                point.y.clamp(bounds.min.y, bounds.max.y),
            )
        });
        let Some(point) = point else {
            finish_area_segment(
                &mut commands,
                &mut segment_start,
                &mut segment_points,
                last_x,
                baseline_y,
            );
            continue;
        };
        if segment_start.is_none() {
            segment_start = Some(commands.len());
            commands.push(PaintPathCommand::MoveTo(Point::new(point.x, baseline_y)));
        }
        commands.push(PaintPathCommand::LineTo(point));
        segment_points += 1;
        last_x = point.x;
    }
    finish_area_segment(
        &mut commands,
        &mut segment_start,
        &mut segment_points,
        last_x,
        baseline_y,
    );
    PaintPath::from(commands)
}

/// Append one sampled curve-area path fill to a paint primitive buffer.
///
/// Returns `true` when at least one contiguous segment with two curve points
/// was sampled. The helper emits one `FillPath` primitive regardless of sample
/// count, keeping renderer submission and brush evaluation constant-sized.
pub fn push_sampled_curve_area_fill(
    primitives: &mut Vec<PaintPrimitive>,
    parts: SampledCurveAreaFillParts,
    point_at: impl FnMut(f32) -> Option<Point>,
) -> bool {
    let path = sampled_curve_area_path(parts.bounds, parts.steps, parts.baseline, point_at);
    if path.is_empty() {
        return false;
    }
    primitives.push(PaintPrimitive::FillPath(PaintFillPath::new(
        parts.widget_id,
        path,
        parts.brush,
    )));
    true
}

fn finish_area_segment(
    commands: &mut Vec<PaintPathCommand>,
    segment_start: &mut Option<usize>,
    segment_points: &mut usize,
    last_x: f32,
    baseline_y: f32,
) {
    let Some(start) = segment_start.take() else {
        return;
    };
    if *segment_points >= 2 {
        commands.push(PaintPathCommand::LineTo(Point::new(last_x, baseline_y)));
        commands.push(PaintPathCommand::Close);
    } else {
        commands.truncate(start);
    }
    *segment_points = 0;
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

    #[test]
    fn sampled_curve_area_path_closes_against_bottom_baseline() {
        let path = sampled_curve_area_path(bounds(), 2, SampledCurveAreaBaseline::Bottom, |t| {
            Some(Point::new(10.0 + t * 100.0, 50.0 - t * 20.0))
        });

        assert_eq!(
            path.commands(),
            [
                PaintPathCommand::MoveTo(Point::new(10.0, 60.0)),
                PaintPathCommand::LineTo(Point::new(10.0, 50.0)),
                PaintPathCommand::LineTo(Point::new(60.0, 40.0)),
                PaintPathCommand::LineTo(Point::new(110.0, 30.0)),
                PaintPathCommand::LineTo(Point::new(110.0, 60.0)),
                PaintPathCommand::Close,
            ]
        );
    }

    #[test]
    fn sampled_curve_area_path_splits_missing_data_without_bridging() {
        let path = sampled_curve_area_path(bounds(), 5, SampledCurveAreaBaseline::Top, |t| {
            ((t - 0.4).abs() > 0.01).then(|| Point::new(10.0 + t * 100.0, 30.0 + t * 10.0))
        });

        assert_eq!(
            path.commands()
                .iter()
                .filter(|command| matches!(command, PaintPathCommand::Close))
                .count(),
            2
        );
    }

    #[test]
    fn push_sampled_curve_area_fill_emits_one_gradient_path_at_high_sample_count() {
        let mut primitives = Vec::new();
        let gradient = crate::runtime::PaintLinearGradient::vertical(
            bounds(),
            Rgba8::new(10, 20, 30, 100),
            Rgba8::new(10, 20, 30, 0),
        );

        assert!(push_sampled_curve_area_fill(
            &mut primitives,
            SampledCurveAreaFillParts::new(
                42,
                bounds(),
                4096,
                SampledCurveAreaBaseline::Bottom,
                PaintBrush::linear_gradient(gradient),
            ),
            |t| Some(Point::new(10.0 + t * 100.0, 40.0)),
        ));

        assert_eq!(primitives.len(), 1);
        let fill = primitives[0].fill_path().expect("curve area fill path");
        assert_eq!(fill.widget_id, 42);
        assert_eq!(fill.brush, PaintBrush::linear_gradient(gradient));
        assert_eq!(fill.path.commands().len(), 4100);
    }

    #[test]
    fn sampled_curve_area_fill_skips_invalid_baseline_or_single_point_segment() {
        let mut primitives = Vec::new();
        assert!(!push_sampled_curve_area_fill(
            &mut primitives,
            SampledCurveAreaFillParts::new(
                42,
                bounds(),
                2,
                SampledCurveAreaBaseline::Y(f32::NAN),
                PaintBrush::solid(Rgba8::new(1, 2, 3, 4)),
            ),
            |t| (t == 0.0).then(|| Point::new(10.0, 30.0)),
        ));
        assert!(primitives.is_empty());
    }
}
