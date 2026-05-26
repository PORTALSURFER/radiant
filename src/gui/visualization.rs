//! Generic visualization primitives.

mod canvas;
mod color_ramp;
mod grid;
mod signal;
mod spatial;
mod strip_layout;
mod timeline;
mod value_axis;
mod value_marker;

pub use canvas::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts, DragHandle,
    DragHandleRole, canvas_layer_at_point, canvas_selection_edge_handles,
    canvas_selection_edge_visual_rect, canvas_selection_rect, drag_handle_at_point,
    horizontal_resize_edge_bracket_rects, horizontal_resize_edge_handles,
    horizontal_resize_edge_visual_rect, horizontal_resize_handles,
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
pub use strip_layout::{HorizontalStripLayout, HorizontalStripLayoutParts};
pub use timeline::{
    TimelineAxis, TimelineAxisParts, TimelineCoordinateMapper, TimelineEditPreview,
    TimelineEditPreviewParts, TimelineFeedbackEvents, TimelineFeedbackParts, TimelineItemLayout,
    TimelineItemLayoutParts, TimelineLaneLayout, TimelineLaneLayoutParts, TimelineMarkerPreview,
    TimelineMotionState, TimelinePanelLayout, TimelinePanelLayoutParts, TimelinePitchItemLayout,
    TimelinePitchItemLayoutParts, TimelinePitchLayout, TimelinePitchLayoutParts,
    TimelinePresentationParts, TimelinePresentationState, TimelineSurfaceParts,
    TimelineSurfaceState, TimelineTransportParts, TimelineTransportState,
    TimelineValueMarkerLayout, TimelineValueMarkerLayoutParts, TimelineViewport,
    TimelineViewportParts,
};
pub use value_axis::{
    HorizontalLogValueAxis, HorizontalLogValueAxisParts, HorizontalValueAxis,
    HorizontalValueAxisParts, VerticalValueAxis, VerticalValueAxisParts,
};
pub use value_marker::{VerticalValueMarker, vertical_value_marker};

#[cfg(test)]
mod tests;
