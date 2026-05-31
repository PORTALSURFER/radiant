use super::super::{Point, Rect};

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
fn rect_center_strips_resolve_horizontal_and_vertical_axes() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 70.0));

    assert_eq!(
        rect.horizontal_center_strip(12.0),
        Rect::from_min_max(Point::new(10.0, 39.0), Point::new(50.0, 51.0))
    );
    assert_eq!(
        rect.vertical_center_strip(8.0),
        Rect::from_min_max(Point::new(26.0, 20.0), Point::new(34.0, 70.0))
    );
}

#[test]
fn rect_center_strips_clamp_to_rect_dimensions() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(14.0, 23.0));

    assert_eq!(rect.horizontal_center_strip(99.0), rect);
    assert_eq!(rect.vertical_center_strip(99.0), rect);
    assert_eq!(
        rect.horizontal_center_strip(-1.0),
        Rect::from_min_max(Point::new(10.0, 21.5), Point::new(14.0, 21.5))
    );
}

#[test]
fn rect_coordinate_center_strips_shift_inside_bounds() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 70.0));

    assert_eq!(
        rect.vertical_strip_around_x(12.0, 10.0),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(20.0, 70.0))
    );
    assert_eq!(
        rect.horizontal_strip_around_y(68.0, 12.0),
        Rect::from_min_max(Point::new(10.0, 58.0), Point::new(50.0, 70.0))
    );
    assert_eq!(
        rect.vertical_strip_around_x(80.0, 0.0),
        Rect::from_min_max(Point::new(50.0, 20.0), Point::new(50.0, 70.0))
    );
}
