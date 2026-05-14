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
