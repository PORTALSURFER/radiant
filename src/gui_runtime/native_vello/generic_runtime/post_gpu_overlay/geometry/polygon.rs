use crate::{
    gui::types::{Point, Rgba8, Vector2},
    runtime::{PaintBrush, PaintFillPath, PaintFillPolygon, PaintStrokePolygon},
};
use vello_svg::usvg::tiny_skia_path::{Path, PathBuilder, Stroke};

use super::{
    OverlayVertex,
    path::{
        paint_path_from_tiny_skia, push_fill_path_vertices,
        push_fill_path_vertices_in_regions_including_opaque,
    },
};

pub(super) fn push_fill_polygon_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    fill: &PaintFillPolygon,
) {
    let Some(path) = polygon_fill(fill.widget_id, &fill.points, fill.color) else {
        return;
    };
    push_fill_path_vertices(vertices, target_size, &path);
}

pub(super) fn push_fill_polygon_vertices_in_regions(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    fill: &PaintFillPolygon,
    regions: &[crate::gui::types::Rect],
) {
    let Some(path) = polygon_fill(fill.widget_id, &fill.points, fill.color) else {
        return;
    };
    push_fill_path_vertices_in_regions_including_opaque(vertices, target_size, &path, regions);
}

pub(super) fn push_stroke_polygon_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    stroke: &PaintStrokePolygon,
) {
    let Some(path) = polygon_stroke(stroke) else {
        return;
    };
    push_fill_path_vertices(vertices, target_size, &path);
}

pub(super) fn push_stroke_polygon_vertices_in_regions(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    stroke: &PaintStrokePolygon,
    regions: &[crate::gui::types::Rect],
) {
    let Some(path) = polygon_stroke(stroke) else {
        return;
    };
    push_fill_path_vertices_in_regions_including_opaque(vertices, target_size, &path, regions);
}

fn polygon_fill(widget_id: u64, points: &[Point], color: Rgba8) -> Option<PaintFillPath> {
    let path = polygon_path(points)?;
    let path = paint_path_from_tiny_skia(&path)?;
    Some(PaintFillPath::new(
        widget_id,
        path,
        PaintBrush::solid(color),
    ))
}

fn polygon_stroke(stroke: &PaintStrokePolygon) -> Option<PaintFillPath> {
    let path = polygon_path(&stroke.points)?;
    let width = if stroke.width.is_finite() && stroke.width > 0.0 {
        stroke.width
    } else {
        1.0
    };
    let outline = path.stroke(
        &Stroke {
            width,
            ..Stroke::default()
        },
        1.0,
    )?;
    let outline = paint_path_from_tiny_skia(&outline)?;
    Some(PaintFillPath::new(
        stroke.widget_id,
        outline,
        PaintBrush::solid(stroke.color),
    ))
}

fn polygon_path(points: &[Point]) -> Option<Path> {
    let first = *points.first()?;
    if points.len() < 3 || !points.iter().all(|point| point.is_finite()) {
        return None;
    }
    let mut builder = PathBuilder::with_capacity(points.len().saturating_add(1), points.len());
    builder.move_to(first.x, first.y);
    for point in points.iter().copied().skip(1) {
        builder.line_to(point.x, point.y);
    }
    builder.close();
    builder.finish()
}
