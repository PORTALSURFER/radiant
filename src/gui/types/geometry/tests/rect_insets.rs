use super::super::{Point, Rect};

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
