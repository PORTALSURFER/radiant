use crate::gui::types::{Point, Rect};

use super::{
    drag_handle::{DragHandle, DragHandleRole},
    numeric::finite_non_negative,
};
fn horizontal_resize_edge_width(rect: Rect, requested_width: f32) -> Option<f32> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let width = finite_non_negative(requested_width).min(rect.width() * 0.5);
    (width > 0.0).then_some(width)
}

/// Return hit-test handles for the horizontal resize edges of a rectangle.
pub fn horizontal_resize_edge_handles(
    rect: Rect,
    edge_width: f32,
    capture_token: u64,
) -> Option<[DragHandle; 2]> {
    let width = horizontal_resize_edge_width(rect, edge_width)?;
    Some([
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_max(rect.min, Point::new(rect.min.x + width, rect.max.y)),
            capture_token,
        ),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_max(Point::new(rect.max.x - width, rect.min.y), rect.max),
            capture_token,
        ),
    ])
}

/// Return body, start-edge, and end-edge handles for a horizontally resizable rectangle.
///
/// Handles are returned in paint-order priority: body first, then start, then
/// end. Passing the result to [`super::drag_handle::drag_handle_at_point`] gives edges priority
/// over the body when hit targets overlap.
pub fn horizontal_resize_handles(
    rect: Rect,
    edge_width: f32,
    capture_token: u64,
) -> Option<[DragHandle; 3]> {
    let [start, end] = horizontal_resize_edge_handles(rect, edge_width, capture_token)?;
    Some([
        DragHandle::new(DragHandleRole::Body, rect, capture_token),
        start,
        end,
    ])
}

/// Return the visible affordance rectangle for one horizontal resize edge.
pub fn horizontal_resize_edge_visual_rect(
    rect: Rect,
    role: DragHandleRole,
    width: f32,
    edge_inset: f32,
    vertical_inset: f32,
) -> Option<Rect> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let width = finite_non_negative(width);
    let edge_inset = finite_non_negative(edge_inset);
    let vertical_inset = finite_non_negative(vertical_inset).min(rect.height() * 0.5);
    let visual_height = rect.height() - vertical_inset * 2.0;
    if width <= 0.0 || visual_height <= 0.0 || width + edge_inset > rect.width() {
        return None;
    }
    let (min_x, max_x) = match role {
        DragHandleRole::Start => (rect.min.x + edge_inset, rect.min.x + edge_inset + width),
        DragHandleRole::End => (rect.max.x - edge_inset - width, rect.max.x - edge_inset),
        _ => return None,
    };
    Some(Rect::from_min_max(
        Point::new(min_x, rect.min.y + vertical_inset),
        Point::new(max_x, rect.max.y - vertical_inset),
    ))
}

/// Return a three-rect bracket affordance for one horizontal resize edge.
///
/// The rectangles are returned as the vertical edge stem, top tick, and bottom
/// tick. This shape is useful for editor-style timeline and canvas items where
/// the resize affordance should read as a bracket instead of a plain edge bar.
pub fn horizontal_resize_edge_bracket_rects(
    rect: Rect,
    role: DragHandleRole,
    stroke: f32,
    tick_length: f32,
) -> Option<[Rect; 3]> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let stroke = finite_non_negative(stroke)
        .min(rect.width())
        .min(rect.height());
    let tick_length = finite_non_negative(tick_length).min(rect.width());
    if stroke <= 0.0 || tick_length <= 0.0 {
        return None;
    }

    let (stem_min_x, tick_min_x, tick_max_x) = match role {
        DragHandleRole::Start => {
            let stem_min_x = rect.min.x;
            (stem_min_x, stem_min_x, stem_min_x + tick_length)
        }
        DragHandleRole::End => {
            let stem_min_x = rect.max.x - stroke;
            (
                stem_min_x,
                stem_min_x + stroke - tick_length,
                stem_min_x + stroke,
            )
        }
        _ => return None,
    };
    Some([
        Rect::from_min_max(
            Point::new(stem_min_x, rect.min.y),
            Point::new(stem_min_x + stroke, rect.max.y),
        ),
        Rect::from_min_max(
            Point::new(tick_min_x, rect.min.y),
            Point::new(tick_max_x, rect.min.y + stroke),
        ),
        Rect::from_min_max(
            Point::new(tick_min_x, rect.max.y - stroke),
            Point::new(tick_max_x, rect.max.y),
        ),
    ])
}
