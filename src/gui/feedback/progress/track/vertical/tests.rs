use super::{
    vertical_center_track_rect, vertical_meter_lane_fill_rect, vertical_value_at_point,
    vertical_value_knob_rect, vertical_value_line_rect,
};
use crate::gui::types::{Point, Rect};

#[test]
fn vertical_value_at_point_maps_bottom_to_top() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

    assert_eq!(vertical_value_at_point(track, Point::new(50.0, 120.0)), 0.0);
    assert_eq!(vertical_value_at_point(track, Point::new(50.0, 70.0)), 0.5);
    assert_eq!(vertical_value_at_point(track, Point::new(50.0, 20.0)), 1.0);
}

#[test]
fn vertical_center_track_rect_resolves_centered_rail() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

    assert_eq!(
        vertical_center_track_rect(track, 4.0),
        Some(Rect::from_min_max(
            Point::new(58.0, 20.0),
            Point::new(62.0, 120.0)
        ))
    );
    assert_eq!(vertical_center_track_rect(track, 0.0), None);
}

#[test]
fn vertical_value_knob_rect_centers_on_normalized_value() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));

    assert_eq!(
        vertical_value_knob_rect(track, 0.25, 16.0),
        Some(Rect::from_min_max(
            Point::new(10.0, 87.0),
            Point::new(110.0, 103.0)
        ))
    );
    assert_eq!(
        vertical_value_knob_rect(track, f32::NAN, 16.0),
        Some(Rect::from_min_max(
            Point::new(10.0, 112.0),
            Point::new(110.0, 128.0)
        ))
    );
    assert_eq!(vertical_value_knob_rect(track, 0.5, 0.0), None);
}

#[test]
fn vertical_meter_lane_fill_rect_projects_multilane_fill() {
    let meter = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 120.0));

    assert_eq!(
        vertical_meter_lane_fill_rect(meter, 1, 2, 0.5, 2.0, 3.0),
        Some(Rect::from_min_max(
            Point::new(31.0, 70.0),
            Point::new(47.0, 117.0)
        ))
    );
    assert_eq!(
        vertical_meter_lane_fill_rect(meter, 0, 2, 0.0, 2.0, 3.0),
        None
    );
    assert_eq!(
        vertical_meter_lane_fill_rect(meter, 0, 0, 0.5, 2.0, 3.0),
        None
    );
    assert_eq!(
        vertical_meter_lane_fill_rect(meter, 0, 40, 0.5, 2.0, 3.0),
        None
    );
}

#[test]
fn vertical_value_line_rect_resolves_inset_marker() {
    let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 120.0));

    assert_eq!(
        vertical_value_line_rect(track, 0.75, 2.0, 2.0),
        Some(Rect::from_min_max(
            Point::new(12.0, 45.0),
            Point::new(48.0, 47.0)
        ))
    );
    assert_eq!(vertical_value_line_rect(track, 0.5, 20.0, 2.0), None);
}
