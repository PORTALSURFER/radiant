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
pub use spatial::{PointRenderMode, SpatialPanel, SpatialPoint, normalized_milli_point_in_rect};
pub use timeline::{
    TimelineCoordinateMapper, TimelineEditPreview, TimelineEditPreviewParts,
    TimelineFeedbackEvents, TimelineMarkerPreview, TimelineMotionState, TimelinePresentationState,
    TimelineSurfaceParts, TimelineSurfaceState, TimelineTransportState, TimelineViewport,
};

#[cfg(test)]
mod tests;
