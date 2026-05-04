//! Generic visualization primitives.

use std::sync::Arc;

use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport},
    types::{ImageRgba, Point, Rect},
};

/// Render mode for two-dimensional point-set visualizations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PointRenderMode {
    /// Rendered as a density heatmap.
    Heatmap,
    /// Rendered as individual points.
    #[default]
    Points,
}

/// Channel layout for visualizing one stream as a combined or split view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelViewMode {
    /// Collapse channels into one combined envelope.
    Mono,
    /// Render channels in a split stereo view.
    Stereo,
}

/// One point in normalized two-dimensional visualization space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialPoint {
    /// Stable host-owned identifier used for selection, focus, and actions.
    pub id: Arc<str>,
    /// X position normalized to milli-units (`0..=1000`) across visualization bounds.
    pub x_milli: u16,
    /// Y position normalized to milli-units (`0..=1000`) across visualization bounds.
    pub y_milli: u16,
    /// Optional cluster id for color grouping.
    pub cluster_id: Option<i32>,
}

/// Summary of one two-dimensional spatial visualization panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SpatialPanel {
    /// Whether the spatial panel is currently active.
    pub active: bool,
    /// Human-readable panel summary line.
    pub summary: String,
    /// Legend/status label for render mode and point density.
    pub legend_label: String,
    /// Selection/focus label for the currently highlighted item.
    pub selection_label: String,
    /// Hover label for the currently hovered item, when any.
    pub hover_label: String,
    /// Cluster summary label for projected points.
    pub cluster_label: String,
    /// Viewport label describing zoom/pan state.
    pub viewport_label: String,
    /// Optional error text shown when spatial data cannot be loaded.
    pub error: Option<String>,
    /// Current point render mode.
    pub render_mode: PointRenderMode,
    /// Host item id currently selected in spatial state, when any.
    pub selected_item_id: Option<String>,
    /// Host item id currently focused from a related list, when any.
    pub focused_item_id: Option<String>,
    /// Points available for rendering in normalized spatial coordinates.
    pub points: Arc<[SpatialPoint]>,
}

/// Project normalized milli-unit coordinates into a rectangular spatial canvas.
pub fn normalized_milli_point_in_rect(rect: Rect, x_milli: u16, y_milli: u16) -> Point {
    let x_ratio = f32::from(x_milli.min(1000)) / 1000.0;
    let y_ratio = f32::from(y_milli.min(1000)) / 1000.0;
    Point::new(
        rect.min.x + (rect.width().max(0.0) * x_ratio),
        rect.min.y + (rect.height().max(0.0) * y_ratio),
    )
}

/// Paint and input order for a generic layered canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanvasLayerOrder {
    /// Background or guide layer.
    Background,
    /// Primary content layer.
    Content,
    /// Selection, hover, or edit affordance layer.
    Interaction,
    /// Transient feedback layer.
    Feedback,
    /// Topmost focus or capture layer.
    Focus,
}

/// One retained canvas layer with optional input participation.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasLayer {
    /// Stable layer identifier.
    pub id: Arc<str>,
    /// Paint and hit-test order.
    pub order: CanvasLayerOrder,
    /// Layer bounds in canvas coordinates.
    pub bounds: Rect,
    /// Whether this layer participates in pointer hit testing.
    pub interactive: bool,
}

impl CanvasLayer {
    /// Build one retained canvas layer.
    pub fn new(
        id: impl Into<String>,
        order: CanvasLayerOrder,
        bounds: Rect,
        interactive: bool,
    ) -> Self {
        Self {
            id: Arc::<str>::from(id.into()),
            order,
            bounds,
            interactive,
        }
    }
}

/// Return the topmost interactive canvas layer containing `point`.
pub fn canvas_layer_at_point(layers: &[CanvasLayer], point: Point) -> Option<&str> {
    layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.interactive && layer.bounds.contains(point))
        .max_by_key(|(index, layer)| (layer.order, *index))
        .map(|(_, layer)| layer.id.as_ref())
}

/// Domain-neutral drag handle role for generic timeline and canvas editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DragHandleRole {
    /// Leading edge of a selected range or shape.
    Start,
    /// Trailing edge of a selected range or shape.
    End,
    /// Interior move handle for an existing selection or shape.
    Body,
    /// Leading auxiliary control.
    LeadingControl,
    /// Trailing auxiliary control.
    TrailingControl,
}

/// One hit-testable drag handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragHandle {
    /// Semantic role emitted to the host.
    pub role: DragHandleRole,
    /// Handle bounds in canvas coordinates.
    pub rect: Rect,
    /// Stable capture token for backends that keep drag ownership after press.
    pub capture_token: u64,
    /// Whether this handle currently accepts input.
    pub enabled: bool,
}

impl DragHandle {
    /// Build one enabled drag handle.
    pub fn new(role: DragHandleRole, rect: Rect, capture_token: u64) -> Self {
        Self {
            role,
            rect,
            capture_token,
            enabled: true,
        }
    }

    /// Set whether this handle accepts input.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Return the topmost enabled drag handle containing `point`.
pub fn drag_handle_at_point(handles: &[DragHandle], point: Point) -> Option<DragHandle> {
    handles
        .iter()
        .rev()
        .copied()
        .find(|handle| handle.enabled && handle.rect.contains(point))
}

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

/// Retained canvas/timeline invalidation summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasInvalidation {
    /// Primary retained content changed.
    pub content_changed: bool,
    /// Layer order, bounds, or hit-test participation changed.
    pub layers_changed: bool,
    /// Pointer capture or focused handle changed.
    pub interaction_changed: bool,
    /// Timeline projection or viewport changed.
    pub projection_changed: bool,
}

impl CanvasInvalidation {
    /// Return whether retained scene content must be rebuilt.
    pub fn requires_scene_rebuild(self) -> bool {
        self.content_changed || self.layers_changed || self.projection_changed
    }

    /// Return whether interaction overlays must be rebuilt.
    pub fn requires_interaction_overlay_rebuild(self) -> bool {
        self.requires_scene_rebuild() || self.interaction_changed
    }
}

/// Retained raster preview for a timeline, signal, or visualization surface.
///
/// Hosts may render expensive visualization content into an image, project a
/// stable signature for cache invalidation, and keep lightweight labels/loading
/// state alongside the shared pixel payload.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SignalRasterPreview {
    /// Display label for the loaded item, when any.
    pub loaded_label: Option<String>,
    /// Whether the preview is waiting for new input content.
    pub loading: bool,
    /// Whether a replacement image is still rendering in the background.
    pub image_rendering: bool,
    /// Stable signature for detecting image updates.
    pub image_signature: Option<u64>,
    /// Optional rasterized image payload.
    pub image: Option<Arc<ImageRgba>>,
}

/// Generic chrome/status state for a signal visualization surface.
///
/// This captures reusable display state such as a transport/status hint,
/// optional reference-anchor metadata, and channel layout. Host-specific tools
/// and edit modes should remain in host state or compatibility adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalChromeState {
    /// Extra status hint shown alongside visualization labels.
    pub status_hint: String,
    /// Whether a host-defined reference anchor is currently available.
    pub reference_anchor_available: bool,
    /// Label for the host-defined reference anchor, when available.
    pub reference_anchor_label: Option<String>,
    /// Channel layout used by the signal visualization.
    pub channel_view: ChannelViewMode,
}

/// Generic enabled/visible tool state for a signal visualization surface.
///
/// The fields intentionally describe interaction roles rather than domain
/// operations. Hosts map these booleans to product-specific tools such as snap
/// modes, overlays, review modes, or cleanup availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalToolState {
    /// Whether the visualization's current mode is locked against host updates.
    pub lock_enabled: bool,
    /// Whether alternate/audition behavior is enabled.
    pub audition_enabled: bool,
    /// Whether the primary snap behavior is enabled.
    pub primary_snap_enabled: bool,
    /// Whether grid/guide alignment uses a relative anchor.
    pub relative_grid_enabled: bool,
    /// Whether the secondary snap behavior is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether marker overlays are visible.
    pub markers_visible: bool,
    /// Whether marker/review mode is active.
    pub review_mode_enabled: bool,
    /// Whether a host-defined cleanup action is available.
    pub cleanup_available: bool,
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
pub struct TimelineMotionState<Marker = TimelineMarkerPreview> {
    /// Whether the host transport or animation input is currently running.
    pub transport_running: bool,
    /// Renderer-facing timeline surface state.
    pub surface: TimelineSurfaceState<Marker>,
    /// Reusable signal chrome/status state.
    pub chrome: SignalChromeState,
    /// Reusable signal tool state.
    pub tools: SignalToolState,
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

impl<Marker> TimelineMotionState<Marker> {
    /// Build timeline motion state from explicit generic parts.
    pub fn new(
        transport_running: bool,
        surface: TimelineSurfaceState<Marker>,
        chrome: SignalChromeState,
        tools: SignalToolState,
    ) -> Self {
        Self {
            transport_running,
            surface,
            chrome,
            tools,
        }
    }
}

impl Default for SignalToolState {
    fn default() -> Self {
        Self {
            lock_enabled: false,
            audition_enabled: false,
            primary_snap_enabled: false,
            relative_grid_enabled: false,
            secondary_snap_enabled: false,
            markers_visible: true,
            review_mode_enabled: false,
            cleanup_available: false,
        }
    }
}

impl SignalToolState {
    /// Build signal tool state from explicit generic flags.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        lock_enabled: bool,
        audition_enabled: bool,
        primary_snap_enabled: bool,
        relative_grid_enabled: bool,
        secondary_snap_enabled: bool,
        markers_visible: bool,
        review_mode_enabled: bool,
        cleanup_available: bool,
    ) -> Self {
        Self {
            lock_enabled,
            audition_enabled,
            primary_snap_enabled,
            relative_grid_enabled,
            secondary_snap_enabled,
            markers_visible,
            review_mode_enabled,
            cleanup_available,
        }
    }
}

impl Default for SignalChromeState {
    fn default() -> Self {
        Self {
            status_hint: String::from("idle"),
            reference_anchor_available: false,
            reference_anchor_label: None,
            channel_view: ChannelViewMode::Mono,
        }
    }
}

impl SignalChromeState {
    /// Build signal chrome state from explicit display values.
    pub fn new(
        status_hint: impl Into<String>,
        reference_anchor_available: bool,
        reference_anchor_label: Option<String>,
        channel_view: ChannelViewMode,
    ) -> Self {
        Self {
            status_hint: status_hint.into(),
            reference_anchor_available,
            reference_anchor_label,
            channel_view,
        }
    }
}

impl SignalRasterPreview {
    /// Build a retained raster preview from explicit state.
    pub fn new(
        loaded_label: Option<String>,
        loading: bool,
        image_rendering: bool,
        image_signature: Option<u64>,
        image: Option<Arc<ImageRgba>>,
    ) -> Self {
        Self {
            loaded_label,
            loading,
            image_rendering,
            image_signature,
            image,
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
    /// Whether this marker is marked for a host-defined output operation.
    pub marked_for_export: bool,
    /// Whether this marker belongs to a cleanup/review candidate batch.
    pub duplicate_cleanup_candidate: bool,
    /// Whether this marker is currently exempted from cleanup/review.
    pub duplicate_cleanup_exempted: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        CanvasInvalidation, CanvasLayer, CanvasLayerOrder, ChannelViewMode, DragHandle,
        DragHandleRole, PointRenderMode, SignalChromeState, SignalRasterPreview, SignalToolState,
        SpatialPanel, SpatialPoint, TimelineCoordinateMapper, TimelineEditPreview,
        TimelineFeedbackEvents, TimelineMarkerPreview, TimelineMotionState,
        TimelinePresentationState, TimelineSurfaceState, TimelineTransportState, TimelineViewport,
        canvas_layer_at_point, drag_handle_at_point, normalized_milli_point_in_rect,
    };
    use crate::gui::{
        range::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport},
        types::{ImageRgba, Point, Rect},
    };
    use std::sync::Arc;

    #[test]
    fn point_render_mode_defaults_to_points() {
        assert_eq!(PointRenderMode::default(), PointRenderMode::Points);
    }

    #[test]
    fn channel_view_mode_distinguishes_combined_and_split_views() {
        assert_ne!(ChannelViewMode::Mono, ChannelViewMode::Stereo);
    }

    #[test]
    fn spatial_point_preserves_normalized_coordinates_and_id() {
        let point = SpatialPoint {
            id: Arc::<str>::from("item-1"),
            x_milli: 250,
            y_milli: 750,
            cluster_id: Some(3),
        };

        assert_eq!(point.id.as_ref(), "item-1");
        assert_eq!(point.x_milli, 250);
        assert_eq!(point.y_milli, 750);
        assert_eq!(point.cluster_id, Some(3));
    }

    #[test]
    fn spatial_panel_defaults_to_inactive_empty_points() {
        let panel = SpatialPanel::default();

        assert!(!panel.active);
        assert_eq!(panel.render_mode, PointRenderMode::Points);
        assert!(panel.points.is_empty());
        assert_eq!(panel.selected_item_id, None);
        assert_eq!(panel.focused_item_id, None);
    }

    #[test]
    fn normalized_milli_point_projects_and_clamps_into_rect() {
        let rect = Rect::from_min_max(Point::new(100.0, 200.0), Point::new(300.0, 500.0));

        assert_eq!(
            normalized_milli_point_in_rect(rect, 250, 500),
            Point::new(150.0, 350.0)
        );
        assert_eq!(normalized_milli_point_in_rect(rect, 1400, 1300), rect.max);
    }

    #[test]
    fn canvas_layer_hit_testing_prefers_topmost_interactive_layer() {
        let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let layers = [
            CanvasLayer::new("base", CanvasLayerOrder::Background, bounds, true),
            CanvasLayer::new("paint", CanvasLayerOrder::Content, bounds, false),
            CanvasLayer::new(
                "handle",
                CanvasLayerOrder::Interaction,
                Rect::from_min_max(Point::new(40.0, 40.0), Point::new(60.0, 60.0)),
                true,
            ),
            CanvasLayer::new(
                "focus",
                CanvasLayerOrder::Focus,
                Rect::from_min_max(Point::new(45.0, 45.0), Point::new(55.0, 55.0)),
                true,
            ),
        ];

        assert_eq!(
            canvas_layer_at_point(&layers, Point::new(50.0, 50.0)),
            Some("focus")
        );
        assert_eq!(
            canvas_layer_at_point(&layers, Point::new(20.0, 20.0)),
            Some("base")
        );
        assert_eq!(
            canvas_layer_at_point(&layers, Point::new(120.0, 20.0)),
            None
        );
    }

    #[test]
    fn drag_handle_hit_testing_uses_reverse_paint_order_and_enabled_state() {
        let handles = [
            DragHandle::new(
                DragHandleRole::Body,
                Rect::from_min_max(Point::new(10.0, 10.0), Point::new(50.0, 30.0)),
                1,
            ),
            DragHandle::new(
                DragHandleRole::Start,
                Rect::from_min_max(Point::new(10.0, 10.0), Point::new(20.0, 30.0)),
                2,
            )
            .with_enabled(false),
            DragHandle::new(
                DragHandleRole::End,
                Rect::from_min_max(Point::new(40.0, 10.0), Point::new(50.0, 30.0)),
                3,
            ),
        ];

        assert_eq!(
            drag_handle_at_point(&handles, Point::new(45.0, 20.0)).map(|handle| handle.role),
            Some(DragHandleRole::End)
        );
        assert_eq!(
            drag_handle_at_point(&handles, Point::new(15.0, 20.0)).map(|handle| handle.role),
            Some(DragHandleRole::Body)
        );
        assert_eq!(drag_handle_at_point(&handles, Point::new(5.0, 20.0)), None);
    }

    #[test]
    fn timeline_coordinate_mapper_projects_and_back_projects_micros() {
        let viewport = TimelineViewport::new(250, 750, 250_000, 750_000, 250_000_000, 750_000_000);
        let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(210.0, 40.0));
        let mapper = TimelineCoordinateMapper::new(viewport, rect, NormalizedPixelSnap::Nearest);

        assert_eq!(
            viewport.normalized_viewport(),
            NormalizedViewport::from_micros(250_000, 750_000)
        );
        assert_eq!(mapper.x_for_micros(250_000), 10.0);
        assert_eq!(mapper.x_for_micros(500_000), 110.0);
        assert_eq!(
            mapper.x_range_for(NormalizedRange::from_micros(300_000, 700_000)),
            (30.0, 190.0)
        );
        assert_eq!(mapper.micros_for_x(110.0), 500_000);
    }

    #[test]
    fn canvas_invalidation_splits_scene_and_interaction_rebuilds() {
        let interaction = CanvasInvalidation {
            interaction_changed: true,
            ..CanvasInvalidation::default()
        };
        let projection = CanvasInvalidation {
            projection_changed: true,
            ..CanvasInvalidation::default()
        };

        assert!(!interaction.requires_scene_rebuild());
        assert!(interaction.requires_interaction_overlay_rebuild());
        assert!(projection.requires_scene_rebuild());
        assert!(projection.requires_interaction_overlay_rebuild());
    }

    #[test]
    fn signal_raster_preview_preserves_label_flags_signature_and_image() {
        let image = Arc::new(ImageRgba::new(1, 1, vec![255, 0, 0, 255]).unwrap());
        let preview = SignalRasterPreview::new(
            Some(String::from("preview")),
            true,
            false,
            Some(42),
            Some(Arc::clone(&image)),
        );

        assert_eq!(preview.loaded_label.as_deref(), Some("preview"));
        assert!(preview.loading);
        assert!(!preview.image_rendering);
        assert_eq!(preview.image_signature, Some(42));
        assert_eq!(preview.image.as_deref(), Some(image.as_ref()));
    }

    #[test]
    fn signal_chrome_state_preserves_status_reference_and_channel_view() {
        let chrome = SignalChromeState::new(
            "playing",
            true,
            Some(String::from("A")),
            ChannelViewMode::Stereo,
        );

        assert_eq!(chrome.status_hint, "playing");
        assert!(chrome.reference_anchor_available);
        assert_eq!(chrome.reference_anchor_label.as_deref(), Some("A"));
        assert_eq!(chrome.channel_view, ChannelViewMode::Stereo);
    }

    #[test]
    fn signal_tool_state_preserves_generic_interaction_flags() {
        let tools = SignalToolState::new(true, true, false, true, false, true, true, false);

        assert!(tools.lock_enabled);
        assert!(tools.audition_enabled);
        assert!(!tools.primary_snap_enabled);
        assert!(tools.relative_grid_enabled);
        assert!(!tools.secondary_snap_enabled);
        assert!(tools.markers_visible);
        assert!(tools.review_mode_enabled);
        assert!(!tools.cleanup_available);
    }

    #[test]
    fn timeline_transport_state_preserves_positions_and_resolves_micro_playhead() {
        let selection = NormalizedRange::new(100, 400);
        let transport = TimelineTransportState::new(Some(120), Some(250), None, Some(selection));

        assert_eq!(transport.cursor_milli, Some(120));
        assert_eq!(transport.playhead_milli, Some(250));
        assert_eq!(transport.resolved_playhead_micros(), Some(250_000));
        assert_eq!(transport.selection, Some(selection));

        let precise = TimelineTransportState::new(None, Some(250), Some(250_125), None);
        assert_eq!(precise.resolved_playhead_micros(), Some(250_125));
    }

    #[test]
    fn timeline_feedback_events_preserve_operation_tokens() {
        let events = TimelineFeedbackEvents::new(10, 20, 30);

        assert_eq!(events.primary_success_nonce, 10);
        assert_eq!(events.primary_failure_nonce, 20);
        assert_eq!(events.secondary_success_nonce, 30);
    }

    #[test]
    fn timeline_presentation_state_preserves_guides_repeat_and_labels() {
        let presentation = TimelinePresentationState::new(
            Some(125_000),
            10_000,
            true,
            Some(String::from("Guide 1")),
            Some(String::from("2x")),
        );

        assert_eq!(presentation.guide_step_micros, Some(125_000));
        assert_eq!(presentation.guide_origin_micros, 10_000);
        assert!(presentation.repeat_enabled);
        assert_eq!(presentation.primary_label.as_deref(), Some("Guide 1"));
        assert_eq!(presentation.viewport_label.as_deref(), Some("2x"));
    }

    #[test]
    fn timeline_viewport_defaults_to_full_normalized_range() {
        let viewport = TimelineViewport::default();

        assert_eq!(viewport.start_milli, 0);
        assert_eq!(viewport.end_milli, 1000);
        assert_eq!(viewport.start_micros, 0);
        assert_eq!(viewport.end_micros, 1_000_000);
        assert_eq!(viewport.start_nanos, 0);
        assert_eq!(viewport.end_nanos, 1_000_000_000);
    }

    #[test]
    fn timeline_edit_preview_preserves_selection_and_handle_positions() {
        let selection = NormalizedRange {
            start_milli: 200,
            end_milli: 800,
            start_micros: 200_000,
            end_micros: 800_000,
            start_nanos: 200_000_000,
            end_nanos: 800_000_000,
        };
        let preview = TimelineEditPreview::new(
            Some(selection),
            Some(300),
            Some(300_000),
            Some(240),
            Some(240_000),
            Some(420),
            Some(700),
            Some(700_000),
            Some(760),
            Some(760_000),
            Some(580),
        );

        assert_eq!(preview.selection, Some(selection));
        assert_eq!(preview.leading_end_micros, Some(300_000));
        assert_eq!(preview.leading_inner_start_milli, Some(240));
        assert_eq!(preview.leading_curve_milli, Some(420));
        assert_eq!(preview.trailing_start_milli, Some(700));
        assert_eq!(preview.trailing_inner_end_micros, Some(760_000));
        assert_eq!(preview.trailing_curve_milli, Some(580));
    }

    #[test]
    fn timeline_marker_preview_preserves_review_and_cleanup_state() {
        let marker = TimelineMarkerPreview {
            range: NormalizedRange {
                start_milli: 100,
                end_milli: 200,
                start_micros: 100_000,
                end_micros: 200_000,
                start_nanos: 100_000_000,
                end_nanos: 200_000_000,
            },
            selected: true,
            focused: false,
            marked_for_export: true,
            duplicate_cleanup_candidate: true,
            duplicate_cleanup_exempted: false,
        };

        assert_eq!(marker.range.start_micros, 100_000);
        assert!(marker.selected);
        assert!(!marker.focused);
        assert!(marker.marked_for_export);
        assert!(marker.duplicate_cleanup_candidate);
        assert!(!marker.duplicate_cleanup_exempted);
    }

    #[test]
    fn timeline_surface_state_aggregates_generic_timeline_parts() {
        let marker = TimelineMarkerPreview {
            range: NormalizedRange::new(100, 200),
            selected: true,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: false,
            duplicate_cleanup_exempted: false,
        };
        let surface = TimelineSurfaceState::new(
            TimelineViewport::new(10, 900, 10_000, 900_000, 10_000_000, 900_000_000),
            TimelineTransportState::new(Some(20), Some(30), Some(30_500), None),
            TimelineEditPreview::default(),
            TimelineFeedbackEvents::new(1, 2, 3),
            TimelinePresentationState::new(None, 0, true, Some(String::from("tempo")), None),
            SignalRasterPreview::default(),
            vec![marker],
        );

        assert_eq!(surface.viewport.start_micros, 10_000);
        assert_eq!(surface.transport.resolved_playhead_micros(), Some(30_500));
        assert_eq!(surface.feedback_events.primary_failure_nonce, 2);
        assert!(surface.presentation.repeat_enabled);
        assert_eq!(surface.markers.len(), 1);
    }

    #[test]
    fn timeline_motion_state_aggregates_surface_chrome_tools_and_transport() {
        let motion = TimelineMotionState::new(
            true,
            TimelineSurfaceState::new(
                TimelineViewport::new(0, 500, 0, 500_000, 0, 500_000_000),
                TimelineTransportState::new(None, Some(10), Some(10_250), None),
                TimelineEditPreview::default(),
                TimelineFeedbackEvents::default(),
                TimelinePresentationState::default(),
                SignalRasterPreview::default(),
                Vec::<TimelineMarkerPreview>::new(),
            ),
            SignalChromeState::new(
                "moving",
                true,
                Some(String::from("anchor")),
                ChannelViewMode::Mono,
            ),
            SignalToolState::new(false, true, true, false, true, true, false, true),
        );

        assert!(motion.transport_running);
        assert_eq!(motion.surface.viewport.end_micros, 500_000);
        assert_eq!(
            motion.surface.transport.resolved_playhead_micros(),
            Some(10_250)
        );
        assert_eq!(motion.chrome.status_hint, "moving");
        assert!(motion.chrome.reference_anchor_available);
        assert!(motion.tools.audition_enabled);
        assert!(motion.tools.cleanup_available);
    }
}
