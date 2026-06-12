/// Host-owned drag/drop state for a generic tree row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TreeRowDragDropState {
    /// A related row or external item is currently being dragged.
    pub drag_active: bool,
    /// This row is the source of the active drag.
    pub drag_source: bool,
    /// This row is a valid drop candidate.
    pub drop_candidate: bool,
    /// This row is the committed drop target.
    pub drop_target: bool,
    /// The surrounding tree currently has a committed drop target.
    pub drop_target_active: bool,
}

impl TreeRowDragDropState {
    /// Build an inactive drag/drop state.
    pub const fn new() -> Self {
        Self {
            drag_active: false,
            drag_source: false,
            drop_candidate: false,
            drop_target: false,
            drop_target_active: false,
        }
    }

    /// Return whether the row should clear stale hover when synchronized.
    pub const fn clears_hover_on_sync(self) -> bool {
        (self.drag_active || self.drop_target_active) && !self.drop_target
    }
}
