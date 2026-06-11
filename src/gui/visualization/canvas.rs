//! Generic retained canvas interaction primitives.

mod drag_handle;
mod invalidation;
mod layer;
mod numeric;
mod resize;
mod selection_affordance;
mod selection_geometry;

pub use drag_handle::{DragHandle, DragHandleRole, drag_handle_at_point};
pub use invalidation::CanvasInvalidation;
pub use layer::{CanvasLayer, CanvasLayerOrder, CanvasLayerParts, canvas_layer_at_point};
pub use resize::{
    horizontal_resize_edge_bracket_rects, horizontal_resize_edge_handles,
    horizontal_resize_edge_visual_rect, horizontal_resize_handles,
};
pub use selection_affordance::{
    CanvasSelectionAffordanceHitTestParts, CanvasSelectionAffordancePaintParts,
    CanvasSelectionAffordanceStyle, CanvasSelectionBodyHandleHitTestParts,
    CanvasSelectionBodyHandlePaintParts, CanvasSelectionBodyHandleStyle,
    CanvasSelectionEdgeHitTestParts, CanvasSelectionEdgeVisualPaintParts,
    CanvasSelectionEdgeVisualStyle, CanvasSelectionPaintStyle,
    CanvasSelectionTrailingControlHitTestParts, CanvasSelectionTrailingControlPaintParts,
    CanvasSelectionTrailingControlStyle,
};
pub use selection_geometry::{
    CanvasSelectionBodyHandleParts, CanvasSelectionGeometry, CanvasSelectionGeometryParts,
    canvas_selection_body_handle_rect, canvas_selection_edge_handles,
    canvas_selection_edge_visual_rect, canvas_selection_rect,
    canvas_selection_trailing_control_rect,
};
