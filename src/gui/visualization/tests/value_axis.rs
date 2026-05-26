use super::super::VerticalValueAxis;
use crate::gui::types::{Point, Rect};

#[test]
fn vertical_value_axis_projects_bottom_up_values() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    let axis = VerticalValueAxis::new(rect, -12.0, 12.0);

    assert_eq!(axis.y_for_value(-12.0), 120.0);
    assert_eq!(axis.y_for_value(0.0), 70.0);
    assert_eq!(axis.y_for_value(12.0), 20.0);
}

#[test]
fn vertical_value_axis_reads_values_from_y() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    let axis = VerticalValueAxis::new(rect, -12.0, 12.0);

    assert_eq!(axis.value_for_y(120.0), -12.0);
    assert_eq!(axis.value_for_y(70.0), 0.0);
    assert_eq!(axis.value_for_y(20.0), 12.0);
}

#[test]
fn vertical_value_axis_clamps_values_and_coordinates() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    let axis = VerticalValueAxis::new(rect, -12.0, 12.0);

    assert_eq!(axis.y_for_value(24.0), 20.0);
    assert_eq!(axis.y_for_value(-24.0), 120.0);
    assert_eq!(axis.value_for_y(-80.0), 12.0);
    assert_eq!(axis.value_for_y(220.0), -12.0);
}

#[test]
fn vertical_value_axis_supports_descending_ranges() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    let axis = VerticalValueAxis::new(rect, 12.0, -12.0);

    assert_eq!(axis.y_for_value(12.0), 120.0);
    assert_eq!(axis.y_for_value(0.0), 70.0);
    assert_eq!(axis.y_for_value(-12.0), 20.0);
    assert_eq!(axis.value_for_y(70.0), 0.0);
}
