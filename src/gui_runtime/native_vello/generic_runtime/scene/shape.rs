use crate::gui_runtime::native_vello::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_path_fill(
    scene: &mut Scene,
    color: Rgba8,
    fill_rule: PaintFillRule,
    path: &BezPath,
) {
    scene.fill(
        match fill_rule {
            PaintFillRule::NonZero => Fill::NonZero,
            PaintFillRule::EvenOdd => Fill::EvenOdd,
        },
        Affine::IDENTITY,
        color_from_rgba(color),
        None,
        path,
    );
}

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_rect(
    scene: &mut Scene,
    color: Rgba8,
    rect: UiRect,
) {
    scene.fill(
        Fill::NonZero,
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
    let first = points.first()?;
    let mut path = BezPath::new();
    path.move_to(KurboPoint::new(first.x as f64, first.y as f64));
    for point in &points[1..] {
        path.line_to(KurboPoint::new(point.x as f64, point.y as f64));
    }
    Some(path)
}
