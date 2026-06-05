use super::{
    horizontal_progress_activity_rect, horizontal_progress_fill_rect,
    horizontal_progress_track_rect, horizontal_value_cursor_rect,
    horizontal_value_range_edge_rects, horizontal_value_range_rect,
    horizontal_wrapped_value_range_rects, push_horizontal_value_cursor_fill,
    push_horizontal_value_cursor_fills, push_horizontal_value_range_edge_fills,
    push_horizontal_value_range_fill,
};
use crate::gui::types::{Point, Rect};
use crate::{
    gui::types::Rgba8,
    runtime::{PaintPrimitive, WidgetPaint},
};

#[test]
fn horizontal_progress_fill_rect_clamps_to_track() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

    let overfilled = horizontal_progress_fill_rect(track, 1.5).expect("filled rect");
    assert_eq!(overfilled.min, track.min);
    assert_eq!(overfilled.max, track.max);

    let partial = horizontal_progress_fill_rect(track, 0.25).expect("partial rect");
    assert_eq!(partial.min, track.min);
    assert_eq!(partial.max, Point::new(35.0, 28.0));
}

#[test]
fn horizontal_progress_fill_rect_omits_empty_tracks_and_zero_fraction() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
    let empty_width = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 28.0));
    let empty_height = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 20.0));

    assert_eq!(horizontal_progress_fill_rect(track, 0.0), None);
    assert_eq!(horizontal_progress_fill_rect(track, -0.5), None);
    assert_eq!(horizontal_progress_fill_rect(empty_width, 0.5), None);
    assert_eq!(horizontal_progress_fill_rect(empty_height, 0.5), None);
}

#[test]
fn horizontal_progress_fill_rect_rejects_nonfinite_geometry_and_fraction() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
    let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::NAN, 28.0));

    assert_eq!(horizontal_progress_fill_rect(invalid_track, 0.5), None);
    assert_eq!(horizontal_progress_fill_rect(track, f32::NAN), None);
    assert_eq!(horizontal_progress_fill_rect(track, f32::INFINITY), None);
}

#[test]
fn horizontal_progress_activity_rect_resolves_moving_segment() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

    let start = horizontal_progress_activity_rect(track, 0.0, 0.24, 18.0).expect("start segment");
    assert_eq!(start.min, track.min);
    assert_eq!(start.max, Point::new(34.0, 28.0));

    let end = horizontal_progress_activity_rect(track, 1.0, 0.24, 18.0).expect("end segment");
    assert_eq!(end.min, Point::new(86.0, 20.0));
    assert_eq!(end.max, track.max);
}

#[test]
fn horizontal_progress_activity_rect_clamps_cramped_tracks() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(20.0, 28.0));

    let segment =
        horizontal_progress_activity_rect(track, 0.5, 0.24, 18.0).expect("cramped segment");
    assert_eq!(segment, track);

    assert_eq!(
        horizontal_progress_activity_rect(track, 0.5, 0.0, 0.0),
        None
    );
}

#[test]
fn horizontal_progress_activity_rect_sanitizes_nonfinite_inputs() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
    let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, f32::NAN));

    assert_eq!(
        horizontal_progress_activity_rect(invalid_track, 0.5, 0.24, 18.0),
        None
    );
    assert_eq!(
        horizontal_progress_activity_rect(track, f32::NAN, 0.24, 18.0),
        Some(Rect::from_min_max(
            Point::new(10.0, 20.0),
            Point::new(34.0, 28.0)
        ))
    );
    assert_eq!(
        horizontal_progress_activity_rect(track, 0.5, f32::NAN, 18.0),
        Some(Rect::from_min_max(
            Point::new(51.0, 20.0),
            Point::new(69.0, 28.0)
        ))
    );
    assert_eq!(
        horizontal_progress_activity_rect(track, 0.5, 0.24, f32::NAN),
        Some(Rect::from_min_max(
            Point::new(48.0, 20.0),
            Point::new(72.0, 28.0)
        ))
    );
}

#[test]
fn horizontal_progress_track_rect_switches_between_activity_and_fill() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

    let activity = horizontal_progress_track_rect(track, 0, 0, 0.5, 0.24, 18.0).expect("activity");
    assert_eq!(activity.min, Point::new(48.0, 20.0));
    assert_eq!(activity.max, Point::new(72.0, 28.0));

    let determinate = horizontal_progress_track_rect(track, 1, 4, 0.5, 0.24, 18.0).expect("fill");
    assert_eq!(determinate.min, track.min);
    assert_eq!(determinate.max, Point::new(35.0, 28.0));
}

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
