use super::{
    EditableTreeActions, EditableTreeRow, RecoverySummary, RetainedVec,
    assigned_row::SplitPaneAssignedRow, slot::SplitPaneSlot, tree_panel::SplitPaneTreePanel,
};

/// Header and search text for a split-pane sidebar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SplitPaneSidebarChrome {
    /// Header text for the sidebar.
    pub header: String,
    /// Active search query for the assignable row list.
    pub search_query: String,
}

/// Pair of split-pane tree surfaces plus the active pane selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPaneSidebarPanes<TreeRow = EditableTreeRow> {
    /// Pane that currently drives the related content surface.
    pub active_pane: SplitPaneSlot,
    /// Upper or leading tree pane.
    pub upper_pane: SplitPaneTreePanel<TreeRow>,
    /// Lower or trailing tree pane.
    pub lower_pane: SplitPaneTreePanel<TreeRow>,
}

impl<TreeRow> Default for SplitPaneSidebarPanes<TreeRow> {
    fn default() -> Self {
        Self {
            active_pane: SplitPaneSlot::default(),
            upper_pane: SplitPaneTreePanel::default(),
            lower_pane: SplitPaneTreePanel::default(),
        }
    }
}

/// Tree filtering controls shared by split-pane sidebar controls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SplitPaneSidebarTreeControls {
    /// Active tree-search query shared by pane controls.
    pub tree_search_query: String,
    /// Whether the tree currently includes otherwise hidden empty items.
    pub show_all_items: bool,
    /// Whether the hidden-item visibility toggle is currently actionable.
    pub can_toggle_show_all_items: bool,
    /// Whether tree filtering includes descendant items in a flattened list.
    pub flattened_view: bool,
    /// Whether the flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
}

/// Selection and busy-row state for split-pane sidebar lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitPaneSidebarSelection {
    /// Selected assignable-row index, if any.
    pub selected_row: Option<usize>,
    /// Assignable row currently hydrating in the background, if any.
    pub loading_row: Option<usize>,
    /// Assignable row currently running a background mutation, if any.
    pub mutation_busy_row: Option<usize>,
    /// Focused tree row index, if any.
    pub focused_tree_row: Option<usize>,
}

/// Assignable rows and active tree content projected into the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPaneSidebarContent<Row = SplitPaneAssignedRow, TreeRow = EditableTreeRow> {
    /// Assignable rows to render in the sidebar.
    pub rows: RetainedVec<Row>,
    /// Tree rows to render in the active tree surface.
    pub tree_rows: RetainedVec<TreeRow>,
    /// Tree action availability for controls.
    pub tree_actions: EditableTreeActions,
    /// Delete/recovery summary for sidebar status.
    pub recovery: RecoverySummary,
}

impl<Row, TreeRow> Default for SplitPaneSidebarContent<Row, TreeRow> {
    fn default() -> Self {
        Self {
            rows: RetainedVec::new(),
            tree_rows: RetainedVec::new(),
            tree_actions: EditableTreeActions::default(),
            recovery: RecoverySummary::default(),
        }
    }
}

/// Generic sidebar state built around an assignable row list plus a two-pane tree surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPaneSidebarState<Row = SplitPaneAssignedRow, TreeRow = EditableTreeRow> {
    /// Header and row-search state.
    pub chrome: SplitPaneSidebarChrome,
    /// Two-pane tree surface models.
    pub panes: SplitPaneSidebarPanes<TreeRow>,
    /// Shared tree filtering controls.
    pub tree_controls: SplitPaneSidebarTreeControls,
    /// Selected and busy row indexes.
    pub selection: SplitPaneSidebarSelection,
    /// Assignable rows and active tree content.
    pub content: SplitPaneSidebarContent<Row, TreeRow>,
}

impl<Row, TreeRow> Default for SplitPaneSidebarState<Row, TreeRow> {
    fn default() -> Self {
        Self {
            chrome: SplitPaneSidebarChrome::default(),
            panes: SplitPaneSidebarPanes::default(),
            tree_controls: SplitPaneSidebarTreeControls::default(),
            selection: SplitPaneSidebarSelection::default(),
            content: SplitPaneSidebarContent::default(),
        }
    }
}

impl<Row, TreeRow> SplitPaneSidebarState<Row, TreeRow> {
    /// Borrow one pane by id.
    pub fn pane(&self, pane: SplitPaneSlot) -> &SplitPaneTreePanel<TreeRow> {
        pane.select(&self.panes.upper_pane, &self.panes.lower_pane)
    }

    /// Mutably borrow one pane by id.
    pub fn pane_mut(&mut self, pane: SplitPaneSlot) -> &mut SplitPaneTreePanel<TreeRow> {
        pane.select_mut(&mut self.panes.upper_pane, &mut self.panes.lower_pane)
    }

    /// Borrow the pane that currently drives the related content surface.
    pub fn active_pane_model(&self) -> &SplitPaneTreePanel<TreeRow> {
        self.pane(self.panes.active_pane)
    }

    /// Mutably borrow the pane that currently drives the related content surface.
    pub fn active_pane_model_mut(&mut self) -> &mut SplitPaneTreePanel<TreeRow> {
        self.pane_mut(self.panes.active_pane)
    }
}
