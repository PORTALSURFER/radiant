use super::{paths::collect_points_for_primitive, *};
use crate::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    runtime::{PaintPrimitive, PaintTextAlign},
    widgets::TextWrap,
};

struct TooShortPointIterator {
    upper_bound: usize,
}

impl Iterator for TooShortPointIterator {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        panic!("iterator should not be polled when upper bound is below primitive minimum");
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.upper_bound))
    }
}

#[test]
fn empty_rect_batches_do_not_emit_primitives() {
    let mut primitives = Vec::new();
    push_fill_rect_batch(&mut primitives, 7, Vec::new(), Rgba8::new(1, 2, 3, 4));
    push_stroke_rect_batch(&mut primitives, 7, Vec::new(), Rgba8::new(1, 2, 3, 4), 1.0);
    assert!(primitives.is_empty());
}

#[test]
fn push_visible_fill_rect_skips_empty_or_invalid_rects() {
    let mut primitives = Vec::new();
    let color = Rgba8::new(1, 2, 3, 4);

    assert!(!push_visible_fill_rect(
        &mut primitives,
        7,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 12.0)),
        color,
    ));
    assert!(!push_visible_fill_rect(
        &mut primitives,
        7,
        Rect::from_min_size(Point::new(f32::NAN, 0.0), Vector2::new(12.0, 12.0)),
        color,
    ));
    assert!(primitives.is_empty());
}

#[test]
fn push_visible_fill_rect_appends_positive_rects() {
    let mut primitives = Vec::new();
    let color = Rgba8::new(1, 2, 3, 4);
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 8.0));

    assert!(push_visible_fill_rect(&mut primitives, 7, rect, color));
    assert_eq!(primitives.len(), 1);
    assert!(matches!(
        &primitives[0],
        PaintPrimitive::FillRect(fill)
            if fill.widget_id == 7 && fill.rect == rect && fill.color == color
    ));
}

#[test]
fn widget_paint_binds_primitives_to_one_widget_id() {
    let mut primitives = Vec::new();
    let color = Rgba8::new(1, 2, 3, 4);
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 8.0));

    let mut paint = WidgetPaint::new(&mut primitives, 11);
    assert_eq!(paint.widget_id(), 11);
    assert!(paint.push_visible_fill_rect(rect, color));
    paint.push_stroke_rect(rect, color, 2.0);

    assert_eq!(primitives.len(), 2);
    assert!(matches!(
        &primitives[0],
        PaintPrimitive::FillRect(fill)
            if fill.widget_id == 11 && fill.rect == rect && fill.color == color
    ));
    assert!(matches!(
        &primitives[1],
        PaintPrimitive::StrokeRect(stroke)
            if stroke.widget_id == 11 && stroke.rect == rect && stroke.color == color
    ));
}

#[test]
fn polygon_and_polyline_helpers_skip_degenerate_point_lists() {
    let mut primitives = Vec::new();

    push_fill_polygon(
        &mut primitives,
        7,
        [Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        Rgba8::new(1, 2, 3, 4),
    );
    push_stroke_polyline(
        &mut primitives,
        7,
        [Point::new(0.0, 0.0)],
        Rgba8::new(1, 2, 3, 4),
        1.0,
    );

    assert!(primitives.is_empty());
}

#[test]
fn polygon_and_polyline_helpers_skip_known_short_iterators_without_polling() {
    let mut primitives = Vec::new();

    push_fill_polygon(
        &mut primitives,
        7,
        TooShortPointIterator { upper_bound: 2 },
        Rgba8::new(1, 2, 3, 4),
    );
    push_stroke_polyline(
        &mut primitives,
        7,
        TooShortPointIterator { upper_bound: 1 },
        Rgba8::new(1, 2, 3, 4),
        1.0,
    );

    assert!(primitives.is_empty());
}

#[test]
fn point_collection_presizes_from_iterator_lower_bound() {
    let points = collect_points_for_primitive(
        (0..128).map(|index| Point::new(index as f32, index as f32 * 0.5)),
        2,
    )
    .expect("valid point list");

    assert_eq!(points.len(), 128);
    assert!(points.capacity() >= 128);
}

#[test]
fn polygon_and_polyline_helpers_emit_shared_point_lists() {
    let mut primitives = Vec::new();

    push_fill_polygon(
        &mut primitives,
        7,
        [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        Rgba8::new(1, 2, 3, 4),
    );
    push_stroke_polyline(
        &mut primitives,
        9,
        [Point::new(1.0, 2.0), Point::new(3.0, 4.0)],
        Rgba8::new(5, 6, 7, 8),
        2.0,
    );

    let [
        PaintPrimitive::FillPolygon(fill),
        PaintPrimitive::StrokePolyline(stroke),
    ] = primitives.as_slice()
    else {
        panic!("expected polygon and polyline primitives");
    };
    assert_eq!(fill.widget_id, 7);
    assert_eq!(fill.points.len(), 3);
    assert_eq!(stroke.widget_id, 9);
    assert_eq!(stroke.points.len(), 2);
    assert_eq!(stroke.width, 2.0);
}

#[test]
fn text_helper_uses_default_single_line_metrics() {
    let mut primitives = Vec::new();
    push_text(
        &mut primitives,
        9,
        "Label",
        Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(30.0, 18.0)),
        Rgba8::new(4, 5, 6, 255),
        PaintTextAlign::Left,
    );

    let [PaintPrimitive::Text(text)] = primitives.as_slice() else {
        panic!("expected text primitive");
    };
    assert_eq!(text.widget_id, 9);
    assert_eq!(text.text.as_str(), "Label");
    assert_eq!(text.font_size, 12.0);
    assert_eq!(text.baseline, Some(16.0));
    assert_eq!(text.wrap, TextWrap::None);
}
