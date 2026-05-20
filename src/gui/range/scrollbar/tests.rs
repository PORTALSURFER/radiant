use super::{
    NormalizedScrollbar, NormalizedScrollbarRequest, normalized_scrollbar_center_at_point,
    normalized_scrollbar_center_for_pointer, normalized_scrollbar_thumb_offset_at_point,
    normalized_scrollbar_thumb_ratio_at_point, resolve_normalized_scrollbar,
};
use crate::gui::types::{Point, Rect};

#[test]
fn normalized_scrollbar_maps_viewport_to_horizontal_thumb() {
    let track = Rect::from_min_max(Point::new(10.0, 40.0), Point::new(210.0, 44.0));
    let scrollbar = resolve_normalized_scrollbar(NormalizedScrollbarRequest {
        track,
        start_micros: 250_000,
        end_micros: 500_000,
        min_thumb_width: 28.0,
    })
    .expect("zoomed normalized viewport should show a scrollbar");

    assert_eq!(scrollbar.track, track);
    assert_eq!(scrollbar.thumb.width(), 50.0);
    assert_eq!(scrollbar.thumb.min.x, 60.0);
    assert_eq!(
        resolve_normalized_scrollbar(NormalizedScrollbarRequest {
            track,
            start_micros: 0,
            end_micros: 1_000_000,
            min_thumb_width: 28.0,
        }),
        None
    );
}

#[test]
fn normalized_scrollbar_resolves_thumb_pointer_state() {
    let scrollbar = NormalizedScrollbar {
        track: Rect::from_min_max(Point::new(10.0, 40.0), Point::new(210.0, 44.0)),
        thumb: Rect::from_min_max(Point::new(60.0, 40.0), Point::new(110.0, 44.0)),
    };

    assert_eq!(
        normalized_scrollbar_thumb_offset_at_point(scrollbar, Point::new(85.0, 42.0)),
        Some(25.0)
    );
    assert_eq!(
        normalized_scrollbar_thumb_ratio_at_point(scrollbar, Point::new(85.0, 42.0)),
        Some(0.5)
    );
    assert_eq!(
        normalized_scrollbar_thumb_offset_at_point(scrollbar, Point::new(85.0, 50.0)),
        None
    );
}

#[test]
fn normalized_scrollbar_resolves_drag_and_track_click_center() {
    let scrollbar = NormalizedScrollbar {
        track: Rect::from_min_max(Point::new(10.0, 40.0), Point::new(210.0, 44.0)),
        thumb: Rect::from_min_max(Point::new(60.0, 40.0), Point::new(110.0, 44.0)),
    };

    assert_eq!(
        normalized_scrollbar_center_for_pointer(scrollbar, 250_000, 500_000, 210.0, 0.0),
        Some(875_000)
    );
    assert_eq!(
        normalized_scrollbar_center_at_point(scrollbar, 250_000, 500_000, Point::new(185.0, 42.0)),
        Some(875_000)
    );
    assert_eq!(
        normalized_scrollbar_center_at_point(scrollbar, 250_000, 500_000, Point::new(85.0, 42.0)),
        None
    );
}
