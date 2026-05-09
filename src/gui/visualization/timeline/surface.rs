use super::{
    TimelineEditPreview, TimelineFeedbackEvents, TimelineMarkerPreview, TimelinePresentationState,
    TimelineTransportState, TimelineViewport,
};
use crate::gui::visualization::{SignalChromeState, SignalRasterPreview, SignalToolState};

/// Aggregated renderer-facing state for a normalized timeline surface.
///
/// Hosts can project domain-specific media, history, or document timelines into
/// these generic slots while keeping product workflow state outside Radiant.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TimelineSurfaceState<Marker = TimelineMarkerPreview> {
    /// Visible viewport bounds.
    pub viewport: TimelineViewport,
    /// Cursor, playhead, and selected range state.
    pub transport: TimelineTransportState,
    /// Editable range and handle preview state.
    pub edit_preview: TimelineEditPreview,
    /// Single-use transient feedback tokens.
    pub feedback_events: TimelineFeedbackEvents,
    /// Guide, repeat, and label presentation state.
    pub presentation: TimelinePresentationState,
    /// Retained raster preview for the timeline body.
    pub raster_preview: SignalRasterPreview,
    /// Host-projected markers shown on the timeline.
    pub markers: Vec<Marker>,
}

/// Motion-frame state for a normalized timeline or signal visualization.
///
/// This groups the retained timeline surface with reusable chrome and tool
/// state for render passes that update overlays between full host projections.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TimelineMotionState<Marker = TimelineMarkerPreview, Tools = SignalToolState> {
    /// Whether the host transport or animation input is currently running.
    pub transport_running: bool,
    /// Renderer-facing timeline surface state.
    pub surface: TimelineSurfaceState<Marker>,
    /// Reusable signal chrome/status state.
    pub chrome: SignalChromeState,
    /// Reusable signal tool state.
    pub tools: Tools,
}

impl<Marker> TimelineSurfaceState<Marker> {
    /// Build a timeline surface state from explicit generic parts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        viewport: TimelineViewport,
        transport: TimelineTransportState,
        edit_preview: TimelineEditPreview,
        feedback_events: TimelineFeedbackEvents,
        presentation: TimelinePresentationState,
        raster_preview: SignalRasterPreview,
        markers: Vec<Marker>,
    ) -> Self {
        Self {
            viewport,
            transport,
            edit_preview,
            feedback_events,
            presentation,
            raster_preview,
            markers,
        }
    }
}

impl<Marker, Tools> TimelineMotionState<Marker, Tools> {
    /// Build timeline motion state from explicit generic parts.
    pub fn new(
        transport_running: bool,
        surface: TimelineSurfaceState<Marker>,
        chrome: SignalChromeState,
        tools: Tools,
    ) -> Self {
        Self {
            transport_running,
            surface,
            chrome,
            tools,
        }
    }
}
