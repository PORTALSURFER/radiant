//! Generic list and row state primitives.

mod editable;
mod geometry;
mod grid;
mod row_paint;
mod selection;
mod tree_guides;
mod virtual_list;

pub use editable::{
    ColumnSummary, ColumnSummaryParts, EditableRowKind, EditableTreeActions,
    EditableTreeDraftInputParts, EditableTreeInputFocus, EditableTreeRow, EditableTreeRowFlags,
    EditableTreeRowInput, EditableTreeRowParts,
};
pub use geometry::{
    bounded_list_height, bounded_list_height_with_gap, bounded_list_visible_rows,
    fixed_row_stack_height,
};
pub use grid::{VirtualGridWindow, VirtualGridWindowRequest, resolve_virtual_grid_window};
pub use row_paint::{
    DenseRowChromeParts, DenseRowLabelParts, DenseRowMarkerEdge, DenseRowMarkerParts,
    DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette, DenseRowVisualState,
    dense_row_fill_color, dense_row_inset_rect, dense_row_label_font_size,
    dense_row_vertical_marker_rect, push_dense_row_chrome, push_dense_row_fill,
    push_dense_row_inset_stroke, push_dense_row_label, push_dense_row_vertical_marker,
};
pub use selection::{
    CyclicListSelectionCycle, KeyedListSelection, ListSelectionController, ListSelectionIntent,
    ListSelectionModifiers, cyclic_list_index_after_delta, list_index_after_delta,
};
pub use tree_guides::{
    TreeGuideOverlay, TreeGuideRow, TreeGuideSegment, TreeGuideStyle, tree_guide_indent,
    tree_guide_overlay, tree_guide_segments,
};
pub use virtual_list::{
    MaterializedVirtualListItem, VirtualListController, VirtualListFocusTarget,
    VirtualListFollowState, VirtualListInvalidation, VirtualListItemKey, VirtualListItemOverlay,
    VirtualListItemState, VirtualListProjection, VirtualListScrollbar, VirtualListScrollbarRequest,
    VirtualListSliceFocus, VirtualListStackMetrics, VirtualListStackMetricsParts,
    VirtualListWindow, VirtualListWindowRequest, resolve_virtual_list_scrollbar,
    resolve_virtual_list_window, virtual_list_scroll_delta_from_units,
    virtual_list_scrollbar_thumb_offset_at_point, virtual_list_scrollbar_view_start_at_point,
    virtual_list_scrollbar_view_start_for_pointer, virtual_list_stacked_item_at_point,
    virtual_list_view_start_after_scroll_delta, virtual_list_view_start_for_scroll_offset,
    virtual_list_viewport_len_for_extent,
};

#[cfg(test)]
mod tests;
