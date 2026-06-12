use crate::gui::types::{Point, Rect};

use super::super::numeric::normalized_fraction;

/// Explicit parts for building reusable normalized canvas selection geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionGeometryParts {
    /// Canvas bounds containing the normalized selection.
    pub bounds: Rect,
    /// Normalized selection start.
    pub start_fraction: f32,
    /// Normalized selection end.
    pub end_fraction: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CanvasSelectionProjection {
    pub(super) bounds: Rect,
    pub(super) start_fraction: f32,
    pub(super) end_fraction: f32,
    pub(super) rect: Rect,
}

pub(super) fn project_canvas_selection(
    parts: CanvasSelectionGeometryParts,
) -> Option<CanvasSelectionProjection> {
    let start_fraction = normalized_fraction(parts.start_fraction);
    let end_fraction = normalized_fraction(parts.end_fraction);
    let rect = canvas_selection_rect(parts.bounds, start_fraction, end_fraction)?;
    Some(CanvasSelectionProjection {
        bounds: parts.bounds,
        start_fraction,
        end_fraction,
        rect,
    })
}

/// Return a normalized horizontal selection rectangle inside a canvas.
pub fn canvas_selection_rect(bounds: Rect, start_fraction: f32, end_fraction: f32) -> Option<Rect> {
    if !bounds.has_finite_positive_area() {
        return None;
    }
    let start = normalized_fraction(start_fraction);
    let end = normalized_fraction(end_fraction);
    if end <= start {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(bounds.x_for_ratio(start), bounds.min.y),
        Point::new(bounds.x_for_ratio(end), bounds.max.y),
    ))
}
