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
        ChannelViewMode, PointRenderMode, SpatialPanel, SpatialPoint, TimelineMarkerPreview,
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
