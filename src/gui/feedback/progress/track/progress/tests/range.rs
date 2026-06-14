use super::*;

#[test]
fn horizontal_value_range_rect_centers_segment_height() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));

    assert_eq!(
        horizontal_value_range_rect(track, 0.25, 0.75, 0.5),
        Some(Rect::from_min_max(
            Point::new(35.0, 30.0),
            Point::new(85.0, 50.0)
        ))
    );
    assert_eq!(horizontal_value_range_rect(track, 0.75, 0.25, 0.5), None);
    assert_eq!(horizontal_value_range_rect(track, 0.25, 0.75, 0.0), None);
}

#[test]
fn push_horizontal_value_range_fill_appends_visible_segment() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(1, 2, 3, 4);
    let mut primitives = Vec::new();

    assert!(push_horizontal_value_range_fill(
        &mut primitives,
        42,
        track,
        0.25,
        0.75,
        0.5,
        color,
    ));

    assert_eq!(primitives.len(), 1);
    match &primitives[0] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(fill.widget_id, 42);
            assert_eq!(
                fill.rect,
                Rect::from_min_max(Point::new(35.0, 30.0), Point::new(85.0, 50.0))
            );
            assert_eq!(fill.color, color);
        }
        primitive => panic!("expected fill rect, got {primitive:?}"),
    }
}

#[test]
fn widget_paint_pushes_horizontal_value_range_fill() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(1, 2, 3, 4);
    let mut primitives = Vec::new();
    let mut paint = WidgetPaint::new(&mut primitives, 45);

    assert!(paint.push_horizontal_value_range_fill(track, 0.25, 0.75, 0.5, color));

    let [PaintPrimitive::FillRect(fill)] = primitives.as_slice() else {
        panic!("expected one fill rect");
    };
    assert_eq!(fill.widget_id, 45);
    assert_eq!(
        fill.rect,
        Rect::from_min_max(Point::new(35.0, 30.0), Point::new(85.0, 50.0))
    );
    assert_eq!(fill.color, color);
}

#[test]
fn push_horizontal_value_range_fill_skips_empty_segment() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let mut primitives = Vec::new();

    assert!(!push_horizontal_value_range_fill(
        &mut primitives,
        42,
        track,
        0.75,
        0.25,
        0.5,
        Rgba8::new(1, 2, 3, 4),
    ));

    assert!(primitives.is_empty());
}

#[test]
fn horizontal_wrapped_value_range_rects_split_across_track_edges() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));

    assert_eq!(
        horizontal_wrapped_value_range_rects(track, 0.5, 0.20, 1.0),
        [
            Some(Rect::from_min_max(
                Point::new(50.0, 20.0),
                Point::new(70.0, 60.0)
            )),
            None
        ]
    );
    let wrapped = horizontal_wrapped_value_range_rects(track, 0.96, 0.12, 0.5);
    assert_eq!(
        wrapped[0],
        Some(Rect::from_min_max(
            Point::new(100.0, 30.0),
            Point::new(110.0, 50.0)
        ))
    );
    assert_rect_near(
        wrapped[1].expect("wrapped head segment"),
        Rect::from_min_max(Point::new(10.0, 30.0), Point::new(12.0, 50.0)),
    );
    assert_eq!(
        horizontal_wrapped_value_range_rects(track, f32::NAN, 0.10, 1.0),
        [
            Some(Rect::from_min_max(
                Point::new(105.0, 20.0),
                Point::new(110.0, 60.0)
            )),
            Some(Rect::from_min_max(
                Point::new(10.0, 20.0),
                Point::new(15.0, 60.0)
            ))
        ]
    );
    assert_eq!(
        horizontal_wrapped_value_range_rects(track, 0.5, 0.0, 1.0),
        [None, None]
    );
}

fn assert_rect_near(actual: Rect, expected: Rect) {
    assert!((actual.min.x - expected.min.x).abs() < 0.001);
    assert!((actual.min.y - expected.min.y).abs() < 0.001);
    assert!((actual.max.x - expected.max.x).abs() < 0.001);
    assert!((actual.max.y - expected.max.y).abs() < 0.001);
}
