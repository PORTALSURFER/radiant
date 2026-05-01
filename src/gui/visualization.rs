//! Generic visualization primitives.

use std::sync::Arc;

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

#[cfg(test)]
mod tests {
    use super::{ChannelViewMode, PointRenderMode, SpatialPanel, SpatialPoint};
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
}
