use super::{
    EditableTreeActions, EditableTreeRow, RecoverySummary, RetainedVec,
    assigned_row::SplitPaneAssignedRow, slot::SplitPaneSlot, tree_panel::SplitPaneTreePanel,
};

/// Generic sidebar state built around an assignable row list plus a two-pane tree surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPaneSidebarState<Row = SplitPaneAssignedRow, TreeRow = EditableTreeRow> {
    /// Header text for the sidebar.
    pub header: String,
    /// Active search query for the assignable row list.
    pub search_query: String,
    /// Pane that currently drives the related content surface.
    pub active_pane: SplitPaneSlot,
    /// Upper or leading tree pane.
    pub upper_pane: SplitPaneTreePanel<TreeRow>,
    /// Lower or trailing tree pane.
    pub lower_pane: SplitPaneTreePanel<TreeRow>,
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
    /// Selected assignable-row index, if any.
    pub selected_row: Option<usize>,
    /// Assignable row currently hydrating in the background, if any.
    pub loading_row: Option<usize>,
    /// Assignable row currently running a background mutation, if any.
    pub mutation_busy_row: Option<usize>,
    /// Focused tree row index, if any.
    pub focused_tree_row: Option<usize>,
    /// Assignable rows to render in the sidebar.
    pub rows: RetainedVec<Row>,
    /// Tree rows to render in the active tree surface.
    pub tree_rows: RetainedVec<TreeRow>,
    /// Tree action availability for controls.
    pub tree_actions: EditableTreeActions,
    /// Delete/recovery summary for sidebar status.
    pub recovery: RecoverySummary,
}

impl<Row, TreeRow> Default for SplitPaneSidebarState<Row, TreeRow> {
    fn default() -> Self {
        Self {
            header: String::new(),
            search_query: String::new(),
            active_pane: SplitPaneSlot::default(),
            upper_pane: SplitPaneTreePanel::default(),
            lower_pane: SplitPaneTreePanel::default(),
            tree_search_query: String::new(),
            show_all_items: false,
            can_toggle_show_all_items: false,
            flattened_view: false,
            can_toggle_flattened_view: false,
            selected_row: None,
            loading_row: None,
            mutation_busy_row: None,
            focused_tree_row: None,
            rows: RetainedVec::new(),
            tree_rows: RetainedVec::new(),
            tree_actions: EditableTreeActions::default(),
            recovery: RecoverySummary::default(),
        }
    }
}

impl<Row, TreeRow> SplitPaneSidebarState<Row, TreeRow> {
    /// Borrow one pane by id.
    pub fn pane(&self, pane: SplitPaneSlot) -> &SplitPaneTreePanel<TreeRow> {
        pane.select(&self.upper_pane, &self.lower_pane)
    }

    /// Mutably borrow one pane by id.
    pub fn pane_mut(&mut self, pane: SplitPaneSlot) -> &mut SplitPaneTreePanel<TreeRow> {
        pane.select_mut(&mut self.upper_pane, &mut self.lower_pane)
    }

    /// Borrow the pane that currently drives the related content surface.
    pub fn active_pane_model(&self) -> &SplitPaneTreePanel<TreeRow> {
        self.pane(self.active_pane)
    }

    /// Mutably borrow the pane that currently drives the related content surface.
    pub fn active_pane_model_mut(&mut self) -> &mut SplitPaneTreePanel<TreeRow> {
        self.pane_mut(self.active_pane)
    }
}
