//! Generic panel and split-pane primitives.

use crate::gui::{
    feedback::RecoverySummary,
    list::{EditableTreeActions, EditableTreeRow},
    retained::RetainedVec,
};
use serde::{Deserialize, Serialize};

/// Stable identifier for one side of a two-pane split surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SplitPaneSlot {
    /// Upper or leading pane in the split surface.
    #[default]
    Upper,
    /// Lower or trailing pane in the split surface.
    Lower,
}

impl SplitPaneSlot {
    /// Return a small stable identifier suitable for routing and automation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }

    /// Select the value associated with this split-pane slot.
    pub fn select<'a, T>(self, upper: &'a T, lower: &'a T) -> &'a T {
        match self {
            Self::Upper => upper,
            Self::Lower => lower,
        }
    }

    /// Select the mutable value associated with this split-pane slot.
    pub fn select_mut<'a, T>(self, upper: &'a mut T, lower: &'a mut T) -> &'a mut T {
        match self {
            Self::Upper => upper,
            Self::Lower => lower,
        }
    }
}

/// Labeled row that can be assigned to either side of a two-pane split surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPaneAssignedRow {
    /// Primary row label.
    pub label: String,
    /// Optional secondary detail text.
    pub detail: String,
    /// Whether the row is currently selected.
    pub selected: bool,
    /// Whether the row's backing item is missing or unavailable.
    pub missing: bool,
    /// Whether this row is assigned to the upper/leading pane.
    pub assigned_to_upper_pane: bool,
    /// Whether this row is assigned to the lower/trailing pane.
    pub assigned_to_lower_pane: bool,
}

impl SplitPaneAssignedRow {
    /// Build a new split-pane assignable row.
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        selected: bool,
        missing: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            selected,
            missing,
            assigned_to_upper_pane: false,
            assigned_to_lower_pane: false,
        }
    }

    /// Mark whether this row is assigned to either pane.
    pub fn with_pane_assignment(mut self, upper: bool, lower: bool) -> Self {
        self.assigned_to_upper_pane = upper;
        self.assigned_to_lower_pane = lower;
        self
    }
}

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

#[cfg(test)]
mod tests {
    use super::{SplitPaneAssignedRow, SplitPaneSlot, SplitPaneTreePanel};

    #[test]
    fn split_pane_slot_defaults_to_upper() {
        assert_eq!(SplitPaneSlot::default(), SplitPaneSlot::Upper);
    }

    #[test]
    fn split_pane_slot_exposes_stable_routing_ids() {
        assert_eq!(SplitPaneSlot::Upper.as_str(), "upper");
        assert_eq!(SplitPaneSlot::Lower.as_str(), "lower");
    }

    #[test]
    fn split_pane_slot_selects_matching_value() {
        assert_eq!(
            SplitPaneSlot::Upper.select(&"leading", &"trailing"),
            &"leading"
        );
        assert_eq!(
            SplitPaneSlot::Lower.select(&"leading", &"trailing"),
            &"trailing"
        );
    }

    #[test]
    fn split_pane_slot_selects_matching_value_mutably() {
        let mut upper = String::from("leading");
        let mut lower = String::from("trailing");

        SplitPaneSlot::Lower
            .select_mut(&mut upper, &mut lower)
            .push_str("-selected");

        assert_eq!(upper, "leading");
        assert_eq!(lower, "trailing-selected");
    }

    #[test]
    fn split_pane_assigned_row_preserves_labels_and_assignments() {
        let row = SplitPaneAssignedRow::new("Inbox", "ready", true, false)
            .with_pane_assignment(true, false);

        assert_eq!(row.label, "Inbox");
        assert_eq!(row.detail, "ready");
        assert!(row.selected);
        assert!(!row.missing);
        assert!(row.assigned_to_upper_pane);
        assert!(!row.assigned_to_lower_pane);
    }

    #[test]
    fn split_pane_tree_panel_defaults_to_empty_unassigned_panel() {
        let panel: SplitPaneTreePanel = SplitPaneTreePanel::default();

        assert_eq!(panel.pane, SplitPaneSlot::Upper);
        assert!(!panel.active);
        assert!(!panel.has_item);
        assert!(panel.tree_rows.is_empty());
        assert_eq!(panel.focused_tree_row, None);
    }
}
