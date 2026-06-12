use crate::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    runtime::{PaintPrimitive, push_fill_rect},
    widgets::WidgetId,
};

/// Edge for dense-row marker geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DenseRowMarkerEdge {
    /// Marker is inset from the leading edge.
    #[default]
    Leading,
    /// Marker is inset from the trailing edge.
    Trailing,
}

/// Named fields for projecting a vertical row marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseRowMarkerParts {
    /// Which horizontal edge owns the marker.
    pub edge: DenseRowMarkerEdge,
    /// Marker width in logical pixels.
    pub width: f32,
    /// Inset from the owning horizontal edge.
    pub edge_inset: f32,
    /// Inset applied to top and bottom before centering the marker.
    pub vertical_inset: f32,
    /// Minimum marker height when the row is taller than the inset area.
    pub min_height: f32,
}

/// Marker paint for a dense-row edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseRowMarkerStyle {
    /// Marker geometry.
    pub parts: DenseRowMarkerParts,
    /// Marker fill color.
    pub color: Rgba8,
}

impl DenseRowMarkerStyle {
    /// Build marker paint from geometry and color.
    pub const fn new(parts: DenseRowMarkerParts, color: Rgba8) -> Self {
        Self { parts, color }
    }
}

impl DenseRowMarkerParts {
    /// Build marker parts for a leading-edge marker.
    pub const fn leading(width: f32) -> Self {
        Self::new(DenseRowMarkerEdge::Leading, width)
    }

    /// Build marker parts for a trailing-edge marker.
    pub const fn trailing(width: f32) -> Self {
        Self::new(DenseRowMarkerEdge::Trailing, width)
    }

    /// Build marker parts for the supplied edge.
    pub const fn new(edge: DenseRowMarkerEdge, width: f32) -> Self {
        Self {
            edge,
            width,
            edge_inset: 1.0,
            vertical_inset: 3.0,
            min_height: 8.0,
        }
    }

    /// Set inset from the owning horizontal edge.
    pub const fn edge_inset(mut self, inset: f32) -> Self {
        self.edge_inset = inset;
        self
    }

    /// Set top and bottom inset before centering the marker.
    pub const fn vertical_inset(mut self, inset: f32) -> Self {
        self.vertical_inset = inset;
        self
    }

    /// Set the minimum marker height.
    pub const fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }
}

/// Project a vertically centered marker on one edge of a dense row.
pub fn dense_row_vertical_marker_rect(bounds: Rect, parts: DenseRowMarkerParts) -> Option<Rect> {
    if parts.width <= 0.0
        || parts.edge_inset < 0.0
        || parts.vertical_inset < 0.0
        || parts.min_height < 0.0
        || !parts.width.is_finite()
        || !parts.edge_inset.is_finite()
        || !parts.vertical_inset.is_finite()
        || !parts.min_height.is_finite()
        || bounds.width() <= 0.0
        || bounds.height() <= 0.0
    {
        return None;
    }
    let available_height = (bounds.height() - parts.vertical_inset * 2.0).max(0.0);
    let marker_height = available_height.max(parts.min_height).min(bounds.height());
    let x = match parts.edge {
        DenseRowMarkerEdge::Leading => bounds.min.x + parts.edge_inset,
        DenseRowMarkerEdge::Trailing => bounds.max.x - parts.edge_inset - parts.width,
    };
    Some(Rect::from_min_size(
        Point::new(x, bounds.min.y + (bounds.height() - marker_height) * 0.5),
        Vector2::new(parts.width, marker_height),
    ))
}

/// Push a vertically centered marker on one edge of a dense row.
///
/// Returns `true` when a marker primitive was appended.
pub fn push_dense_row_vertical_marker(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    parts: DenseRowMarkerParts,
    color: Rgba8,
) -> bool {
    let Some(rect) = dense_row_vertical_marker_rect(bounds, parts) else {
        return false;
    };
    push_fill_rect(primitives, widget_id, rect, color);
    true
}
