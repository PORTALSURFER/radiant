use super::*;

#[test]
fn horizontal_value_cursor_rect_centers_pixel_stable_full_height_strip() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));

    assert_eq!(
        horizontal_value_cursor_rect(track, 0.333, 2.0),
        Some(Rect::from_min_max(
            Point::new(42.0, 20.0),
            Point::new(44.0, 60.0)
        ))
    );
    assert_eq!(
        horizontal_value_cursor_rect(track, 0.0, 12.0),
        Some(Rect::from_min_max(
            Point::new(10.0, 20.0),
            Point::new(22.0, 60.0)
        ))
    );
    assert_eq!(
        horizontal_value_cursor_rect(track, 1.0, 12.0),
        Some(Rect::from_min_max(
            Point::new(98.0, 20.0),
            Point::new(110.0, 60.0)
        ))
    );
}

#[test]
fn push_horizontal_value_cursor_fill_appends_visible_cursor() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(5, 6, 7, 8);
    let mut primitives = Vec::new();

    assert!(push_horizontal_value_cursor_fill(
        &mut primitives,
        43,
        track,
        0.333,
        2.0,
        color,
    ));

    assert_eq!(primitives.len(), 1);
    match &primitives[0] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(fill.widget_id, 43);
            assert_eq!(
                fill.rect,
                Rect::from_min_max(Point::new(42.0, 20.0), Point::new(44.0, 60.0))
            );
            assert_eq!(fill.color, color);
        }
        primitive => panic!("expected fill rect, got {primitive:?}"),
    }
}

#[test]
fn widget_paint_pushes_horizontal_value_cursor_fill() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(5, 6, 7, 8);
    let mut primitives = Vec::new();
    let mut paint = WidgetPaint::new(&mut primitives, 47);

    assert!(paint.push_horizontal_value_cursor_fill(track, 0.333, 2.0, color));

    let [PaintPrimitive::FillRect(fill)] = primitives.as_slice() else {
        panic!("expected one fill rect");
    };
    assert_eq!(fill.widget_id, 47);
    assert_eq!(
        fill.rect,
        Rect::from_min_max(Point::new(42.0, 20.0), Point::new(44.0, 60.0))
    );
    assert_eq!(fill.color, color);
}

#[test]
fn push_horizontal_value_cursor_fills_appends_visible_cursors() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(5, 6, 7, 8);
    let mut primitives = Vec::new();

    assert_eq!(
        push_horizontal_value_cursor_fills(&mut primitives, 43, track, [0.25, 0.75], 2.0, color),
        2
    );

    let [
        PaintPrimitive::FillRect(first),
        PaintPrimitive::FillRect(second),
    ] = primitives.as_slice()
    else {
        panic!("expected two fill rects");
    };
    assert_eq!(first.widget_id, 43);
    assert_eq!(
        first.rect,
        Rect::from_min_max(Point::new(34.0, 20.0), Point::new(36.0, 60.0))
    );
    assert_eq!(first.color, color);
    assert_eq!(second.widget_id, 43);
    assert_eq!(
        second.rect,
        Rect::from_min_max(Point::new(84.0, 20.0), Point::new(86.0, 60.0))
    );
    assert_eq!(second.color, color);
}

#[test]
fn widget_paint_pushes_horizontal_value_cursor_fills() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(5, 6, 7, 8);
    let mut primitives = Vec::new();
    let mut paint = WidgetPaint::new(&mut primitives, 47);

    assert_eq!(
        paint.push_horizontal_value_cursor_fills(track, [0.25, 0.75], 2.0, color),
        2
    );

    assert_eq!(primitives.len(), 2);
    assert!(primitives.iter().all(|primitive| match primitive {
        PaintPrimitive::FillRect(fill) => fill.widget_id == 47 && fill.color == color,
        _ => false,
    }));
}

#[test]
fn push_horizontal_value_cursor_fill_skips_invalid_track() {
    let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::NAN, 60.0));
    let mut primitives = Vec::new();

    assert!(!push_horizontal_value_cursor_fill(
        &mut primitives,
        43,
        invalid_track,
        0.5,
        2.0,
        Rgba8::new(5, 6, 7, 8),
    ));

    assert!(primitives.is_empty());
}

#[test]
fn horizontal_value_cursor_rect_sanitizes_invalid_inputs() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::NAN, 60.0));

    assert_eq!(horizontal_value_cursor_rect(invalid_track, 0.5, 2.0), None);
    assert_eq!(
        horizontal_value_cursor_rect(track, f32::NAN, 2.0),
        Some(Rect::from_min_max(
            Point::new(10.0, 20.0),
            Point::new(12.0, 60.0)
        ))
    );
    assert_eq!(
        horizontal_value_cursor_rect(track, 0.5, f32::NAN),
        Some(Rect::from_min_max(
            Point::new(59.5, 20.0),
            Point::new(60.5, 60.0)
        ))
    );
}
