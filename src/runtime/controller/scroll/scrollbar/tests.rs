use super::*;

#[test]
fn scrollbar_hit_column_rejects_points_far_from_right_edge() {
    let viewport = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(200.0, 100.0));

    assert!(!scrollbar_hit_column_contains_point(
        viewport,
        Point::new(24.0, 40.0)
    ));
    assert!(scrollbar_hit_column_contains_point(
        viewport,
        Point::new(205.0, 40.0)
    ));
    assert!(!scrollbar_hit_column_contains_point(
        viewport,
        Point::new(205.0, 140.0)
    ));
}
