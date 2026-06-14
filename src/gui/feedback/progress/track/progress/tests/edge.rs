use super::*;

#[test]
fn horizontal_value_range_edge_rects_returns_top_and_bottom_strips() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));

    assert_eq!(
        horizontal_value_range_edge_rects(track, 0.25, 0.75, 3.0),
        [
            Some(Rect::from_min_max(
                Point::new(35.0, 20.0),
                Point::new(85.0, 23.0)
            )),
            Some(Rect::from_min_max(
                Point::new(35.0, 57.0),
                Point::new(85.0, 60.0)
            ))
        ]
    );
}

#[test]
fn horizontal_value_range_edge_rects_clamps_height_and_skips_empty_ranges() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 24.0));

    assert_eq!(
        horizontal_value_range_edge_rects(track, 0.25, 0.75, 12.0),
        [
            Some(Rect::from_min_max(
                Point::new(35.0, 20.0),
                Point::new(85.0, 24.0)
            )),
            Some(Rect::from_min_max(
                Point::new(35.0, 20.0),
                Point::new(85.0, 24.0)
            ))
        ]
    );
    assert_eq!(
        horizontal_value_range_edge_rects(track, 0.75, 0.25, 3.0),
        [None, None]
    );
}

#[test]
fn push_horizontal_value_range_edge_fills_appends_edge_strips() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(9, 10, 11, 12);
    let mut primitives = Vec::new();

    assert_eq!(
        push_horizontal_value_range_edge_fills(&mut primitives, 44, track, 0.25, 0.75, 3.0, color,),
        2
    );

    assert_eq!(primitives.len(), 2);
    match primitives.as_slice() {
        [
            PaintPrimitive::FillRect(top),
            PaintPrimitive::FillRect(bottom),
        ] => {
            assert_eq!(top.widget_id, 44);
            assert_eq!(
                top.rect,
                Rect::from_min_max(Point::new(35.0, 20.0), Point::new(85.0, 23.0))
            );
            assert_eq!(bottom.widget_id, 44);
            assert_eq!(
                bottom.rect,
                Rect::from_min_max(Point::new(35.0, 57.0), Point::new(85.0, 60.0))
            );
            assert_eq!(top.color, color);
            assert_eq!(bottom.color, color);
        }
        primitives => panic!("expected two fill rects, got {primitives:?}"),
    }
}

#[test]
fn widget_paint_pushes_horizontal_value_range_edge_fills() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 60.0));
    let color = Rgba8::new(9, 10, 11, 12);
    let mut primitives = Vec::new();
    let mut paint = WidgetPaint::new(&mut primitives, 46);

    assert_eq!(
        paint.push_horizontal_value_range_edge_fills(track, 0.25, 0.75, 3.0, color),
        2
    );

    let [
        PaintPrimitive::FillRect(top),
        PaintPrimitive::FillRect(bottom),
    ] = primitives.as_slice()
    else {
        panic!("expected two fill rects");
    };
    assert_eq!(top.widget_id, 46);
    assert_eq!(bottom.widget_id, 46);
    assert_eq!(top.color, color);
    assert_eq!(bottom.color, color);
}
