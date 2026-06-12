/// Generic visual state for a dense list or tree row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DenseRowVisualState {
    /// The row is selected by the host application.
    pub selected: bool,
    /// Pointer is hovering the row.
    pub hovered: bool,
    /// Primary pointer activation is pressed or armed.
    pub pressed: bool,
    /// The row is the committed target for an active operation.
    pub active_target: bool,
    /// The row is a valid candidate for an active operation.
    pub candidate: bool,
}

impl DenseRowVisualState {
    /// Return whether this dense row is in a state that should emphasize its label.
    ///
    /// Use this for custom row painters whose foreground text should become
    /// higher-contrast for selected rows, committed operation targets, or
    /// hovered operation candidates while following Radiant's dense-row state
    /// priority.
    pub const fn emphasizes_label(self) -> bool {
        self.active_target || (self.hovered && self.candidate) || self.selected
    }
}
