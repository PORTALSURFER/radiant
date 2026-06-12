use crate::gui::types::{Point, Rect, Vector2};

use super::{CanvasSelectionGeometry, projection::canvas_selection_rect};
use crate::gui::visualization::canvas::{
    drag_handle::{DragHandle, DragHandleRole},
    numeric::{finite_non_negative, normalized_fraction},
};

pub(super) fn edge_visual_rect_for_geometry(
    geometry: CanvasSelectionGeometry,
    bounds: Rect,
    role: DragHandleRole,
    width: f32,
    vertical_inset: f32,
) -> Option<Rect> {
    let fraction = match role {
        DragHandleRole::Start => geometry.start_fraction,
        DragHandleRole::End => geometry.end_fraction,
        _ => return None,
    };
    canvas_selection_edge_visual_rect(bounds, fraction, width, vertical_inset)
}

/// Return hit-test handles for the start and end edges of a normalized canvas selection.
pub fn canvas_selection_edge_handles(
    bounds: Rect,
    start_fraction: f32,
    end_fraction: f32,
    hit_width: f32,
    capture_token: u64,
) -> Option<[DragHandle; 2]> {
    let selection = canvas_selection_rect(bounds, start_fraction, end_fraction)?;
    let width = finite_non_negative(hit_width);
    if width <= 0.0 {
        return None;
    }
    Some([
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_size(
                Point::new(selection.min.x - width * 0.5, bounds.min.y),
                Vector2::new(width, bounds.height()),
            ),
            capture_token,
        ),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_size(
                Point::new(selection.max.x - width * 0.5, bounds.min.y),
                Vector2::new(width, bounds.height()),
            ),
            capture_token,
        ),
    ])
}

/// Return the visible edge handle for a normalized canvas selection.
pub fn canvas_selection_edge_visual_rect(
    bounds: Rect,
    edge_fraction: f32,
    width: f32,
    vertical_inset: f32,
) -> Option<Rect> {
    if !bounds.has_finite_positive_area() {
        return None;
    }
    let width = finite_non_negative(width);
    let inset = finite_non_negative(vertical_inset).min(bounds.height() * 0.5);
    if width <= 0.0 || bounds.height() - inset * 2.0 <= 0.0 {
        return None;
    }
    let center_x = bounds.x_for_ratio(normalized_fraction(edge_fraction));
    Some(Rect::from_min_size(
        Point::new(center_x - width * 0.5, bounds.min.y + inset),
        Vector2::new(width, bounds.height() - inset * 2.0),
    ))
}
