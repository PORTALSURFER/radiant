//! Generic virtual-list primitives.
//!
//! Large-list contract:
//! - Hosts keep stable row identity and durable selection state.
//! - Radiant resolves a bounded materialized window from total item count,
//!   viewport length, overscan, scroll, and optional focus.
//! - Hosts construct widgets only for `window_start..window_end`; hidden rows
//!   are represented by logical counts and scroll geometry, not hidden widgets.
//! - Hit testing, overlay invalidation, repaint, and scroll state are scoped to
//!   one resolved list window so one large list does not force other lists to
//!   rebuild or repaint.
//! - Overscan is explicit and small enough for the interaction, rather than an
//!   accidental full-list materialization escape hatch.

mod controller;
mod focus;
mod geometry;
mod invalidation;
mod item;
mod projection;
mod scrollbar;
mod window;

pub use controller::VirtualListController;
pub use focus::{VirtualListFocusTarget, VirtualListFollowState, VirtualListSliceFocus};
pub use geometry::{
    VirtualListStackMetrics, VirtualListStackMetricsParts, virtual_list_stacked_item_at_point,
    virtual_list_viewport_len_for_extent,
};
pub use invalidation::VirtualListInvalidation;
pub use item::{
    MaterializedVirtualListItem, VirtualListItemKey, VirtualListItemOverlay, VirtualListItemState,
};
pub use projection::VirtualListProjection;
pub use scrollbar::{
    VirtualListScrollbar, VirtualListScrollbarRequest, resolve_virtual_list_scrollbar,
    virtual_list_scrollbar_thumb_offset_at_point, virtual_list_scrollbar_view_start_at_point,
    virtual_list_scrollbar_view_start_for_pointer,
};
pub use window::{
    VirtualListWindow, VirtualListWindowChange, VirtualListWindowRequest,
    resolve_virtual_list_window, virtual_list_scroll_delta_from_units,
    virtual_list_view_start_after_scroll_delta, virtual_list_view_start_for_scroll_offset,
};
