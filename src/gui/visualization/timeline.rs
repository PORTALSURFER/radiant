//! Generic timeline visualization primitives.

use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport},
    types::Rect,
};

use super::{SignalChromeState, SignalRasterPreview, SignalToolState};

/// Mapper between normalized timeline coordinates and local canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineCoordinateMapper {
    /// Normalized timeline viewport.
    pub viewport: TimelineViewport,
    /// Local canvas rect used for projection.
    pub rect: Rect,
    /// Pixel snapping policy.
    pub snap: NormalizedPixelSnap,
}

impl TimelineCoordinateMapper {
    /// Build a mapper for one timeline viewport and canvas rect.
    pub fn new(viewport: TimelineViewport, rect: Rect, snap: NormalizedPixelSnap) -> Self {
        Self {
            viewport,
            rect,
            snap,
        }
    }

    /// Project one normalized micro position into local x coordinates.
    pub fn x_for_micros(self, micros: u32) -> f32 {
        self.viewport
            .normalized_viewport()
            .x_for_micros(self.rect, micros, self.snap)
    }

    /// Project one normalized range into local x bounds.
    pub fn x_range_for(self, range: NormalizedRange) -> (f32, f32) {
        (
            self.x_for_micros(range.start_micros),
            self.x_for_micros(range.end_micros),
        )
    }

    /// Convert a local x coordinate back into normalized micro units.
    pub fn micros_for_x(self, x: f32) -> u32 {
        if self.rect.width() <= f32::EPSILON {
            return self.viewport.start_micros.min(1_000_000);
        }
        let local_ratio = ((x - self.rect.min.x) / self.rect.width()).clamp(0.0, 1.0) as f64;
        let viewport = self.viewport.normalized_viewport();
        ((viewport.start_ratio + (local_ratio * viewport.width_ratio)).clamp(0.0, 1.0)
            * 1_000_000.0)
            .round() as u32
    }
}

/// Cursor, playhead, and selected range for a normalized timeline.
///
/// The playhead can carry both milli and micro precision so render passes can
/// use a coarse label while preserving smoother motion when hosts provide it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineTransportState {
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units.
    pub playhead_micros: Option<u32>,
    /// Selected playback/review range.
    pub selection: Option<NormalizedRange>,
}

/// Single-use feedback event tokens for a normalized timeline.
///
/// Hosts increment these counters when user-visible operations complete or
/// fail. Radiant renderers can compare tokens across frames and show transient
/// feedback without owning host-specific timestamps, operation names, or domain
/// workflows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineFeedbackEvents {
    /// Token for the primary successful timeline operation.
    pub primary_success_nonce: u64,
    /// Token for the primary failed timeline operation.
    pub primary_failure_nonce: u64,
    /// Token for a secondary successful timeline operation.
    pub secondary_success_nonce: u64,
}

/// Presentation metadata for a normalized timeline.
///
/// This covers renderer-facing timeline guides, repeat state, and compact
/// labels without tying the primitive to any host domain concept.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TimelinePresentationState {
    /// Optional guide spacing in normalized micro-units.
    pub guide_step_micros: Option<u32>,
    /// Guide origin in normalized micro-units.
    pub guide_origin_micros: u32,
    /// Whether repeat playback/review behavior is enabled.
    pub repeat_enabled: bool,
    /// Optional primary metadata label.
    pub primary_label: Option<String>,
    /// Optional viewport/zoom metadata label.
    pub viewport_label: Option<String>,
}

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

impl TimelineTransportState {
    /// Build a timeline transport state from explicit normalized values.
    pub fn new(
        cursor_milli: Option<u16>,
        playhead_milli: Option<u16>,
        playhead_micros: Option<u32>,
        selection: Option<NormalizedRange>,
    ) -> Self {
        Self {
            cursor_milli,
            playhead_milli,
            playhead_micros,
            selection,
        }
    }

    /// Return the most precise available playhead value in micro-units.
    pub fn resolved_playhead_micros(self) -> Option<u32> {
        self.playhead_micros
            .or_else(|| self.playhead_milli.map(|milli| u32::from(milli) * 1000))
    }
}

impl TimelineFeedbackEvents {
    /// Build timeline feedback events from explicit monotonic tokens.
    pub fn new(
        primary_success_nonce: u64,
        primary_failure_nonce: u64,
        secondary_success_nonce: u64,
    ) -> Self {
        Self {
            primary_success_nonce,
            primary_failure_nonce,
            secondary_success_nonce,
        }
    }
}

impl TimelinePresentationState {
    /// Build timeline presentation state from explicit guide and label values.
    pub fn new(
        guide_step_micros: Option<u32>,
        guide_origin_micros: u32,
        repeat_enabled: bool,
        primary_label: Option<String>,
        viewport_label: Option<String>,
    ) -> Self {
        Self {
            guide_step_micros,
            guide_origin_micros,
            repeat_enabled,
            primary_label,
            viewport_label,
        }
    }
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

/// Visible normalized viewport for a timeline or signal visualization.
///
/// The same range is kept at milli, micro, and nano precision so hosts can
/// use coarse labels and deep-zoom pointer mapping without recomputing bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineViewport {
    /// Visible viewport start in normalized milli-units.
    pub start_milli: u16,
    /// Visible viewport end in normalized milli-units.
    pub end_milli: u16,
    /// Visible viewport start in normalized micro-units.
    pub start_micros: u32,
    /// Visible viewport end in normalized micro-units.
    pub end_micros: u32,
    /// Visible viewport start in normalized nanounits.
    pub start_nanos: u32,
    /// Visible viewport end in normalized nanounits.
    pub end_nanos: u32,
}

/// Editable range and fade handles for a normalized timeline or signal view.
///
/// The structure is deliberately host-neutral: it models a selected interval,
/// optional leading/trailing handle positions, and optional curve controls.
/// Hosts decide whether those controls represent animation ramps, trim previews,
/// easing handles, or other domain behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineEditPreview {
    /// Range currently being edited.
    pub selection: Option<NormalizedRange>,
    /// End position for the leading/top handle in normalized milli-units.
    pub leading_end_milli: Option<u16>,
    /// End position for the leading/top handle in normalized micro-units.
    pub leading_end_micros: Option<u32>,
    /// Start position for the leading/bottom handle in normalized milli-units.
    pub leading_inner_start_milli: Option<u16>,
    /// Start position for the leading/bottom handle in normalized micro-units.
    pub leading_inner_start_micros: Option<u32>,
    /// Leading curve tension in normalized milli-units.
    pub leading_curve_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized milli-units.
    pub trailing_start_milli: Option<u16>,
    /// Start position for the trailing/top handle in normalized micro-units.
    pub trailing_start_micros: Option<u32>,
    /// End position for the trailing/bottom handle in normalized milli-units.
    pub trailing_inner_end_milli: Option<u16>,
    /// End position for the trailing/bottom handle in normalized micro-units.
    pub trailing_inner_end_micros: Option<u32>,
    /// Trailing curve tension in normalized milli-units.
    pub trailing_curve_milli: Option<u16>,
}

impl TimelineEditPreview {
    /// Build an edit preview with all handle positions supplied explicitly.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        selection: Option<NormalizedRange>,
        leading_end_milli: Option<u16>,
        leading_end_micros: Option<u32>,
        leading_inner_start_milli: Option<u16>,
        leading_inner_start_micros: Option<u32>,
        leading_curve_milli: Option<u16>,
        trailing_start_milli: Option<u16>,
        trailing_start_micros: Option<u32>,
        trailing_inner_end_milli: Option<u16>,
        trailing_inner_end_micros: Option<u32>,
        trailing_curve_milli: Option<u16>,
    ) -> Self {
        Self {
            selection,
            leading_end_milli,
            leading_end_micros,
            leading_inner_start_milli,
            leading_inner_start_micros,
            leading_curve_milli,
            trailing_start_milli,
            trailing_start_micros,
            trailing_inner_end_milli,
            trailing_inner_end_micros,
            trailing_curve_milli,
        }
    }
}

impl TimelineViewport {
    /// Build a timeline viewport from explicit normalized bounds.
    pub fn new(
        start_milli: u16,
        end_milli: u16,
        start_micros: u32,
        end_micros: u32,
        start_nanos: u32,
        end_nanos: u32,
    ) -> Self {
        Self {
            start_milli,
            end_milli,
            start_micros,
            end_micros,
            start_nanos,
            end_nanos,
        }
    }

    /// Return this viewport as a generic normalized viewport projector.
    pub fn normalized_viewport(self) -> NormalizedViewport {
        NormalizedViewport::from_bounds(
            self.start_micros,
            self.end_micros,
            Some(self.start_nanos),
            Some(self.end_nanos),
        )
    }
}

impl Default for TimelineViewport {
    fn default() -> Self {
        Self {
            start_milli: 0,
            end_milli: 1000,
            start_micros: 0,
            end_micros: 1_000_000,
            start_nanos: 0,
            end_nanos: 1_000_000_000,
        }
    }
}

/// Generic marker preview for a normalized timeline or signal visualization.
///
/// The range is expressed in normalized milli, micro, and nano precision so
/// hosts can project markers into deeply zoomed timelines without losing
/// pointer precision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineMarkerPreview {
    /// Marker range in normalized timeline precision.
    pub range: NormalizedRange,
    /// Whether this marker is currently selected for edit operations.
    pub selected: bool,
    /// Whether this marker is focused for keyboard review.
    pub focused: bool,
}
