//! Generic virtual-list primitives.

mod controller;
mod geometry;
mod invalidation;
mod item;
mod scrollbar;
mod window;

pub use controller::{
    VirtualListController, VirtualListFocusTarget, VirtualListFollowState, VirtualListProjection,
    VirtualListSliceFocus,
};
pub use geometry::{
    VirtualListStackMetrics, VirtualListStackMetricsParts, virtual_list_stacked_item_at_point,
    virtual_list_viewport_len_for_extent,
};
pub use invalidation::VirtualListInvalidation;
pub use item::{
    MaterializedVirtualListItem, VirtualListItemKey, VirtualListItemOverlay, VirtualListItemState,
};
pub use scrollbar::{
    VirtualListScrollbar, VirtualListScrollbarRequest, resolve_virtual_list_scrollbar,
    virtual_list_scrollbar_thumb_offset_at_point, virtual_list_scrollbar_view_start_at_point,
    virtual_list_scrollbar_view_start_for_pointer,
};
pub use window::{
    VirtualListWindow, VirtualListWindowRequest, resolve_virtual_list_window,
    virtual_list_scroll_delta_from_units, virtual_list_view_start_after_scroll_delta,
    virtual_list_view_start_for_scroll_offset,
};
