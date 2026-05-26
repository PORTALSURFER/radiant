use super::super::super::*;
use crate::gui::types::{Point, Rect};

#[test]
fn timeline_item_layout_projects_centered_lane_items() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 140.0));
    let axis = TimelineAxis::new(rect, 0.0, 100.0).with_trailing_padding(20.0);
    let lanes = TimelineLaneLayout::fixed_height(rect, 3, 40.0);
    let items = TimelineItemLayout::new(axis, lanes, 18.0).with_horizontal_inset(2.0);

    assert_eq!(
        items.item_rect(1, 25.0, 50.0),
        Rect::from_min_max(Point::new(57.0, 71.0), Point::new(98.0, 89.0))
    );
}

#[test]
fn timeline_item_layout_sanitizes_height_and_inset() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 24.0));
    let axis = TimelineAxis::new(rect, 0.0, 10.0);
    let lanes = TimelineLaneLayout::fixed_height(rect, 1, 24.0);

    assert_eq!(
        TimelineItemLayout::new(axis, lanes, f32::NAN)
            .with_horizontal_inset(f32::INFINITY)
            .item_rect(0, 2.0, 4.0),
        Rect::from_min_max(Point::new(20.0, 0.0), Point::new(40.0, 24.0))
    );
    assert_eq!(
        TimelineItemLayout::new(axis, lanes, 120.0)
            .with_vertical_inset(f32::NAN)
            .item_rect(0, 2.0, 4.0),
        Rect::from_min_max(Point::new(20.0, 0.0), Point::new(40.0, 24.0))
    );
}

#[test]
fn timeline_item_layout_can_fill_lanes_inside_vertical_insets() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 80.0));
    let axis = TimelineAxis::new(rect, 0.0, 10.0);
    let lanes = TimelineLaneLayout::fixed_height(rect, 2, 40.0);

    assert_eq!(
        TimelineItemLayout::fill_lanes(axis, lanes)
            .with_vertical_inset(6.0)
            .item_rect(1, 2.0, 4.0),
        Rect::from_min_max(Point::new(20.0, 46.0), Point::new(40.0, 74.0))
    );
}
