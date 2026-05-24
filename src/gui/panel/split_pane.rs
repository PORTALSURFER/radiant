use crate::gui::{
    feedback::RecoverySummary,
    list::{EditableTreeActions, EditableTreeRow},
    retained::RetainedVec,
};

mod assigned_row;
mod sidebar_state;
mod slot;
mod tree_panel;

pub use assigned_row::{SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment};
pub use sidebar_state::{
    SplitPaneSidebarChrome, SplitPaneSidebarContent, SplitPaneSidebarPanes,
    SplitPaneSidebarSelection, SplitPaneSidebarState, SplitPaneSidebarTreeControls,
};
pub use slot::SplitPaneSlot;
pub use tree_panel::{
    SplitPaneTreePanel, SplitPaneTreePanelActivity, SplitPaneTreePanelAssignment,
    SplitPaneTreePanelContent, SplitPaneTreePanelControls, SplitPaneTreePanelIdentity,
};
