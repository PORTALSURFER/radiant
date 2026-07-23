use crate::runtime::PaintText;

/// Visual chrome treatment for a badge or pill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BadgeChrome {
    /// Paint the standard filled control surface.
    #[default]
    Filled,
    /// Paint a background-matched surface with a one-pixel outline.
    Outline,
}

/// Immutable public properties for a reusable badge or pill widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BadgeProps {
    /// User-visible badge label.
    pub label: PaintText,
    /// Badge surface treatment.
    pub chrome: BadgeChrome,
}

/// Mutable interaction state for a reusable badge or pill widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BadgeState {
    /// Whether a primary press started inside the badge and is still armed.
    pub armed: bool,
}
