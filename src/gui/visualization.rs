//! Generic visualization primitives.

mod canvas;
mod signal;
mod spatial;
mod timeline;

pub use canvas::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, DragHandle, DragHandleRole,
    canvas_layer_at_point, drag_handle_at_point,
};
pub use signal::{
    ChannelViewMode, SignalChromeState, SignalRasterPreview, SignalToolFlags, SignalToolState,
};
pub use spatial::{PointRenderMode, SpatialPanel, SpatialPoint, normalized_milli_point_in_rect};
pub use timeline::{
    TimelineCoordinateMapper, TimelineEditPreview, TimelineFeedbackEvents, TimelineMarkerPreview,
    TimelineMotionState, TimelinePresentationState, TimelineSurfaceState, TimelineTransportState,
    TimelineViewport,
};

#[cfg(test)]
mod tests;
