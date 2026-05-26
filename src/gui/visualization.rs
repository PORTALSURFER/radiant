//! Generic visualization primitives.

mod canvas;
mod color_ramp;
mod grid;
mod signal;
mod spatial;
mod timeline;

pub use canvas::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts, DragHandle,
    DragHandleRole, canvas_layer_at_point, canvas_selection_edge_handles,
    canvas_selection_edge_visual_rect, canvas_selection_rect, drag_handle_at_point,
    horizontal_resize_edge_handles, horizontal_resize_edge_visual_rect, horizontal_resize_handles,
};
pub use color_ramp::{ColorRamp, ColorRampStop};
pub use grid::{DenseGridCell, DenseGridLayout, DenseGridLayoutParts};
pub use signal::{
    ChannelViewMode, SignalChromeParts, SignalChromeState, SignalRasterPreview,
    SignalRasterPreviewParts, SignalToolFlags, SignalToolState,
};
pub use spatial::{
    PointRenderMode, SpatialPanel, SpatialPanelLabels, SpatialPanelPoints, SpatialPanelSelection,
    SpatialPanelStatus, SpatialPoint, normalized_milli_point_in_rect,
};
pub use timeline::{
    TimelineAxis, TimelineAxisParts, TimelineCoordinateMapper, TimelineEditPreview,
    TimelineEditPreviewParts, TimelineFeedbackEvents, TimelineFeedbackParts, TimelineLaneLayout,
    TimelineLaneLayoutParts, TimelineMarkerPreview, TimelineMotionState, TimelinePresentationParts,
    TimelinePresentationState, TimelineSurfaceParts, TimelineSurfaceState, TimelineTransportParts,
    TimelineTransportState, TimelineViewport, TimelineViewportParts,
};

#[cfg(test)]
mod tests;
