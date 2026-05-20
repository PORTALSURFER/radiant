use super::super::super::{
    VirtualListScrollbar, VirtualListScrollbarRequest, resolve_virtual_list_scrollbar,
    virtual_list_scrollbar_thumb_offset_at_point, virtual_list_scrollbar_view_start_at_point,
    virtual_list_scrollbar_view_start_for_pointer,
};
use crate::gui::types::{Point, Rect};

#[test]
fn virtual_list_scrollbar_maps_viewport_and_pointer_drag() {
    let track = Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, 210.0));
    let scrollbar = resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
        track,
        total_items: 100,
        viewport_len: 20,
        viewport_start: 40,
        min_thumb_extent: 18.0,
    })
    .expect("overflowing list has scrollbar");

    assert_eq!(scrollbar.track, track);
    assert_eq!(scrollbar.thumb.height(), 40.0);
    assert_eq!(scrollbar.thumb.min.y, 90.0);
    assert_eq!(
        virtual_list_scrollbar_view_start_for_pointer(scrollbar, 20, 100, 170.0, 20.0),
        Some(70)
    );
    assert_eq!(
        resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
            track,
            total_items: 10,
            viewport_len: 10,
            viewport_start: 0,
            min_thumb_extent: 18.0,
        }),
        None
    );
}

#[test]
fn virtual_list_scrollbar_resolves_thumb_hit_offset_with_slop() {
    let scrollbar = VirtualListScrollbar {
        track: Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, 210.0)),
        thumb: Rect::from_min_max(Point::new(190.0, 90.0), Point::new(198.0, 130.0)),
    };

    assert_eq!(
        virtual_list_scrollbar_thumb_offset_at_point(scrollbar, Point::new(188.0, 100.0), 3.0),
        Some(10.0)
    );
    assert_eq!(
        virtual_list_scrollbar_thumb_offset_at_point(scrollbar, Point::new(196.0, 88.0), 3.0),
        Some(0.0)
    );
    assert_eq!(
        virtual_list_scrollbar_thumb_offset_at_point(scrollbar, Point::new(186.0, 100.0), 3.0),
        None
    );
}

#[test]
fn virtual_list_scrollbar_track_click_centers_thumb_on_pointer() {
    let scrollbar = VirtualListScrollbar {
        track: Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, 210.0)),
        thumb: Rect::from_min_max(Point::new(190.0, 90.0), Point::new(198.0, 130.0)),
    };

    assert_eq!(
        virtual_list_scrollbar_view_start_at_point(scrollbar, 20, 100, Point::new(194.0, 170.0)),
        Some(70)
    );
    assert_eq!(
        virtual_list_scrollbar_view_start_at_point(scrollbar, 20, 100, Point::new(194.0, 100.0)),
        None
    );
    assert_eq!(
        virtual_list_scrollbar_view_start_at_point(scrollbar, 20, 100, Point::new(184.0, 170.0)),
        None
    );
}

#[test]
fn virtual_list_scrollbar_saturates_oversized_minimum_thumb() {
    let track = Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, 26.0));
    let scrollbar = resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
        track,
        total_items: 100,
        viewport_len: 20,
        viewport_start: 40,
        min_thumb_extent: 48.0,
    })
    .expect("overflowing list with cramped track still has scrollbar geometry");

    assert_eq!(scrollbar.thumb, track);
    assert_eq!(
        virtual_list_scrollbar_view_start_for_pointer(scrollbar, 20, 100, 40.0, 0.0),
        Some(0)
    );
}

#[test]
fn virtual_list_scrollbar_rejects_nonfinite_track_geometry() {
    assert_eq!(
        resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
            track: Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, f32::NAN)),
            total_items: 100,
            viewport_len: 20,
            viewport_start: 40,
            min_thumb_extent: 18.0,
        }),
        None
    );
}

#[test]
fn virtual_list_scrollbar_drag_saturates_oversized_thumb_geometry() {
    let scrollbar = VirtualListScrollbar {
        track: Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, 26.0)),
        thumb: Rect::from_min_max(Point::new(190.0, 8.0), Point::new(198.0, 40.0)),
    };

    assert_eq!(
        virtual_list_scrollbar_view_start_for_pointer(scrollbar, 20, 100, 120.0, 0.0),
        Some(0)
    );
}
