//! Visualization and canvas geometry prelude exports.

pub use crate::gui::visualization::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts,
    CanvasSelectionBodyHandleParts, CanvasSelectionGeometry, CanvasSelectionGeometryParts,
    ColorRamp, ColorRampStop, DenseGridCell, DenseGridLabelLayout, DenseGridLabelLayoutParts,
    DenseGridLayout, DenseGridLayoutParts, DenseGridRasterLayout, DenseGridRasterLayoutParts,
    DenseGridRowOrigin, DragHandle, DragHandleRole, HorizontalLogValueAxis,
    HorizontalLogValueAxisParts, HorizontalStripLayout, HorizontalStripLayoutParts,
    HorizontalValueAxis, HorizontalValueAxisParts, SampledCurveStrokeParts, TimelineAxis,
    TimelineAxisParts, TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry,
    TimelineEditPreview, TimelineEditPreviewParts, TimelineItemLayout, TimelineItemLayoutParts,
    TimelineLaneLayout, TimelineLaneLayoutParts, TimelinePanelLayout, TimelinePanelLayoutParts,
    TimelinePitchItemLayout, TimelinePitchItemLayoutParts, TimelinePitchLayout,
    TimelinePitchLayoutParts, TimelineValueMarkerLayout, TimelineValueMarkerLayoutParts,
    TimelineViewport, TimelineViewportParts, VerticalStripStackLayout,
    VerticalStripStackLayoutParts, VerticalStripStackOrigin, VerticalValueAxis,
    VerticalValueAxisParts, canvas_layer_at_point, canvas_selection_body_handle_rect,
    canvas_selection_edge_handles, canvas_selection_edge_visual_rect, canvas_selection_rect,
    canvas_selection_trailing_control_rect, drag_handle_at_point,
    horizontal_resize_edge_bracket_rects, push_sampled_curve_stroke, sampled_curve_points,
};
