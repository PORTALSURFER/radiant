use crate::gui_runtime::native_vello::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_path_fill(
    scene: &mut Scene,
    color: Rgba8,
    transform: PaintTransform,
    fill_rule: PaintFillRule,
    path: &PaintPath,
) {
    if !transform.is_finite() {
        return;
    }
    let Some(path) = to_kurbo_path(path) else {
        return;
    };

    scene.fill(
        match fill_rule {
            PaintFillRule::NonZero => Fill::NonZero,
            PaintFillRule::EvenOdd => Fill::EvenOdd,
        },
        Affine::new(transform.coefficients()),
        color_from_rgba(color),
        None,
        &path,
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_rect(
    scene: &mut Scene,
    color: Rgba8,
    rect: UiRect,
) {
    if !rect.has_finite_positive_area() {
        return;
    }
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        &to_kurbo_rect(rect),
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_rect_stroke(
    scene: &mut Scene,
    color: Rgba8,
    width: f32,
    rect: UiRect,
) {
    if !rect.has_finite_positive_area() || !paintable_stroke_width(width) {
        return;
    }
    scene.stroke(
        &vello::kurbo::Stroke::new(width as f64),
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        &to_kurbo_rect(rect),
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_polygon_fill(
    scene: &mut Scene,
    color: Rgba8,
    points: &[Point],
) {
    if let Some(path) = polygon_path(points) {
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_polygon_stroke(
    scene: &mut Scene,
    color: Rgba8,
    width: f32,
    points: &[Point],
) {
    if !paintable_stroke_width(width) {
        return;
    }
    if let Some(path) = polygon_path(points) {
        scene.stroke(
            &vello::kurbo::Stroke::new(width as f64),
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_polyline_stroke(
    scene: &mut Scene,
    color: Rgba8,
    width: f32,
    points: &[Point],
) {
    if !paintable_stroke_width(width) {
        return;
    }
    if let Some(path) = polyline_path(points) {
        scene.stroke(
            &vello::kurbo::Stroke::new(width as f64),
            Affine::IDENTITY,
            color_from_rgba(color),
            None,
            &path,
        );
    }
}

fn polygon_path(points: &[Point]) -> Option<BezPath> {
    if points.len() < 3 || !points.iter().all(|point| point.is_finite()) {
        return None;
    }
    let first = points.first()?;
    let mut path = BezPath::new();
    path.move_to(KurboPoint::new(first.x as f64, first.y as f64));
    for point in &points[1..] {
        path.line_to(KurboPoint::new(point.x as f64, point.y as f64));
    }
    path.close_path();
    Some(path)
}

fn polyline_path(points: &[Point]) -> Option<BezPath> {
    if points.len() < 2 || !points.iter().all(|point| point.is_finite()) {
        return None;
    }
    let first = points.first()?;
    let mut path = BezPath::new();
    path.move_to(KurboPoint::new(first.x as f64, first.y as f64));
    for point in &points[1..] {
        path.line_to(KurboPoint::new(point.x as f64, point.y as f64));
    }
    Some(path)
}

fn to_kurbo_path(path: &PaintPath) -> Option<BezPath> {
    let mut bezier = BezPath::new();
    for command in path.commands() {
        match *command {
            PaintPathCommand::MoveTo(point) => {
                if !point.is_finite() {
                    return None;
                }
                bezier.move_to(KurboPoint::new(point.x as f64, point.y as f64));
            }
            PaintPathCommand::LineTo(point) => {
                if !point.is_finite() {
                    return None;
                }
                bezier.line_to(KurboPoint::new(point.x as f64, point.y as f64));
            }
            PaintPathCommand::QuadTo { control, to } => {
                if !control.is_finite() || !to.is_finite() {
                    return None;
                }
                bezier.quad_to(
                    KurboPoint::new(control.x as f64, control.y as f64),
                    KurboPoint::new(to.x as f64, to.y as f64),
                );
            }
            PaintPathCommand::CurveTo {
                control1,
                control2,
                to,
            } => {
                if !control1.is_finite() || !control2.is_finite() || !to.is_finite() {
                    return None;
                }
                bezier.curve_to(
                    KurboPoint::new(control1.x as f64, control1.y as f64),
                    KurboPoint::new(control2.x as f64, control2.y as f64),
                    KurboPoint::new(to.x as f64, to.y as f64),
                );
            }
            PaintPathCommand::Close => {
                bezier.close_path();
            }
        }
    }
    (!bezier.is_empty()).then_some(bezier)
}

fn paintable_stroke_width(width: f32) -> bool {
    width.is_finite() && width > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_and_polyline_paths_reject_invalid_points() {
        let triangle = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        ];
        let line = [Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        let invalid = [Point::new(0.0, 0.0), Point::new(f32::NAN, 1.0)];

        assert!(polygon_path(&triangle).is_some());
        assert!(polygon_path(&line).is_none());
        assert!(polyline_path(&line).is_some());
        assert!(polyline_path(&invalid).is_none());
    }

    #[test]
    fn kurbo_path_conversion_rejects_empty_or_nonfinite_commands() {
        let valid = PaintPath::from([
            PaintPathCommand::MoveTo(Point::new(0.0, 0.0)),
            PaintPathCommand::LineTo(Point::new(1.0, 1.0)),
        ]);
        let invalid = PaintPath::from([
            PaintPathCommand::MoveTo(Point::new(0.0, 0.0)),
            PaintPathCommand::QuadTo {
                control: Point::new(f32::INFINITY, 0.0),
                to: Point::new(1.0, 1.0),
            },
        ]);

        assert!(to_kurbo_path(&valid).is_some());
        assert!(to_kurbo_path(&PaintPath::empty()).is_none());
        assert!(to_kurbo_path(&invalid).is_none());
    }

    #[test]
    fn paintable_stroke_width_rejects_empty_or_nonfinite_widths() {
        assert!(paintable_stroke_width(1.0));
        assert!(!paintable_stroke_width(0.0));
        assert!(!paintable_stroke_width(f32::NAN));
    }
}
