//! Widget focus and interaction-state contracts.

/// Focus participation contract for a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusBehavior {
    /// Widget cannot receive focus.
    None,
    /// Widget can receive pointer focus but is skipped by keyboard traversal.
    Pointer,
    /// Widget participates in deterministic keyboard focus traversal.
    Keyboard,
}

/// Shared visual-state vocabulary for widget styling and behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WidgetState {
    /// Pointer is currently hovering the widget.
    pub hovered: bool,
    /// Primary action is currently pressed/armed.
    pub pressed: bool,
    /// Widget currently owns keyboard focus.
    pub focused: bool,
    /// Widget is semantically selected.
    pub selected: bool,
    /// Widget is semantically active/on.
    pub active: bool,
    /// Widget rejects interaction but still paints.
    pub disabled: bool,
    /// Widget is read-only but remains visible/focusable.
    pub read_only: bool,
}
