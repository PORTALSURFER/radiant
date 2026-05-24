//! Generic panel and split-pane primitives.

mod anchored;
mod floating;
mod split_pane;

pub use anchored::{AnchoredPanelRectParts, anchored_panel_rect, anchored_panel_rect_from_parts};
pub use floating::{
    FloatingPanelDrag, FloatingPanelDragParts, FloatingPanelRectParts, floating_panel_rect,
    floating_panel_rect_from_parts,
};
pub use split_pane::{
    SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment, SplitPaneSidebarChrome,
    SplitPaneSidebarContent, SplitPaneSidebarPanes, SplitPaneSidebarSelection,
    SplitPaneSidebarState, SplitPaneSidebarTreeControls, SplitPaneSlot, SplitPaneTreePanel,
    SplitPaneTreePanelActivity, SplitPaneTreePanelAssignment, SplitPaneTreePanelContent,
    SplitPaneTreePanelControls, SplitPaneTreePanelIdentity,
};

#[cfg(test)]
mod tests;
