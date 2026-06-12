use crate::{
    gui::types::{Point, Rgba8},
    runtime::{PaintFillPolygon, PaintPrimitive, PaintStrokePolyline},
    widgets::WidgetId,
};
use std::sync::Arc;

/// Push a filled polygon from generated or caller-owned points.
pub fn push_fill_polygon(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    points: impl IntoIterator<Item = Point>,
    color: Rgba8,
) {
    if let Some(points) = collect_points_for_primitive(points, 3) {
        primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
            widget_id,
            points: Arc::from(points),
            color,
        }));
    }
}

/// Push an open stroked polyline from generated or caller-owned points.
pub fn push_stroke_polyline(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    points: impl IntoIterator<Item = Point>,
    color: Rgba8,
    width: f32,
) {
    if let Some(points) = collect_points_for_primitive(points, 2) {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id,
            points: Arc::from(points),
            color,
            width,
        }));
    }
}

pub(super) fn collect_points_for_primitive(
    points: impl IntoIterator<Item = Point>,
    minimum_points: usize,
) -> Option<Vec<Point>> {
    let iter = points.into_iter();
    let (lower_bound, upper_bound) = iter.size_hint();
    if upper_bound.is_some_and(|upper_bound| upper_bound < minimum_points) {
        return None;
    }
    let mut points = Vec::with_capacity(lower_bound.max(minimum_points));
    points.extend(iter);
    (points.len() >= minimum_points).then_some(points)
}
