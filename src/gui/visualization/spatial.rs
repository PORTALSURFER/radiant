//! Generic spatial visualization primitives.

use std::sync::Arc;

use crate::gui::types::{Point, Rect};

/// Render mode for two-dimensional point-set visualizations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PointRenderMode {
    /// Rendered as a density heatmap.
    Heatmap,
    /// Rendered as individual points.
    #[default]
    Points,
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
pub struct SpatialPanelStatus {
    /// Whether the spatial panel is currently active.
    pub active: bool,
    /// Human-readable panel summary line.
    pub summary: String,
    /// Optional error text shown when spatial data cannot be loaded.
    pub error: Option<String>,
}

/// Product-neutral labels for one two-dimensional spatial visualization panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SpatialPanelLabels {
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
}

/// Selection and related-list focus state for one spatial visualization panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SpatialPanelSelection {
    /// Host item id currently selected in spatial state, when any.
    pub selected_item_id: Option<String>,
    /// Host item id currently focused from a related list, when any.
    pub focused_item_id: Option<String>,
}

/// Render mode and point payload for one spatial visualization panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SpatialPanelPoints {
    /// Current point render mode.
    pub render_mode: PointRenderMode,
    /// Points available for rendering in normalized spatial coordinates.
    pub points: Arc<[SpatialPoint]>,
}

/// Summary of one two-dimensional spatial visualization panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SpatialPanel {
    /// Activation, summary, and error state.
    pub status: SpatialPanelStatus,
    /// Product-neutral labels for projected spatial state.
    pub labels: SpatialPanelLabels,
    /// Selection and related-list focus state.
    pub selection: SpatialPanelSelection,
    /// Render mode and point payload.
    pub points: SpatialPanelPoints,
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
