use crate::{
    runtime::{PaintPrimitive, push_visible_fill_rect},
    widgets::WidgetId,
};

use super::CanvasSelectionGeometry;
use crate::gui::visualization::canvas::selection_affordance::{
    CanvasSelectionBodyHandlePaintParts, CanvasSelectionEdgeVisualPaintParts,
    CanvasSelectionTrailingControlPaintParts,
};

pub(super) fn push_body_handle_fill(
    geometry: CanvasSelectionGeometry,
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    parts: CanvasSelectionBodyHandlePaintParts,
) -> bool {
    let Some(rect) = geometry.body_handle_rect(
        parts.height,
        parts.end_inset,
        parts.max_end_inset_fraction,
        parts.min_width_after_inset,
    ) else {
        return false;
    };
    push_visible_fill_rect(primitives, widget_id, rect, parts.color)
}

pub(super) fn push_trailing_control_fill(
    geometry: CanvasSelectionGeometry,
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    parts: CanvasSelectionTrailingControlPaintParts,
) -> bool {
    let Some(rect) = geometry.trailing_control_rect(parts.side, parts.inset) else {
        return false;
    };
    push_visible_fill_rect(primitives, widget_id, rect, parts.color)
}

pub(super) fn push_edge_visual_fill(
    geometry: CanvasSelectionGeometry,
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    parts: CanvasSelectionEdgeVisualPaintParts,
) -> bool {
    let Some(rect) =
        geometry.edge_visual_rect(parts.bounds, parts.role, parts.width, parts.vertical_inset)
    else {
        return false;
    };
    push_visible_fill_rect(primitives, widget_id, rect, parts.color)
}
