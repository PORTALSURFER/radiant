use super::super::vertical_value_marker;
use crate::gui::types::{Point, Rect};

#[test]
fn vertical_value_marker_projects_stem_and_handle() {
    let lane = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    let marker = vertical_value_marker(lane, 60.0, 0.75, 2.0, 8.0).expect("marker");

    assert_eq!(
        marker.stem,
        Rect::from_min_max(Point::new(59.0, 45.0), Point::new(61.0, 120.0))
    );
    assert_eq!(
        marker.handle,
        Rect::from_min_max(Point::new(56.0, 41.0), Point::new(64.0, 49.0))
    );
}

#[test]
fn vertical_value_marker_clamps_value_but_allows_offscreen_x() {
    let lane = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    let marker = vertical_value_marker(lane, -20.0, 1.5, 2.0, 8.0).expect("marker");

    assert_eq!(
        marker.stem,
        Rect::from_min_max(Point::new(-21.0, 20.0), Point::new(-19.0, 120.0))
    );
    assert_eq!(
        marker.handle,
        Rect::from_min_max(Point::new(-24.0, 16.0), Point::new(-16.0, 24.0))
    );
}

#[test]
fn vertical_value_marker_rejects_invalid_geometry() {
    let lane = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        vertical_value_marker(lane.empty_at_min(), 60.0, 0.5, 2.0, 8.0),
        None
    );
    assert_eq!(vertical_value_marker(lane, f32::NAN, 0.5, 2.0, 8.0), None);
    assert_eq!(vertical_value_marker(lane, 60.0, 0.5, 0.0, 8.0), None);
    assert_eq!(vertical_value_marker(lane, 60.0, 0.5, 2.0, 0.0), None);
}
