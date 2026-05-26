use super::super::super::*;
use crate::gui::types::{Point, Rect};

#[test]
fn timeline_pitch_layout_projects_top_down_pitch_rows() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 100.0));
    let layout = TimelinePitchLayout::new(rect, 60, 4);

    assert_eq!(layout.pitch_end(), 63);
    assert_eq!(layout.row_height(), 20.0);
    assert_eq!(
        layout.pitch_rect(62),
        Rect::from_min_max(Point::new(10.0, 40.0), Point::new(110.0, 60.0))
    );
    assert_eq!(layout.pitch_at(Point::new(20.0, 41.0)), Some(62));
}

#[test]
fn timeline_pitch_layout_clamps_pitch_hit_testing_to_visible_rows() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 90.0));
    let layout = TimelinePitchLayout::new(rect, 48, 3);

    assert_eq!(layout.pitch_at(Point::new(10.0, 0.0)), Some(50));
    assert_eq!(layout.pitch_at(Point::new(10.0, 89.0)), Some(48));
    assert_eq!(layout.pitch_at(Point::new(10.0, 120.0)), None);
    assert_eq!(
        TimelinePitchLayout::new(rect, 48, 0).pitch_at(Point::new(10.0, 10.0)),
        None
    );
}
