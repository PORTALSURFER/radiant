//! Generic visualization primitives.

mod canvas;
mod signal;
mod spatial;
mod timeline;

pub use canvas::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts, DragHandle,
    DragHandleRole, canvas_layer_at_point, drag_handle_at_point,
};
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
