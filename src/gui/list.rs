//! Generic list and row state primitives.

mod editable;
mod grid;
mod virtual_list;

pub use editable::{ColumnSummary, EditableRowKind, EditableTreeActions, EditableTreeRow};
pub use grid::{VirtualGridWindow, VirtualGridWindowRequest, resolve_virtual_grid_window};
pub use virtual_list::{
    MaterializedVirtualListItem, VirtualListInvalidation, VirtualListItemKey,
    VirtualListItemOverlay, VirtualListItemState, VirtualListScrollbar,
    VirtualListScrollbarRequest, VirtualListStackMetrics, VirtualListWindow,
    VirtualListWindowRequest, resolve_virtual_list_scrollbar, resolve_virtual_list_window,
    virtual_list_scroll_delta_from_units, virtual_list_scrollbar_view_start_for_pointer,
    virtual_list_stacked_item_at_point, virtual_list_view_start_after_scroll_delta,
    virtual_list_viewport_len_for_extent,
};

#[cfg(test)]
mod tests {
    use super::{
        ColumnSummary, EditableRowKind, EditableTreeActions, EditableTreeRow,
        MaterializedVirtualListItem, VirtualGridWindow, VirtualGridWindowRequest,
        VirtualListInvalidation, VirtualListItemKey, VirtualListItemOverlay, VirtualListItemState,
        VirtualListScrollbarRequest, VirtualListStackMetrics, VirtualListWindow,
        VirtualListWindowRequest, resolve_virtual_grid_window, resolve_virtual_list_scrollbar,
        resolve_virtual_list_window, virtual_list_scroll_delta_from_units,
        virtual_list_scrollbar_view_start_for_pointer, virtual_list_stacked_item_at_point,
        virtual_list_view_start_after_scroll_delta, virtual_list_viewport_len_for_extent,
    };
    use crate::gui::types::{Point, Rect};

    #[test]
    fn column_summary_preserves_title_and_count() {
        let column = ColumnSummary::new("Inbox", 42);

        assert_eq!(column.title, "Inbox");
        assert_eq!(column.item_count, 42);
    }

    #[test]
    fn editable_row_kind_defaults_to_existing() {
        assert_eq!(EditableRowKind::default(), EditableRowKind::Existing);
    }

    #[test]
    fn editable_tree_actions_default_to_unavailable() {
        let actions = EditableTreeActions::default();

        assert!(!actions.can_create_child);
        assert!(!actions.can_create_root);
        assert!(!actions.can_rename);
        assert!(!actions.can_delete);
        assert!(!actions.can_restore_retained);
        assert!(!actions.can_purge_retained);
        assert!(!actions.can_clear_history);
    }

    #[test]
    fn editable_tree_row_preserves_existing_and_draft_state() {
        let existing = EditableTreeRow::new("Root", "3 items", 0, true, false, true, true, true)
            .with_backing_index(7);
        let draft = EditableTreeRow::rename_draft(1, "Draft", "Name", None, true);

        assert_eq!(existing.label, "Root");
        assert_eq!(existing.detail, "3 items");
        assert_eq!(existing.kind, EditableRowKind::Existing);
        assert_eq!(existing.backing_index, Some(7));
        assert_eq!(draft.kind, EditableRowKind::RenameDraft);
        assert_eq!(draft.input_value.as_deref(), Some("Draft"));
        assert!(draft.input_focused);
        assert!(draft.select_all_on_focus);
    }

    #[test]
    fn virtual_list_window_clamps_requested_bounds_and_applies_overscan() {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 100,
            viewport_len: 10,
            requested_start: 95,
            overscan: 3,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(
            window,
            VirtualListWindow {
                total_items: 100,
                viewport_start: 90,
                viewport_end: 100,
                window_start: 87,
                window_end: 100,
            }
        );
        assert_eq!(window.viewport_len(), 10);
        assert_eq!(window.window_len(), 13);
        assert!(window.contains(99));
        assert!(!window.contains(86));
    }

    #[test]
    fn virtual_list_window_keeps_interior_focus_stable() {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(310),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(window.viewport_start, 300);
        assert_eq!(window.viewport_end, 320);
    }

    #[test]
    fn virtual_list_window_scrolls_when_focus_reaches_guard_band() {
        let top = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(302),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });
        let bottom = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(318),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(top.viewport_start, 298);
        assert_eq!(bottom.viewport_start, 303);
    }

    #[test]
    fn virtual_list_window_handles_empty_or_zero_viewport_requests() {
        assert!(resolve_virtual_list_window(VirtualListWindowRequest::default()).is_empty());
        assert!(
            resolve_virtual_list_window(VirtualListWindowRequest {
                total_items: 10,
                viewport_len: 0,
                ..VirtualListWindowRequest::default()
            })
            .is_empty()
        );
    }

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
    fn virtual_list_item_state_and_invalidation_are_overlay_oriented() {
        let idle = VirtualListItemState::default();
        let active = VirtualListItemState {
            selected: false,
            focused: true,
            hovered: false,
            active_target: false,
            disabled: false,
            overlay: VirtualListItemOverlay::Active,
        };
        let item = MaterializedVirtualListItem::new(
            VirtualListItemKey(9),
            3,
            Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 20.0)),
        )
        .with_state(active);
        let state_only = VirtualListInvalidation {
            item_state_changed: true,
            ..VirtualListInvalidation::default()
        };

        assert!(!idle.requires_overlay());
        assert!(item.state.requires_overlay());
        assert!(!state_only.requires_geometry_rebuild());
        assert!(state_only.requires_overlay_rebuild());
        assert!(
            VirtualListInvalidation {
                window_changed: true,
                ..VirtualListInvalidation::default()
            }
            .requires_geometry_rebuild()
        );
    }

    #[test]
    fn virtual_grid_window_clamps_rows_and_applies_overscan() {
        let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 103,
            columns: 5,
            viewport_rows: 4,
            requested_row: 99,
            overscan_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(
            window,
            VirtualGridWindow {
                total_items: 103,
                columns: 5,
                total_rows: 21,
                viewport_row_start: 17,
                viewport_row_end: 21,
                window_row_start: 15,
                window_row_end: 21,
                item_start: 75,
                item_end: 103,
            }
        );
        assert_eq!(window.viewport_row_len(), 4);
        assert_eq!(window.window_row_len(), 6);
        assert_eq!(window.item_len(), 28);
        assert!(window.contains_item(102));
        assert!(!window.contains_item(74));
    }

    #[test]
    fn virtual_grid_window_keeps_interior_focus_stable() {
        let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(178),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(window.viewport_row_start, 40);
        assert_eq!(window.viewport_row_end, 50);
    }

    #[test]
    fn virtual_grid_window_scrolls_when_focus_reaches_guard_row() {
        let top = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(164),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });
        let bottom = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(192),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(top.viewport_row_start, 39);
        assert_eq!(bottom.viewport_row_start, 41);
    }

    #[test]
    fn virtual_grid_window_handles_empty_zero_column_or_zero_viewport_requests() {
        assert!(resolve_virtual_grid_window(VirtualGridWindowRequest::default()).is_empty());
        assert!(
            resolve_virtual_grid_window(VirtualGridWindowRequest {
                total_items: 10,
                columns: 0,
                viewport_rows: 2,
                ..VirtualGridWindowRequest::default()
            })
            .is_empty()
        );
        assert!(
            resolve_virtual_grid_window(VirtualGridWindowRequest {
                total_items: 10,
                columns: 3,
                viewport_rows: 0,
                ..VirtualGridWindowRequest::default()
            })
            .is_empty()
        );
    }
}
