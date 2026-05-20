use super::super::{PointRenderMode, SpatialPanel, SpatialPoint, normalized_milli_point_in_rect};
use crate::gui::types::{Point, Rect};
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
