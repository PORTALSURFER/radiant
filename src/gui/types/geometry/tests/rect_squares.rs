use super::super::{Point, Rect};

#[test]
fn rect_centered_square_clamps_side_and_centers() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 70.0));

    assert_eq!(
        rect.centered_square(80.0),
        Rect::from_min_max(Point::new(35.0, 20.0), Point::new(85.0, 70.0))
    );
}

#[test]
fn rect_square_around_centers_on_point() {
    assert_eq!(
        Rect::square_around(Point::new(20.0, 30.0), 8.0),
        Rect::from_min_max(Point::new(16.0, 26.0), Point::new(24.0, 34.0))
    );
    assert_eq!(
        Rect::square_around(Point::new(20.0, 30.0), 0.0),
        Rect::from_min_max(Point::new(20.0, 30.0), Point::new(20.0, 30.0))
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
fn rect_corner_squares_anchor_to_each_corner() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 70.0));

    assert_eq!(
        rect.top_left_square(12.0, 3.0),
        Rect::from_min_max(Point::new(13.0, 23.0), Point::new(25.0, 35.0))
    );
    assert_eq!(
        rect.top_right_square(12.0, 3.0),
        Rect::from_min_max(Point::new(35.0, 23.0), Point::new(47.0, 35.0))
    );
    assert_eq!(
        rect.bottom_left_square(12.0, 3.0),
        Rect::from_min_max(Point::new(13.0, 55.0), Point::new(25.0, 67.0))
    );
    assert_eq!(
        rect.bottom_right_square(12.0, 3.0),
        Rect::from_min_max(Point::new(35.0, 55.0), Point::new(47.0, 67.0))
    );
}

#[test]
fn rect_corner_squares_clamp_to_bounds() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(18.0, 26.0));

    assert_eq!(
        rect.bottom_right_square(20.0, 0.0),
        Rect::from_min_max(Point::new(12.0, 20.0), Point::new(18.0, 26.0))
    );
    assert_eq!(
        rect.bottom_right_square(20.0, 100.0),
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 20.0))
    );
    assert_eq!(
        rect.top_left_square(0.0, 2.0),
        Rect::from_min_max(Point::new(12.0, 22.0), Point::new(12.0, 22.0))
    );
}
