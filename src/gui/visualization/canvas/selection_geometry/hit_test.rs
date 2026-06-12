use crate::gui::types::{Point, Rect};

use super::CanvasSelectionGeometry;
use crate::gui::visualization::canvas::{
    drag_handle::DragHandleRole,
    selection_affordance::{
        CanvasSelectionAffordanceHitTestParts, CanvasSelectionBodyHandleHitTestParts,
        CanvasSelectionTrailingControlHitTestParts,
    },
};

pub(super) fn body_handle_at_point(
    geometry: CanvasSelectionGeometry,
    height: f32,
    end_inset: f32,
    max_end_inset_fraction: f32,
    min_width_after_inset: f32,
    point: Point,
) -> bool {
    geometry
        .body_handle_rect(
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
        )
        .is_some_and(|rect| rect.contains(point))
}

pub(super) fn body_affordance_at_point(
    geometry: CanvasSelectionGeometry,
    parts: CanvasSelectionBodyHandleHitTestParts,
) -> bool {
    body_handle_at_point(
        geometry,
        parts.height,
        parts.end_inset,
        parts.max_end_inset_fraction,
        parts.min_width_after_inset,
        parts.point,
    )
}

pub(super) fn trailing_control_at_point(
    geometry: CanvasSelectionGeometry,
    side: f32,
    inset: f32,
    point: Point,
) -> bool {
    geometry
        .trailing_control_rect(side, inset)
        .is_some_and(|rect| rect.contains(point))
}

pub(super) fn trailing_control_affordance_at_point(
    geometry: CanvasSelectionGeometry,
    parts: CanvasSelectionTrailingControlHitTestParts,
) -> bool {
    trailing_control_at_point(geometry, parts.side, parts.inset, parts.point)
}

pub(super) fn edge_at_point(
    geometry: CanvasSelectionGeometry,
    bounds: Rect,
    point: Point,
    width: f32,
    vertical_inset: f32,
) -> Option<DragHandleRole> {
    [DragHandleRole::Start, DragHandleRole::End]
        .into_iter()
        .find(|role| {
            geometry
                .edge_visual_rect(bounds, *role, width, vertical_inset)
                .is_some_and(|rect| rect.contains(point))
        })
}

pub(super) fn affordance_at_point(
    geometry: CanvasSelectionGeometry,
    parts: CanvasSelectionAffordanceHitTestParts,
) -> Option<DragHandleRole> {
    if parts
        .trailing_control
        .is_some_and(|trailing| trailing_control_affordance_at_point(geometry, trailing))
    {
        return Some(DragHandleRole::TrailingControl);
    }
    if let Some(edge) = parts.edge
        && let Some(role) = edge_at_point(
            geometry,
            edge.bounds,
            edge.point,
            edge.width,
            edge.vertical_inset,
        )
    {
        return Some(role);
    }
    if parts
        .body
        .is_some_and(|body| body_affordance_at_point(geometry, body))
    {
        return Some(DragHandleRole::Body);
    }
    None
}
