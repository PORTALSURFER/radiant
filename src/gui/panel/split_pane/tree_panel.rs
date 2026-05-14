use super::{
    EditableTreeActions, EditableTreeRow, RecoverySummary, RetainedVec, slot::SplitPaneSlot,
};

/// Generic tree/list panel assigned to one side of a split surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPaneTreePanel<Row = EditableTreeRow> {
    /// Stable pane identity used by routing.
    pub pane: SplitPaneSlot,
    /// Short title shown in the pane header.
    pub title: String,
    /// Primary label for the item currently assigned to the pane.
    pub item_label: String,
    /// Secondary detail text for the assigned item.
    pub item_detail: String,
    /// Whether this pane currently drives the related content surface.
    pub active: bool,
    /// Whether an item is assigned to this pane.
    pub has_item: bool,
    /// Whether this pane is hydrating its assigned item snapshot.
    pub loading: bool,
    /// Whether this pane is asynchronously rebuilding its tree rows.
    pub projecting: bool,
    /// Whether this pane's assigned item currently owns a background mutation.
    pub mutation_busy: bool,
    /// Active tree-search query for this pane.
    pub tree_search_query: String,
    /// Whether the tree currently includes otherwise hidden empty items.
    pub show_all_items: bool,
    /// Whether the hidden-item visibility toggle is currently actionable.
    pub can_toggle_show_all_items: bool,
    /// Whether tree filtering includes descendant items in a flattened list.
    pub flattened_view: bool,
    /// Whether the flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Focused tree row index, if any.
    pub focused_tree_row: Option<usize>,
    /// Tree rows to render in this pane.
    pub tree_rows: RetainedVec<Row>,
    /// Tree action availability projected for this pane.
    pub tree_actions: EditableTreeActions,
    /// Delete/recovery summary projected for this pane.
    pub recovery: RecoverySummary,
}

impl<Row> Default for SplitPaneTreePanel<Row> {
    fn default() -> Self {
        Self {
            pane: SplitPaneSlot::default(),
            title: String::new(),
            item_label: String::new(),
            item_detail: String::new(),
            active: false,
            has_item: false,
            loading: false,
            projecting: false,
            mutation_busy: false,
            tree_search_query: String::new(),
            show_all_items: false,
            can_toggle_show_all_items: false,
            flattened_view: false,
            can_toggle_flattened_view: false,
            focused_tree_row: None,
            tree_rows: RetainedVec::new(),
            tree_actions: EditableTreeActions::default(),
            recovery: RecoverySummary::default(),
        }
    }
}
