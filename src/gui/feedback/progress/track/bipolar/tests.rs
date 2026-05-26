use super::{vertical_bipolar_fill_rect, vertical_bipolar_value_at_point};
use crate::gui::types::{Point, Rect};

#[test]
fn vertical_bipolar_value_maps_track_edges_to_signed_range() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

    assert_eq!(
        vertical_bipolar_value_at_point(track, Point::new(50.0, 20.0)),
        1.0
    );
    assert_eq!(
        vertical_bipolar_value_at_point(track, Point::new(50.0, 70.0)),
        0.0
    );
    assert_eq!(
        vertical_bipolar_value_at_point(track, Point::new(50.0, 120.0)),
        -1.0
    );
}

#[test]
fn vertical_bipolar_fill_rect_projects_positive_and_negative_values() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

    assert_eq!(
        vertical_bipolar_fill_rect(track, 0.5, 12.0, 0.44),
        Some(Rect::from_min_max(
            Point::new(22.0, 48.0),
            Point::new(98.0, 70.0)
        ))
    );
    assert_eq!(
        vertical_bipolar_fill_rect(track, -0.5, 12.0, 0.44),
        Some(Rect::from_min_max(
            Point::new(22.0, 70.0),
            Point::new(98.0, 92.0)
        ))
    );
}

#[test]
fn vertical_bipolar_fill_rect_rejects_invalid_or_empty_output() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

    assert_eq!(vertical_bipolar_fill_rect(track, 0.0, 12.0, 0.44), None);
    assert_eq!(
        vertical_bipolar_fill_rect(track, f32::NAN, 12.0, 0.44),
        None
    );
    assert_eq!(vertical_bipolar_fill_rect(track, 1.0, 50.0, 0.44), None);
    assert_eq!(vertical_bipolar_fill_rect(track, 1.0, 12.0, 0.0), None);
    assert_eq!(
        vertical_bipolar_fill_rect(track.empty_at_min(), 1.0, 12.0, 0.44),
        None
    );
}
