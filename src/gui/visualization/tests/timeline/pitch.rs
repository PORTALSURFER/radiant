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

#[test]
fn timeline_pitch_item_layout_projects_note_rects() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 100.0));
    let axis = TimelineAxis::new(rect, 0.0, 10.0);
    let pitches = TimelinePitchLayout::new(rect, 60, 4);
    let items = TimelinePitchItemLayout::new(axis, pitches).with_vertical_inset(2.0);

    assert_eq!(
        items.item_rect(62, 2.0, 4.0),
        Rect::from_min_max(Point::new(50.0, 42.0), Point::new(90.0, 58.0))
    );
}

#[test]
fn timeline_pitch_item_layout_can_project_unclamped_note_rects() {
    let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 100.0));
    let axis = TimelineAxis::new(rect, 2.0, 6.0);
    let pitches = TimelinePitchLayout::new(rect, 60, 4);
    let items = TimelinePitchItemLayout::new(axis, pitches)
        .with_horizontal_inset(f32::NAN)
        .with_vertical_inset(f32::INFINITY);

    assert_eq!(
        items.item_rect_unclamped(61, 1.0, 7.0),
        Rect::from_min_max(Point::new(-40.0, 60.0), Point::new(260.0, 80.0))
    );
}
