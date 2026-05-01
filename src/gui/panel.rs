//! Generic panel and split-pane primitives.

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

#[cfg(test)]
mod tests {
    use super::{SplitPaneAssignedRow, SplitPaneSlot};

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
}
