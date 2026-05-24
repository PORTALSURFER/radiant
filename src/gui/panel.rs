//! Generic panel and split-pane primitives.

mod anchored;
mod floating;
mod split_pane;

pub use anchored::anchored_panel_rect;
pub use floating::{FloatingPanelDrag, FloatingPanelDragParts, floating_panel_rect};
pub use split_pane::{
    SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment, SplitPaneSidebarChrome,
    SplitPaneSidebarContent, SplitPaneSidebarPanes, SplitPaneSidebarSelection,
    SplitPaneSidebarState, SplitPaneSidebarTreeControls, SplitPaneSlot, SplitPaneTreePanel,
    SplitPaneTreePanelActivity, SplitPaneTreePanelAssignment, SplitPaneTreePanelContent,
    SplitPaneTreePanelControls, SplitPaneTreePanelIdentity,
};

#[cfg(test)]
mod tests;
