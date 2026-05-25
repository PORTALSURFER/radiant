use super::*;

#[test]
fn fixed_width_row_rects_start_places_items_from_left_edge() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
    let rects = fixed_width_row_rects_start(bounds, 4.0, &[20.0, 30.0], 1, 10);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x, 10.0);
    assert_eq!(rects[0].max.x, 30.0);
    assert_eq!(rects[1].min.x, 34.0);
    assert_eq!(rects[1].max.x, 64.0);
}

#[test]
fn fixed_width_row_rects_end_places_items_against_right_edge() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
    let rects = fixed_width_row_rects_end(bounds, 4.0, &[20.0, 30.0], 1, 2, 10);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x, 56.0);
    assert_eq!(rects[0].max.x, 76.0);
    assert_eq!(rects[1].min.x, 80.0);
    assert_eq!(rects[1].max.x, 110.0);
}

#[test]
fn fixed_width_row_rects_end_overflow_starts_at_left_edge() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(60.0, 40.0));
    let rects = fixed_width_row_rects_end(bounds, 4.0, &[40.0, 40.0], 1, 2, 10);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x, 10.0);
    assert_eq!(rects[0].max.x, 50.0);
    assert_eq!(rects[1].min.x, 54.0);
    assert_eq!(rects[1].max.x, 60.0);
}

#[test]
fn fixed_width_row_rects_presizes_output() {
    let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 20.0));
    let rects = fixed_width_row_rects_start(bounds, 2.0, &[10.0, 20.0, 30.0], 1, 10);

    assert_eq!(rects.len(), 3);
    assert!(rects.capacity() >= 3);
}

#[test]
fn fixed_width_row_rects_into_reuses_output_storage() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
    let mut rects = Vec::with_capacity(8);
    rects.push(Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
    ));
    let capacity = rects.capacity();

    fixed_width_row_rects_start_into(bounds, 4.0, &[20.0, 30.0], 1, 10, &mut rects);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects.capacity(), capacity);
    assert_eq!(rects[0].min.x, 10.0);
    assert_eq!(rects[1].max.x, 64.0);

    fixed_width_row_rects_end_into(bounds, 4.0, &[20.0, 30.0], 1, 2, 10, &mut rects);

    assert_eq!(rects.capacity(), capacity);
    assert_eq!(rects[0].min.x, 56.0);
    assert_eq!(rects[1].max.x, 110.0);
}
