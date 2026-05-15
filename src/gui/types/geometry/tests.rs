use super::{Point, Rect};

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
fn rect_inset_horizontal_clamps_to_rect_edges() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(
        rect.inset_horizontal(4.0, 6.0),
        Rect::from_min_max(Point::new(14.0, 20.0), Point::new(44.0, 30.0))
    );
    assert_eq!(
        rect.inset_horizontal(80.0, 6.0),
        Rect::from_min_max(Point::new(50.0, 20.0), Point::new(50.0, 30.0))
    );
}

#[test]
fn rect_inset_vertical_clamps_to_rect_edges() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(
        rect.inset_vertical(2.0, 3.0),
        Rect::from_min_max(Point::new(10.0, 22.0), Point::new(50.0, 27.0))
    );
    assert_eq!(
        rect.inset_vertical(80.0, 3.0),
        Rect::from_min_max(Point::new(10.0, 30.0), Point::new(50.0, 30.0))
    );
}

#[test]
fn rect_split_at_y_clamps_split_line_to_rect_bounds() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(
        rect.split_at_y(24.0),
        (
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 24.0)),
            Rect::from_min_max(Point::new(10.0, 24.0), Point::new(50.0, 30.0))
        )
    );
    assert_eq!(
        rect.split_at_y(80.0),
        (
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0)),
            Rect::from_min_max(Point::new(10.0, 30.0), Point::new(50.0, 30.0))
        )
    );
}

#[test]
fn rect_inset_horizontal_saturating_caps_at_half_width() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(
        rect.inset_horizontal_saturating(6.0),
        Rect::from_min_max(Point::new(16.0, 20.0), Point::new(44.0, 30.0))
    );
    assert_eq!(
        rect.inset_horizontal_saturating(80.0),
        Rect::from_min_max(Point::new(30.0, 20.0), Point::new(30.0, 30.0))
    );
}

#[test]
fn rect_inset_uniform_saturating_caps_at_half_extents() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 30.0));

    assert_eq!(
        rect.inset_uniform_saturating(4.0),
        Rect::from_min_max(Point::new(14.0, 24.0), Point::new(46.0, 26.0))
    );
    assert_eq!(
        rect.inset_uniform_saturating(80.0),
        Rect::from_min_max(Point::new(30.0, 25.0), Point::new(30.0, 25.0))
    );
}

#[test]
fn rect_centered_square_clamps_side_and_centers() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 70.0));

    assert_eq!(
        rect.centered_square(80.0),
        Rect::from_min_max(Point::new(35.0, 20.0), Point::new(85.0, 70.0))
    );
}

#[test]
fn rect_centered_pixel_square_clamps_and_snaps_origin() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(49.0, 61.0));

    assert_eq!(
        rect.centered_pixel_square(14.7, 8.0, 20.0),
        Some(Rect::from_min_max(
            Point::new(22.0, 33.0),
            Point::new(36.0, 47.0)
        ))
    );
    assert_eq!(rect.centered_pixel_square(0.0, 0.0, 20.0), None);
}

#[test]
fn rect_centered_pixel_square_normalizes_side_range_inputs() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(49.0, 61.0));

    assert_eq!(
        rect.centered_pixel_square(14.7, 20.0, 8.0),
        Some(Rect::from_min_max(
            Point::new(22.0, 33.0),
            Point::new(36.0, 47.0)
        ))
    );
    assert_eq!(rect.centered_pixel_square(f32::NAN, 8.0, 20.0), None);
    assert_eq!(rect.centered_pixel_square(14.7, f32::NAN, 20.0), None);
    assert_eq!(rect.centered_pixel_square(14.7, 8.0, f32::NAN), None);
}

#[test]
fn rect_centered_odd_pixel_square_forces_odd_side() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(22.0, 32.0));

    assert_eq!(
        rect.centered_odd_pixel_square(5.0, 9.0),
        Some(Rect::from_min_max(
            Point::new(11.0, 21.0),
            Point::new(20.0, 30.0)
        ))
    );
    assert_eq!(
        Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1.0, 10.0))
            .centered_odd_pixel_square(5.0, 9.0),
        None
    );
}

#[test]
fn rect_centered_odd_pixel_square_normalizes_side_range_inputs() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(22.0, 32.0));

    assert_eq!(
        rect.centered_odd_pixel_square(9.0, 5.0),
        Some(Rect::from_min_max(
            Point::new(11.0, 21.0),
            Point::new(20.0, 30.0)
        ))
    );
    assert_eq!(rect.centered_odd_pixel_square(f32::NAN, 9.0), None);
    assert_eq!(rect.centered_odd_pixel_square(5.0, f32::NAN), None);
}

#[test]
fn rect_stroke_aligned_rect_snaps_to_stroke_grid() {
    let rect = Rect::from_min_max(Point::new(10.4, 20.6), Point::new(111.2, 119.1));

    assert_eq!(
        rect.stroke_aligned_rect(2.0),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(112.0, 120.0))
    );
}

#[test]
fn rect_stroke_aligned_rect_keeps_tiny_rects() {
    let rect = Rect::from_min_max(Point::new(10.4, 20.6), Point::new(12.1, 22.2));

    assert_eq!(rect.stroke_aligned_rect(0.25), rect);
}

#[test]
fn rect_top_right_square_places_square_inside_anchor() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 70.0));

    assert_eq!(
        rect.top_right_square(12.0, 3.0),
        Rect::from_min_max(Point::new(35.0, 23.0), Point::new(47.0, 35.0))
    );
}

#[test]
fn rect_top_right_square_clamps_to_bounds() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(18.0, 26.0));

    assert_eq!(
        rect.top_right_square(20.0, 1.0),
        Rect::from_min_max(Point::new(11.0, 21.0), Point::new(17.0, 26.0))
    );
    assert_eq!(
        rect.top_right_square(0.0, 1.0),
        Rect::from_min_max(Point::new(17.0, 21.0), Point::new(17.0, 21.0))
    );
}

#[test]
fn rect_edge_strips_resolve_each_side() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 70.0));

    assert_eq!(
        rect.top_edge_strip(3.0),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 23.0))
    );
    assert_eq!(
        rect.bottom_edge_strip(4.0),
        Rect::from_min_max(Point::new(10.0, 66.0), Point::new(50.0, 70.0))
    );
    assert_eq!(
        rect.left_edge_strip(5.0),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(15.0, 70.0))
    );
    assert_eq!(
        rect.right_edge_strip(6.0),
        Rect::from_min_max(Point::new(44.0, 20.0), Point::new(50.0, 70.0))
    );
}

#[test]
fn rect_edge_strips_clamp_to_rect_dimensions() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(14.0, 23.0));

    assert_eq!(rect.top_edge_strip(99.0), rect);
    assert_eq!(rect.right_edge_strip(99.0), rect);
    assert_eq!(
        rect.left_edge_strip(-1.0),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 23.0))
    );
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
