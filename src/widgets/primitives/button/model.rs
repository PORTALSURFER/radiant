//! Button data model types.

/// Immutable public properties for a reusable button widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ButtonProps {
    /// User-visible label rendered inside the button surface.
    pub label: String,
    /// Whether secondary/right clicks should emit a distinct activation message.
    pub secondary_click: bool,
    /// Whether primary pointer drags should emit drag lifecycle messages.
    pub drag: bool,
}

/// Mutable interaction state for a reusable button widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ButtonState {
    /// Whether a primary press started inside the button and is still armed.
    pub armed: bool,
    /// Whether the current primary press has become a drag.
    pub dragged: bool,
}
