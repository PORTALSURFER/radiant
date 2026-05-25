use super::super::{Point, Rect};

#[test]
fn rect_clamp_to_limits_rect_to_bounds() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    let rect = Rect::from_min_max(Point::new(0.0, 40.0), Point::new(50.0, 140.0));

    assert_eq!(
        rect.clamp_to(bounds),
        Rect::from_min_max(Point::new(10.0, 40.0), Point::new(50.0, 120.0))
    );
}

#[test]
fn rect_from_points_normalizes_opposite_corners() {
    assert_eq!(
        Rect::from_points(Point::new(90.0, 20.0), Point::new(10.0, 70.0)),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(90.0, 70.0))
    );
}

#[test]
fn rect_clamp_to_returns_empty_bounds_origin_for_disjoint_rect() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
    let rect = Rect::from_min_max(Point::new(200.0, 40.0), Point::new(250.0, 80.0));

    assert_eq!(rect.clamp_to(bounds), bounds.empty_at_min());
}

#[test]
fn rect_empty_at_max_returns_max_corner_empty_rect() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(
        rect.empty_at_max(),
        Rect::from_min_max(Point::new(50.0, 30.0), Point::new(50.0, 30.0))
    );
}

#[test]
fn rect_center_returns_midpoint() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(rect.center(), Point::new(30.0, 25.0));
}

#[test]
fn rect_ratio_projection_maps_between_axes_and_local_ratios() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 70.0));

    assert_eq!(rect.x_for_ratio(0.25), 35.0);
    assert_eq!(rect.y_for_ratio(0.40), 40.0);
    assert_eq!(rect.y_for_ratio_from_bottom(0.40), 50.0);
    assert_eq!(rect.x_for_ratio_unclamped(1.25), 135.0);
    assert_eq!(rect.y_for_ratio_unclamped(-0.20), 10.0);
    assert!((rect.y_for_ratio_from_bottom_unclamped(1.20) - 10.0).abs() < 0.00001);
    assert_eq!(rect.ratio_for_x(35.0), 0.25);
    assert_eq!(rect.ratio_for_y(40.0), 0.40);
    assert_eq!(rect.ratio_for_y_from_bottom(50.0), 0.40);
    assert_eq!(rect.x_for_ratio(-1.0), 10.0);
    assert_eq!(rect.y_for_ratio(2.0), 70.0);
    assert_eq!(rect.y_for_ratio_from_bottom(2.0), 20.0);
    assert_eq!(rect.ratio_for_x(1_000.0), 1.0);
    assert_eq!(rect.ratio_for_y(-10.0), 0.0);
    assert_eq!(rect.ratio_for_y_from_bottom(1_000.0), 0.0);
    assert_eq!(rect.ratio_for_y_from_bottom(-10.0), 1.0);
}

#[test]
fn rect_ratio_projection_returns_zero_for_invalid_or_empty_axes() {
    let empty_width = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 70.0));
    let empty_height = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 20.0));

    assert_eq!(empty_width.ratio_for_x(10.0), 0.0);
    assert_eq!(empty_height.ratio_for_y(20.0), 0.0);
    assert_eq!(empty_height.ratio_for_y_from_bottom(20.0), 0.0);
    assert_eq!(empty_width.ratio_for_x(f32::NAN), 0.0);
    assert_eq!(empty_height.ratio_for_y(f32::NAN), 0.0);
    assert_eq!(empty_height.ratio_for_y_from_bottom(f32::NAN), 0.0);
}

#[test]
fn rect_intersection_helpers_distinguish_edge_contact_from_positive_overlap() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 80.0));
    let touching = Rect::from_min_max(Point::new(50.0, 40.0), Point::new(80.0, 60.0));
    let overlapping = Rect::from_min_max(Point::new(49.0, 40.0), Point::new(80.0, 60.0));
    let disjoint = Rect::from_min_max(Point::new(51.0, 40.0), Point::new(80.0, 60.0));

    assert!(rect.intersects(touching));
    assert!(!rect.overlaps(touching));
    assert!(rect.intersects(overlapping));
    assert!(rect.overlaps(overlapping));
    assert!(!rect.intersects(disjoint));
    assert!(!rect.overlaps(disjoint));
}

#[test]
fn point_and_rect_finiteness_helpers_reject_invalid_geometry() {
    assert!(Point::new(1.0, 2.0).is_finite());
    assert!(!Point::new(f32::NAN, 2.0).is_finite());

    let valid = Rect::from_min_max(Point::new(1.0, 2.0), Point::new(3.0, 4.0));
    let empty = Rect::from_min_max(Point::new(1.0, 2.0), Point::new(1.0, 4.0));
    let invalid = Rect::from_min_max(Point::new(f32::NEG_INFINITY, 2.0), Point::new(3.0, 4.0));

    assert!(valid.is_finite());
    assert!(valid.has_finite_positive_area());
    assert!(empty.is_finite());
    assert!(!empty.has_finite_positive_area());
    assert!(!invalid.is_finite());
    assert!(!invalid.has_finite_positive_area());
}

#[test]
fn rect_union_covers_both_inputs() {
    let first = Rect::from_min_max(Point::new(10.0, 40.0), Point::new(90.0, 70.0));
    let second = Rect::from_min_max(Point::new(30.0, 20.0), Point::new(120.0, 60.0));

    assert_eq!(
        first.union(second),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(120.0, 70.0))
    );
}
