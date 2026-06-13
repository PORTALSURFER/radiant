mod affordances;
mod layers;
mod paint_style;
mod resize_handles;
mod selection_geometry;

mod fixtures {
    pub(super) use super::super::super::{
        CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts,
        CanvasSelectionAffordanceHitTestParts, CanvasSelectionAffordancePaintParts,
        CanvasSelectionAffordanceStyle, CanvasSelectionBodyHandleHitTestParts,
        CanvasSelectionBodyHandlePaintParts, CanvasSelectionBodyHandleParts,
        CanvasSelectionBodyHandleStyle, CanvasSelectionEdgeHitTestParts,
        CanvasSelectionEdgeVisualPaintParts, CanvasSelectionEdgeVisualStyle,
        CanvasSelectionGeometry, CanvasSelectionPaintStyle,
        CanvasSelectionTrailingControlHitTestParts, CanvasSelectionTrailingControlPaintParts,
        CanvasSelectionTrailingControlStyle, DragHandle, DragHandleRole, canvas_layer_at_point,
        canvas_selection_body_handle_rect, canvas_selection_edge_handles,
        canvas_selection_edge_visual_rect, canvas_selection_rect,
        canvas_selection_trailing_control_rect, drag_handle_at_point,
        horizontal_resize_edge_bracket_rects, horizontal_resize_edge_handles,
        horizontal_resize_edge_visual_rect, horizontal_resize_handles,
    };
    pub(super) use crate::{
        gui::{
            range::{IndexViewport, IndexViewportScope, NormalizedRange},
            types::{Point, Rect, Rgba8},
        },
        runtime::PaintPrimitive,
    };
}
