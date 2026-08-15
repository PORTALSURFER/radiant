use crate::gui::{
    feedback::RecoverySummary,
    list::{EditableTreeActions, EditableTreeRow},
    retained::RetainedVec,
};

mod assigned_row;
mod geometry;
mod sidebar_state;
mod slot;
mod tree_panel;

pub use assigned_row::{
    SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment, SplitPaneAssignmentState,
};
pub(crate) use geometry::sanitized_split_pane_ratio;
pub use geometry::{SplitPaneAxis, SplitPaneLayout, SplitPaneLayoutParts};
pub use sidebar_state::{
    SplitPaneSidebarChrome, SplitPaneSidebarContent, SplitPaneSidebarPanes,
    SplitPaneSidebarSelection, SplitPaneSidebarState, SplitPaneSidebarTreeControls,
};
pub use slot::SplitPaneSlot;
pub use tree_panel::{
    SplitPaneTreePanel, SplitPaneTreePanelActivity, SplitPaneTreePanelAssignment,
    SplitPaneTreePanelContent, SplitPaneTreePanelControls, SplitPaneTreePanelIdentity,
};
