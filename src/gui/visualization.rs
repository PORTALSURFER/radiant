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

#[cfg(test)]
mod tests {
    use super::{PointRenderMode, SpatialPoint};
    use std::sync::Arc;

    #[test]
    fn point_render_mode_defaults_to_points() {
        assert_eq!(PointRenderMode::default(), PointRenderMode::Points);
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
}
