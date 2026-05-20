use super::*;
use crate::gui::types::{Point, Vector2};

#[test]
fn layout_axis_resolves_main_and_cross_extents() {
    let rect = Rect::from_min_size(Point::new(4.0, 8.0), Vector2::new(120.0, 48.0));

    assert_eq!(LayoutAxis::Horizontal.main_extent(rect), 120.0);
    assert_eq!(LayoutAxis::Horizontal.cross_extent(rect), 48.0);
    assert_eq!(LayoutAxis::Vertical.main_extent(rect), 48.0);
    assert_eq!(LayoutAxis::Vertical.cross_extent(rect), 120.0);
}

#[test]
fn layout_axis_reports_overflow_direction() {
    assert_eq!(LayoutAxis::Horizontal.overflow_flags(), (true, false));
    assert_eq!(LayoutAxis::Vertical.overflow_flags(), (false, true));
}
