//! Dense-list and virtualization prelude exports.

pub use crate::gui::{
    layout_core::{StackedLayoutCursor, StackedLayoutItem},
    list::{
        ColumnSummary, ColumnSummaryParts, CyclicListSelectionCycle, DenseRowMarkerParts,
        DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette, KeyedListSelection,
        ListSelectionController, ListSelectionIntent, ListSelectionModifiers, StyledTreeGuideStyle,
        TreeGuideMetrics, TreeGuideStyle, VirtualListController, VirtualListFocusTarget,
        VirtualListFollowState, VirtualListProjection, VirtualListSliceFocus,
        VirtualListStackMetrics, VirtualListStackMetricsParts, VirtualListWindow,
        VirtualListWindowChange, VirtualListWindowRequest, bounded_list_height,
        bounded_list_height_with_gap, bounded_list_visible_rows, cyclic_list_index_after_delta,
        fixed_row_stack_height, list_index_after_delta, resolve_virtual_list_window,
        unit_interval_index, virtual_list_view_start_for_scroll_offset,
    },
};
