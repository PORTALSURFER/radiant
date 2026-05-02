//! Generic visualization primitives.

use std::sync::Arc;

use crate::gui::range::NormalizedRange;

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
        ChannelViewMode, PointRenderMode, SpatialPanel, SpatialPoint, TimelineEditPreview,
        TimelineMarkerPreview, TimelineViewport,
    };
    use crate::gui::range::NormalizedRange;
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
