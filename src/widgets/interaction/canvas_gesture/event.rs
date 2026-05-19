use super::pointer::CanvasPointer;
use crate::{
    gui::types::Vector2,
    widgets::interaction::{PointerButton, PointerModifiers},
};

/// High-level canvas gesture event resolved from [`crate::widgets::interaction::WidgetInput`].
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasGestureEvent {
    /// Pointer moved without an active drag.
    Hover(CanvasPointer),
    /// Pointer button pressed.
    Press {
        /// Pointer information at press time.
        pointer: CanvasPointer,
        /// Pressed button.
        button: PointerButton,
        /// Modifier state at press time.
        modifiers: PointerModifiers,
    },
    /// Pointer moved while the same button is captured.
    Drag {
        /// Pointer information for the current move.
        pointer: CanvasPointer,
        /// Pointer information from the original press.
        origin: CanvasPointer,
        /// Drag delta in host logical coordinates.
        delta: Vector2,
        /// Captured button.
        button: PointerButton,
        /// Modifier state from the original press.
        modifiers: PointerModifiers,
    },
    /// Captured pointer button was released.
    Release {
        /// Pointer information at release time.
        pointer: CanvasPointer,
        /// Pointer information from the original press.
        origin: CanvasPointer,
        /// Release delta in host logical coordinates.
        delta: Vector2,
        /// Released button.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// Pointer button was double-clicked.
    DoubleClick {
        /// Pointer information at double-click time.
        pointer: CanvasPointer,
        /// Clicked button.
        button: PointerButton,
        /// Modifier state at double-click time.
        modifiers: PointerModifiers,
    },
    /// Pointer wheel or trackpad scroll occurred.
    Wheel {
        /// Pointer information at wheel time.
        pointer: CanvasPointer,
        /// Logical scroll delta. Positive values move content right/down.
        delta: Vector2,
    },
    /// Captured pointer was dropped or canceled.
    Drop {
        /// Pointer information at drop time.
        pointer: CanvasPointer,
        /// Pointer information from the original press, when this state owned one.
        origin: Option<CanvasPointer>,
        /// Dropped button.
        button: PointerButton,
        /// Modifier state at drop time.
        modifiers: PointerModifiers,
    },
    /// Keyboard focus changed.
    FocusChanged(bool),
}
