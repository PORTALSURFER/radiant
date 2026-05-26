use super::super::super::*;
use crate::gui::types::{Point, Rect};

#[test]
fn timeline_axis_projects_visible_values_and_padding() {
    let rect = Rect::from_min_max(Point::new(10.0, 4.0), Point::new(210.0, 40.0));
    let axis = TimelineAxis::new(rect, 16.0, 80.0).with_trailing_padding(8.0);

    assert_eq!(axis.projection_rect().max.x, 202.0);
    assert_eq!(axis.x_for_value(16.0), 10.0);
    assert_eq!(axis.x_for_value(48.0), 106.0);
    assert_eq!(axis.x_for_value(80.0), 202.0);
    assert_eq!(axis.value_for_x(106.0), 48.0);
}

#[test]
fn timeline_axis_supports_unclamped_projection_for_offscreen_items() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 20.0));
    let axis = TimelineAxis::new(rect, 10.0, 20.0);

    assert_eq!(axis.x_for_value_unclamped(5.0), -50.0);
    assert_eq!(axis.x_for_value(5.0), 0.0);
    assert_eq!(axis.value_for_x_unclamped(125.0), 22.5);
    assert_eq!(axis.value_for_x(125.0), 20.0);
}
