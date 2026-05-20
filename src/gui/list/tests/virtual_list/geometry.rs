use super::super::super::{
    MaterializedVirtualListItem, VirtualListItemKey, VirtualListStackMetrics,
    VirtualListStackMetricsParts, virtual_list_scroll_delta_from_units,
    virtual_list_stacked_item_at_point, virtual_list_view_start_after_scroll_delta,
    virtual_list_viewport_len_for_extent,
};
use crate::gui::types::{Point, Rect};

#[test]
fn virtual_list_scroll_delta_clamps_to_visible_bounds() {
    assert_eq!(
        virtual_list_view_start_after_scroll_delta(10, 40, 12, -3),
        Some(7)
    );
    assert_eq!(
        virtual_list_view_start_after_scroll_delta(0, 40, 12, -3),
        Some(0)
    );
    assert_eq!(
        virtual_list_view_start_after_scroll_delta(27, 40, 12, 5),
        Some(28)
    );
    assert_eq!(
        virtual_list_view_start_after_scroll_delta(4, 0, 12, 2),
        None
    );
    assert_eq!(
        virtual_list_view_start_after_scroll_delta(4, 20, 0, 2),
        None
    );
    assert_eq!(
        virtual_list_view_start_after_scroll_delta(4, 20, 12, 0),
        None
    );
}

#[test]
fn virtual_list_scroll_delta_from_units_rounds_and_clamps_steps() {
    assert_eq!(virtual_list_scroll_delta_from_units(0.0), None);
    assert_eq!(virtual_list_scroll_delta_from_units(0.2), Some(1));
    assert_eq!(virtual_list_scroll_delta_from_units(-0.2), Some(-1));
    assert_eq!(virtual_list_scroll_delta_from_units(3.4), Some(3));
    assert_eq!(virtual_list_scroll_delta_from_units(-3.6), Some(-4));
    assert_eq!(virtual_list_scroll_delta_from_units(400.0), Some(i8::MAX));
    assert_eq!(virtual_list_scroll_delta_from_units(-400.0), Some(i8::MIN));
}

#[test]
fn virtual_list_viewport_len_uses_geometry_and_caps_capacity() {
    let metrics = VirtualListStackMetrics::new(24.0, 4.0).with_max_viewport_len(6);

    assert_eq!(virtual_list_viewport_len_for_extent(0.0, metrics), 1);
    assert_eq!(virtual_list_viewport_len_for_extent(139.0, metrics), 5);
    assert_eq!(virtual_list_viewport_len_for_extent(10_000.0, metrics), 6);
}

#[test]
fn virtual_list_stack_metrics_support_named_parts_construction() {
    let metrics = VirtualListStackMetrics::from_parts(VirtualListStackMetricsParts {
        item_extent: 0.0,
        item_gap: -4.0,
        max_viewport_len: Some(0),
    });

    assert_eq!(metrics.item_extent, 1.0);
    assert_eq!(metrics.item_gap, 0.0);
    assert_eq!(metrics.max_viewport_len, Some(1));
    assert_eq!(metrics.stride(), 1.0);
}

#[test]
fn virtual_list_hit_testing_returns_stable_logical_indices() {
    let items = [
        MaterializedVirtualListItem::new(
            VirtualListItemKey(41),
            10,
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0)),
        ),
        MaterializedVirtualListItem::new(
            VirtualListItemKey(42),
            11,
            Rect::from_min_max(Point::new(10.0, 44.0), Point::new(110.0, 64.0)),
        ),
        MaterializedVirtualListItem::new(
            VirtualListItemKey(43),
            12,
            Rect::from_min_max(Point::new(10.0, 68.0), Point::new(110.0, 88.0)),
        ),
    ];

    assert_eq!(
        virtual_list_stacked_item_at_point(&items, Point::new(20.0, 45.0)),
        Some(11)
    );
    assert_eq!(
        virtual_list_stacked_item_at_point(&items, Point::new(20.0, 42.0)),
        None
    );
    assert_eq!(
        virtual_list_stacked_item_at_point(&items, Point::new(120.0, 45.0)),
        None
    );
}
