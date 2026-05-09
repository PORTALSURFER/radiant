//! Toggle data model types.

/// Immutable public properties for a reusable toggle widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToggleProps {
    /// User-visible toggle label.
    pub label: String,
}

/// Mutable interaction state for a reusable toggle widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ToggleState {
    /// Whether the toggle is currently checked/on.
    pub checked: bool,
    /// Whether a primary press started inside the toggle and is still armed.
    pub armed: bool,
}
