use super::super::super::*;
use crate::gui::types::{Point, Rect};

#[test]
fn timeline_lane_layout_divides_even_lanes() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 140.0));
    let lanes = TimelineLaneLayout::even(rect, 3);

    assert_eq!(lanes.lane_height(), 40.0);
    assert_eq!(
        lanes.lane_rect(1),
        Rect::from_min_max(Point::new(10.0, 60.0), Point::new(210.0, 100.0))
    );
    assert_eq!(lanes.lane_at(Point::new(40.0, 61.0)), Some(1));
}

#[test]
fn timeline_lane_layout_uses_fixed_height_and_clamps_last_lane() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 110.0));
    let lanes = TimelineLaneLayout::fixed_height(rect, 3, 48.0);

    assert_eq!(
        lanes.lane_rect(2),
        Rect::from_min_max(Point::new(0.0, 96.0), Point::new(100.0, 110.0))
    );
    assert_eq!(lanes.lane_at(Point::new(20.0, 109.0)), Some(2));
    assert_eq!(lanes.lane_at(Point::new(20.0, 140.0)), None);
}
