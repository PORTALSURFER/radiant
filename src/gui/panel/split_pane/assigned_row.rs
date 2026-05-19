/// Pane assignment flags for a split-pane row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitPaneAssignment {
    /// Whether this row is assigned to the upper/leading pane.
    pub upper: bool,
    /// Whether this row is assigned to the lower/trailing pane.
    pub lower: bool,
}

/// Explicit parts used to build a split-pane assignable row.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SplitPaneAssignedRowParts {
    /// Primary row label.
    pub label: String,
    /// Optional secondary detail text.
    pub detail: String,
    /// Whether the row is currently selected.
    pub selected: bool,
    /// Whether the row's backing item is missing or unavailable.
    pub missing: bool,
    /// Current pane assignment flags.
    pub assignment: SplitPaneAssignment,
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
    /// Build a split-pane assignable row from named generic parts.
    pub fn from_parts(parts: SplitPaneAssignedRowParts) -> Self {
        Self {
            label: parts.label,
            detail: parts.detail,
            selected: parts.selected,
            missing: parts.missing,
            assigned_to_upper_pane: parts.assignment.upper,
            assigned_to_lower_pane: parts.assignment.lower,
        }
    }

    /// Build a new split-pane assignable row.
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        selected: bool,
        missing: bool,
    ) -> Self {
        Self::from_parts(SplitPaneAssignedRowParts {
            label: label.into(),
            detail: detail.into(),
            selected,
            missing,
            assignment: SplitPaneAssignment::default(),
        })
    }

    /// Mark whether this row is assigned to either pane from named flags.
    pub fn with_assignment(mut self, assignment: SplitPaneAssignment) -> Self {
        self.assigned_to_upper_pane = assignment.upper;
        self.assigned_to_lower_pane = assignment.lower;
        self
    }

    /// Mark whether this row is assigned to either pane.
    pub fn with_pane_assignment(self, upper: bool, lower: bool) -> Self {
        self.with_assignment(SplitPaneAssignment { upper, lower })
    }
}
