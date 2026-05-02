//! Generic visualization primitives.

use std::sync::Arc;

use crate::gui::{range::NormalizedRange, types::ImageRgba};

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

/// Retained raster preview for a timeline, signal, or visualization surface.
///
/// Hosts may render expensive visualization content into an image, project a
/// stable signature for cache invalidation, and keep lightweight labels/loading
/// state alongside the shared pixel payload.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SignalRasterPreview {
    /// Display label for the loaded item, when any.
    pub loaded_label: Option<String>,
    /// Whether the preview is waiting for new source content.
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
/// The playhead can carry both milli and micro precision so render loops can
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

/// One-shot feedback event tokens for a normalized timeline.
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
/// Hosts decide whether those controls represent audio fades, animation ramps,
/// trim previews, or other domain behavior.
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
        ChannelViewMode, PointRenderMode, SignalChromeState, SignalRasterPreview, SignalToolState,
        SpatialPanel, SpatialPoint, TimelineEditPreview, TimelineFeedbackEvents,
        TimelineMarkerPreview, TimelineTransportState, TimelineViewport,
    };
    use crate::gui::{range::NormalizedRange, types::ImageRgba};
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
}
