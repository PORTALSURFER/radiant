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

impl WidgetState {
    /// Return whether the pointer is currently hovering the widget.
    pub const fn is_hovered(self) -> bool {
        self.hovered
    }

    /// Return whether the primary action is currently pressed or armed.
    pub const fn is_pressed(self) -> bool {
        self.pressed
    }

    /// Return whether the widget currently owns keyboard focus.
    pub const fn is_focused(self) -> bool {
        self.focused
    }

    /// Return whether the widget is semantically selected.
    pub const fn is_selected(self) -> bool {
        self.selected
    }

    /// Return whether the widget is semantically active or on.
    pub const fn is_active(self) -> bool {
        self.active
    }

    /// Return whether the widget rejects interaction while still painting.
    pub const fn is_disabled(self) -> bool {
        self.disabled
    }

    /// Return whether the widget is read-only but remains visible or focusable.
    pub const fn is_read_only(self) -> bool {
        self.read_only
    }
}
