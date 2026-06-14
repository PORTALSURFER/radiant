use super::*;
use crate::gui::{
    types::Rgba8,
    visualization::{TimelineEditCurveStrokeParts, TimelineEditRamp, TimelineEditRampSide},
};

fn stroke_lengths(primitives: &[crate::runtime::PaintPrimitive]) -> Vec<usize> {
    primitives
        .iter()
        .filter_map(|primitive| primitive.stroke_polyline())
        .map(|stroke| stroke.points.len())
        .collect()
}

#[test]
fn timeline_edit_preview_pushes_standard_ramp_curve_strokes() {
    let mut primitives = Vec::new();

    assert!(
        preview().push_standard_ramp_curve_strokes(
            &mut primitives,
            TimelineEditCurveStrokeParts::new(11, mapper(), Rgba8::new(30, 60, 90, 220), 2.0)
                .step_bounds(4, 8),
            |side, fraction| match side {
                TimelineEditRampSide::Leading => Some(fraction),
                TimelineEditRampSide::Trailing => Some(1.0 - fraction),
            },
        )
    );

    assert_eq!(stroke_lengths(&primitives), vec![9, 9]);
    let first = primitives[0].stroke_polyline().expect("leading stroke");
    assert_eq!(first.widget_id, 11);
    assert_eq!(first.color, Rgba8::new(30, 60, 90, 220));
    assert_rect_near(
        Some(Rect::from_min_max(first.points[0], first.points[0])),
        Rect::from_min_max(Point::new(20.0, 72.0), Point::new(20.0, 72.0)),
    );
    let trailing = primitives[1].stroke_polyline().expect("trailing stroke");
    assert_rect_near(
        Some(Rect::from_min_max(trailing.points[0], trailing.points[0])),
        Rect::from_min_max(Point::new(100.0, 40.0), Point::new(100.0, 40.0)),
    );
}

#[test]
fn timeline_edit_preview_skips_ramp_curve_strokes_without_visible_selection() {
    let mut primitives = Vec::new();
    let preview = TimelineEditPreview::default();

    assert!(!preview.push_standard_ramp_curve_strokes(
        &mut primitives,
        TimelineEditCurveStrokeParts::new(11, mapper(), Rgba8::new(30, 60, 90, 220), 2.0),
        |_, _| Some(0.5),
    ));

    assert!(primitives.is_empty());
}

#[test]
fn timeline_edit_preview_builds_from_normalized_ramps() {
    let preview = TimelineEditPreview::from_normalized_ramps(
        NormalizedRange::from_micros(200_000, 600_000),
        Some(TimelineEditRamp::new(0.25, 0.5, Some(0.35))),
        Some(TimelineEditRamp::new(0.5, 0.25, Some(0.65))),
    );

    assert_eq!(
        preview.selection,
        Some(NormalizedRange::from_micros(200_000, 600_000))
    );
    assert_eq!(preview.leading_end_micros, Some(300_000));
    assert_eq!(preview.leading_inner_start_micros, Some(0));
    assert_eq!(preview.leading_curve_milli, Some(350));
    assert_eq!(preview.trailing_start_micros, Some(400_000));
    assert_eq!(preview.trailing_inner_end_micros, Some(700_000));
    assert_eq!(preview.trailing_curve_milli, Some(650));
}

#[test]
fn timeline_edit_ramp_from_length_has_no_outer_extension() {
    let preview = TimelineEditPreview::from_normalized_ramps(
        NormalizedRange::from_micros(200_000, 600_000),
        Some(TimelineEditRamp::from_length(0.25, None)),
        None,
    );

    assert_eq!(preview.leading_end_micros, Some(300_000));
    assert_eq!(preview.leading_inner_start_micros, Some(200_000));
    assert_eq!(preview.leading_curve_milli, None);
    assert_eq!(preview.trailing_start_micros, None);
}
