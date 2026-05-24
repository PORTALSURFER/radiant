/// Named assignment state for a split-pane row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitPaneAssignmentState {
    /// The row is not assigned to either pane.
    #[default]
    Free,
    /// The row is assigned to the upper/leading pane.
    Upper,
    /// The row is assigned to the lower/trailing pane.
    Lower,
    /// The row is assigned to both panes.
    Both,
}

/// Pane assignment flags for a split-pane row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitPaneAssignment {
    /// Whether this row is assigned to the upper/leading pane.
    pub upper: bool,
    /// Whether this row is assigned to the lower/trailing pane.
    pub lower: bool,
}

impl SplitPaneAssignment {
    /// Build pane assignment flags from a named state.
    pub const fn from_state(state: SplitPaneAssignmentState) -> Self {
        match state {
            SplitPaneAssignmentState::Free => Self {
                upper: false,
                lower: false,
            },
            SplitPaneAssignmentState::Upper => Self {
                upper: true,
                lower: false,
            },
            SplitPaneAssignmentState::Lower => Self {
                upper: false,
                lower: true,
            },
            SplitPaneAssignmentState::Both => Self {
                upper: true,
                lower: true,
            },
        }
    }

    /// Return the named assignment state represented by these flags.
    pub const fn state(self) -> SplitPaneAssignmentState {
        SplitPaneAssignmentState::from_flags(self.upper, self.lower)
    }
}

impl SplitPaneAssignmentState {
    /// Build an assignment state from compatibility flags.
    pub const fn from_flags(upper: bool, lower: bool) -> Self {
        match (upper, lower) {
            (true, true) => Self::Both,
            (true, false) => Self::Upper,
            (false, true) => Self::Lower,
            (false, false) => Self::Free,
        }
    }
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
    /// Current pane assignment state.
    pub assignment: SplitPaneAssignmentState,
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
        let assignment = SplitPaneAssignment::from_state(parts.assignment);
        Self {
            label: parts.label,
            detail: parts.detail,
            selected: parts.selected,
            missing: parts.missing,
            assigned_to_upper_pane: assignment.upper,
            assigned_to_lower_pane: assignment.lower,
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
            assignment: SplitPaneAssignmentState::default(),
        })
    }

    /// Return this row's named pane assignment state.
    pub const fn assignment_state(&self) -> SplitPaneAssignmentState {
        SplitPaneAssignmentState::from_flags(
            self.assigned_to_upper_pane,
            self.assigned_to_lower_pane,
        )
    }

    /// Mark whether this row is assigned to either pane from named flags.
    pub fn with_assignment(mut self, assignment: SplitPaneAssignment) -> Self {
        self.assigned_to_upper_pane = assignment.upper;
        self.assigned_to_lower_pane = assignment.lower;
        self
    }

    /// Mark this row with a named pane assignment state.
    pub fn with_assignment_state(self, state: SplitPaneAssignmentState) -> Self {
        self.with_assignment(SplitPaneAssignment::from_state(state))
    }

    /// Add this row to a pane without clearing its other pane assignment.
    pub fn assign_to_pane(&mut self, pane: super::SplitPaneSlot) {
        match pane {
            super::SplitPaneSlot::Upper => self.assigned_to_upper_pane = true,
            super::SplitPaneSlot::Lower => self.assigned_to_lower_pane = true,
        }
    }

    /// Mark whether this row is assigned to either pane.
    pub fn with_pane_assignment(self, upper: bool, lower: bool) -> Self {
        self.with_assignment_state(SplitPaneAssignmentState::from_flags(upper, lower))
    }
}
