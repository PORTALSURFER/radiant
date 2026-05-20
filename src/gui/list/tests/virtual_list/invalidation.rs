use super::super::super::{
    MaterializedVirtualListItem, VirtualListInvalidation, VirtualListItemKey,
    VirtualListItemOverlay, VirtualListItemState,
};
use crate::gui::types::{Point, Rect};

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
