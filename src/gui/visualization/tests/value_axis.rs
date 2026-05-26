use super::super::{HorizontalLogValueAxis, HorizontalValueAxis, VerticalValueAxis};
use crate::gui::types::{Point, Rect};

#[test]
fn horizontal_value_axis_projects_values() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let axis = HorizontalValueAxis::new(rect, -1.0, 1.0);

    assert_eq!(axis.x_for_value(-1.0), 10.0);
    assert_eq!(axis.x_for_value(0.0), 110.0);
    assert_eq!(axis.x_for_value(1.0), 210.0);
}

#[test]
fn horizontal_value_axis_reads_values_from_x() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let axis = HorizontalValueAxis::new(rect, -1.0, 1.0);

    assert_eq!(axis.value_for_x(10.0), -1.0);
    assert_eq!(axis.value_for_x(110.0), 0.0);
    assert_eq!(axis.value_for_x(210.0), 1.0);
}

#[test]
fn horizontal_value_axis_clamps_values_and_coordinates() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let axis = HorizontalValueAxis::new(rect, -1.0, 1.0);

    assert_eq!(axis.x_for_value(-2.0), 10.0);
    assert_eq!(axis.x_for_value(2.0), 210.0);
    assert_eq!(axis.value_for_x(-90.0), -1.0);
    assert_eq!(axis.value_for_x(310.0), 1.0);
}

#[test]
fn horizontal_value_axis_supports_descending_ranges() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let axis = HorizontalValueAxis::new(rect, 1.0, -1.0);

    assert_eq!(axis.x_for_value(1.0), 10.0);
    assert_eq!(axis.x_for_value(0.0), 110.0);
    assert_eq!(axis.x_for_value(-1.0), 210.0);
    assert_eq!(axis.value_for_x(110.0), 0.0);
}

#[test]
fn horizontal_log_value_axis_projects_positive_values() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 120.0));
    let axis = HorizontalLogValueAxis::new(rect, 20.0, 20_000.0);

    assert_eq!(axis.x_for_value(20.0), 10.0);
    assert!((axis.x_for_value(200.0) - 110.0).abs() < 0.001);
    assert!((axis.x_for_value(2_000.0) - 210.0).abs() < 0.001);
    assert_eq!(axis.x_for_value(20_000.0), 310.0);
}

#[test]
fn horizontal_log_value_axis_reads_values_from_x_and_ratio() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 120.0));
    let axis = HorizontalLogValueAxis::new(rect, 20.0, 20_000.0);

    assert!((axis.value_for_x(10.0) - 20.0).abs() < 0.001);
    assert!((axis.value_for_x(110.0) - 200.0).abs() < 0.001);
    assert!((axis.value_for_x(210.0) - 2_000.0).abs() < 0.01);
    assert!((axis.value_for_ratio(1.0 / 3.0) - 200.0).abs() < 0.001);
}

#[test]
fn horizontal_log_value_axis_clamps_values_and_coordinates() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 120.0));
    let axis = HorizontalLogValueAxis::new(rect, 20.0, 20_000.0);

    assert_eq!(axis.x_for_value(2.0), 10.0);
    assert_eq!(axis.x_for_value(200_000.0), 310.0);
    assert!((axis.value_for_x(-90.0) - 20.0).abs() < 0.001);
    assert!((axis.value_for_x(410.0) - 20_000.0).abs() < 0.01);
}

#[test]
fn horizontal_log_value_axis_supports_descending_ranges() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 120.0));
    let axis = HorizontalLogValueAxis::new(rect, 20_000.0, 20.0);

    assert_eq!(axis.x_for_value(20_000.0), 10.0);
    assert!((axis.x_for_value(2_000.0) - 110.0).abs() < 0.001);
    assert!((axis.x_for_value(200.0) - 210.0).abs() < 0.001);
    assert_eq!(axis.x_for_value(20.0), 310.0);
    assert!((axis.value_for_x(210.0) - 200.0).abs() < 0.001);
}

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
