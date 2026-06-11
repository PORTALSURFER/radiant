use super::super::super::{
    MaterializedVirtualListItem, VirtualListController, VirtualListInvalidation,
    VirtualListItemKey, VirtualListWindowRequest, resolve_virtual_list_window,
    virtual_list_stacked_item_at_point,
};
use crate::gui::types::{Point, Rect};

#[test]
fn virtual_list_materializes_only_viewport_plus_explicit_overscan() {
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 1_000_000,
        viewport_len: 24,
        requested_start: 42_000,
        overscan: 3,
        focused_index: None,
        previous_start: None,
        guard_band: 0,
    });

    assert_eq!(window.viewport_start, 42_000);
    assert_eq!(window.viewport_len(), 24);
    assert_eq!(window.window_start, 41_997);
    assert_eq!(window.window_end, 42_027);
    assert_eq!(window.window_len(), 30);
    assert!(window.window_len() <= window.viewport_len() + 6);
}

#[test]
fn virtual_list_hit_testing_is_limited_to_materialized_rows() {
    let rows = materialized_rows(500, 4);

    assert_eq!(
        virtual_list_stacked_item_at_point(&rows, Point::new(8.0, 24.0)),
        Some(501)
    );
    assert_eq!(
        virtual_list_stacked_item_at_point(&rows, Point::new(8.0, 121.0)),
        None
    );
}

#[test]
fn virtual_list_scroll_state_isolated_by_controller_instance() {
    let mut left = VirtualListController::with_items(10_000, 20);
    let mut right = VirtualListController::with_items(10_000, 20);
    left.set_overscan(2);
    right.set_overscan(2);

    let left_window = left.scroll_rows(400).expect("left scrolls");
    let right_window = right.resolve();

    assert_eq!(left_window.viewport_start, 400);
    assert_eq!(right_window.viewport_start, 0);
    assert_eq!(left.viewport_start(), 400);
    assert_eq!(right.viewport_start(), 0);
}

#[test]
fn virtual_list_item_state_changes_are_overlay_only() {
    let state_only = VirtualListInvalidation {
        item_state_changed: true,
        ..VirtualListInvalidation::default()
    };

    assert!(!state_only.requires_geometry_rebuild());
    assert!(state_only.requires_overlay_rebuild());
}

fn materialized_rows(first_index: usize, count: usize) -> Vec<MaterializedVirtualListItem> {
    (0..count)
        .map(|offset| {
            let index = first_index + offset;
            let y = offset as f32 * 24.0;
            MaterializedVirtualListItem::new(
                VirtualListItemKey(index as u64),
                index,
                Rect::from_min_max(Point::new(0.0, y), Point::new(100.0, y + 20.0)),
            )
        })
        .collect()
}
