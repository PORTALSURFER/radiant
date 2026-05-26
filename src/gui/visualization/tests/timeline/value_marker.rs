use super::super::super::{TimelineAxis, TimelineValueMarkerLayout};
use crate::gui::types::{Point, Rect};

#[test]
fn timeline_value_marker_layout_projects_marker_geometry() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let axis = TimelineAxis::new(rect, 2.0, 6.0);
    let layout = TimelineValueMarkerLayout::new(axis, 2.0, 8.0);

    let marker = layout.marker(4.0, 0.75).expect("marker");

    assert_eq!(
        marker.stem,
        Rect::from_min_max(Point::new(109.0, 45.0), Point::new(111.0, 120.0))
    );
    assert_eq!(
        marker.handle,
        Rect::from_min_max(Point::new(106.0, 41.0), Point::new(114.0, 49.0))
    );
}

#[test]
fn timeline_value_marker_layout_can_project_unclamped_positions() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let axis = TimelineAxis::new(rect, 2.0, 6.0);
    let layout = TimelineValueMarkerLayout::new(axis, 2.0, 8.0);

    let marker = layout.marker_unclamped(1.0, 1.5).expect("marker");

    assert_eq!(
        marker.stem,
        Rect::from_min_max(Point::new(-41.0, 20.0), Point::new(-39.0, 120.0))
    );
    assert_eq!(
        marker.handle,
        Rect::from_min_max(Point::new(-44.0, 16.0), Point::new(-36.0, 24.0))
    );
}
