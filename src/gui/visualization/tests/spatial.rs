use super::super::{
    PointRenderMode, SpatialPanel, SpatialPanelLabels, SpatialPanelPoints, SpatialPanelSelection,
    SpatialPanelStatus, SpatialPoint, normalized_milli_point_in_rect,
};
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

    assert!(!panel.status.active);
    assert_eq!(panel.points.render_mode, PointRenderMode::Points);
    assert!(panel.points.points.is_empty());
    assert_eq!(panel.selection.selected_item_id, None);
    assert_eq!(panel.selection.focused_item_id, None);
}

#[test]
fn spatial_panel_groups_labels_selection_and_point_data() {
    let panel = SpatialPanel {
        status: SpatialPanelStatus {
            active: true,
            summary: String::from("42 projected points"),
            error: None,
        },
        labels: SpatialPanelLabels {
            legend_label: String::from("points"),
            selection_label: String::from("selected"),
            hover_label: String::from("hovered"),
            cluster_label: String::from("cluster"),
            viewport_label: String::from("100%"),
        },
        selection: SpatialPanelSelection {
            selected_item_id: Some(String::from("item-1")),
            focused_item_id: Some(String::from("item-2")),
        },
        points: SpatialPanelPoints {
            render_mode: PointRenderMode::Heatmap,
            points: Arc::from([SpatialPoint {
                id: Arc::<str>::from("item-1"),
                x_milli: 250,
                y_milli: 750,
                cluster_id: Some(3),
            }]),
        },
    };

    assert!(panel.status.active);
    assert_eq!(panel.labels.viewport_label, "100%");
    assert_eq!(panel.selection.selected_item_id.as_deref(), Some("item-1"));
    assert_eq!(panel.points.render_mode, PointRenderMode::Heatmap);
    assert_eq!(panel.points.points.len(), 1);
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
