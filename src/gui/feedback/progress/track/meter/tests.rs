use super::{horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect};
use crate::gui::types::{Point, Rect};

#[test]
fn horizontal_meter_fill_rect_clamps_level_and_minimum_width() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

    let minimum = horizontal_meter_fill_rect(track, 0.0, 1.0).expect("minimum meter");
    assert_eq!(minimum.min, track.min);
    assert_eq!(minimum.max, Point::new(11.0, 28.0));

    let overfilled = horizontal_meter_fill_rect(track, 2.0, 1.0).expect("overfilled meter");
    assert_eq!(overfilled, track);

    assert_eq!(horizontal_meter_fill_rect(track, 0.0, 0.0), None);
}

#[test]
fn horizontal_meter_fill_rect_sanitizes_nonfinite_inputs() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
    let invalid_track = Rect::from_min_max(Point::new(f32::NAN, 20.0), Point::new(110.0, 28.0));

    assert_eq!(horizontal_meter_fill_rect(invalid_track, 0.5, 1.0), None);
    assert_eq!(horizontal_meter_fill_rect(track, f32::NAN, 0.0), None);
    assert_eq!(
        horizontal_meter_fill_rect(track, f32::NAN, 1.0),
        Some(Rect::from_min_max(
            Point::new(10.0, 20.0),
            Point::new(11.0, 28.0)
        ))
    );
    assert_eq!(
        horizontal_meter_fill_rect(track, 0.5, f32::NAN)
            .unwrap()
            .max,
        Point::new(60.0, 28.0)
    );
}

#[test]
fn horizontal_discrete_meter_fill_rect_rounds_and_clamps_byte_levels() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

    assert_eq!(horizontal_discrete_meter_fill_rect(track, 0, 255), None);
    assert_eq!(horizontal_discrete_meter_fill_rect(track, 1, 255), None);

    let half = horizontal_discrete_meter_fill_rect(track, 128, 255).expect("half meter");
    assert_eq!(half.min, track.min);
    assert_eq!(half.max, Point::new(60.0, 28.0));

    let full = horizontal_discrete_meter_fill_rect(track, 999, 255).expect("full meter");
    assert_eq!(full, track);
}

#[test]
fn horizontal_discrete_meter_fill_rect_rejects_nonfinite_tracks() {
    let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::INFINITY, 28.0));

    assert_eq!(
        horizontal_discrete_meter_fill_rect(invalid_track, 128, 255),
        None
    );
}
