use super::*;

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
