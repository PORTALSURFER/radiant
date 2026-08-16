//! Generic panel and split-pane primitives.

mod anchored;
mod floating;
mod resize;
mod split_pane;

pub use anchored::{AnchoredPanelRectParts, anchored_panel_rect, anchored_panel_rect_from_parts};
pub use floating::{
    FloatingPanelDrag, FloatingPanelDragParts, FloatingPanelRectParts, floating_panel_rect,
    floating_panel_rect_from_parts,
};
pub use resize::{
    CollapsiblePanelResizeConstraints, PanelResizeConstraints, PanelResizeDrag, PanelResizeEdge,
    PanelResizeState, update_collapsible_panel_resize_drag, update_panel_resize_drag,
};
pub(crate) use split_pane::SplitPaneCollapseTarget;
pub use split_pane::{
    SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment, SplitPaneAssignmentState,
    SplitPaneAxis, SplitPaneCollapsePolicy, SplitPaneLayout, SplitPaneLayoutParts,
    SplitPaneSidebarChrome, SplitPaneSidebarContent, SplitPaneSidebarPanes,
    SplitPaneSidebarSelection, SplitPaneSidebarState, SplitPaneSidebarTreeControls, SplitPaneSlot,
    SplitPaneTreePanel, SplitPaneTreePanelActivity, SplitPaneTreePanelAssignment,
    SplitPaneTreePanelContent, SplitPaneTreePanelControls, SplitPaneTreePanelIdentity,
};
pub(crate) use split_pane::{
    quantized_split_pane_rects, sanitized_split_pane_ratio, split_pane_collapse_target,
};

#[cfg(test)]
mod tests;
