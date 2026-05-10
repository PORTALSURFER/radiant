use crate::runtime::PaintText;

/// Immutable public properties for a reusable badge or pill widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BadgeProps {
    /// User-visible badge label.
    pub label: PaintText,
}

/// Mutable interaction state for a reusable badge or pill widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BadgeState {
    /// Whether a primary press started inside the badge and is still armed.
    pub armed: bool,
}
