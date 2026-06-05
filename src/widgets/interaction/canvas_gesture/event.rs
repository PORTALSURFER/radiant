use super::pointer::CanvasPointer;
use crate::{
    gui::types::{Rect, Vector2},
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

impl CanvasGestureEvent {
    /// Return the pointer for a hover event.
    pub fn hover_pointer(&self) -> Option<CanvasPointer> {
        match self {
            Self::Hover(pointer) => Some(*pointer),
            _ => None,
        }
    }

    /// Return the pointer for a press with `button`.
    pub fn press_pointer(&self, button: PointerButton) -> Option<CanvasPointer> {
        match self {
            Self::Press {
                pointer,
                button: pressed,
                ..
            } if *pressed == button => Some(*pointer),
            _ => None,
        }
    }

    /// Return the pointer for a press with `button` when it is inside `bounds`.
    pub fn press_pointer_inside(
        &self,
        bounds: Rect,
        button: PointerButton,
    ) -> Option<CanvasPointer> {
        self.press_pointer(button)
            .filter(|pointer| pointer.is_inside(bounds))
    }

    /// Return the pointer for a double-click with `button`.
    pub fn double_click_pointer(&self, button: PointerButton) -> Option<CanvasPointer> {
        match self {
            Self::DoubleClick {
                pointer,
                button: clicked,
                ..
            } if *clicked == button => Some(*pointer),
            _ => None,
        }
    }

    /// Return the pointer for a double-click with `button` when it is inside `bounds`.
    pub fn double_click_pointer_inside(
        &self,
        bounds: Rect,
        button: PointerButton,
    ) -> Option<CanvasPointer> {
        self.double_click_pointer(button)
            .filter(|pointer| pointer.is_inside(bounds))
    }

    /// Return the pointer for a release with `button`.
    pub fn release_pointer(&self, button: PointerButton) -> Option<CanvasPointer> {
        match self {
            Self::Release {
                pointer,
                button: released,
                ..
            } if *released == button => Some(*pointer),
            _ => None,
        }
    }

    /// Return the pointer for a release with `button` when it is inside `bounds`.
    pub fn release_pointer_inside(
        &self,
        bounds: Rect,
        button: PointerButton,
    ) -> Option<CanvasPointer> {
        self.release_pointer(button)
            .filter(|pointer| pointer.is_inside(bounds))
    }

    /// Return the pointer and delta for a wheel event.
    pub fn wheel_pointer_delta(&self) -> Option<(CanvasPointer, Vector2)> {
        match self {
            Self::Wheel { pointer, delta } => Some((*pointer, *delta)),
            _ => None,
        }
    }

    /// Return the pointer and delta for a wheel event when the pointer is inside `bounds`.
    pub fn wheel_pointer_delta_inside(&self, bounds: Rect) -> Option<(CanvasPointer, Vector2)> {
        self.wheel_pointer_delta()
            .filter(|(pointer, _)| pointer.is_inside(bounds))
    }

    /// Return the current pointer carried by pointer-like gesture events.
    pub fn pointer(&self) -> Option<CanvasPointer> {
        match self {
            Self::Hover(pointer)
            | Self::Press { pointer, .. }
            | Self::Drag { pointer, .. }
            | Self::Release { pointer, .. }
            | Self::DoubleClick { pointer, .. }
            | Self::Wheel { pointer, .. }
            | Self::Drop { pointer, .. } => Some(*pointer),
            Self::FocusChanged(_) => None,
        }
    }

    /// Return the captured gesture origin when the event has one.
    pub fn origin(&self) -> Option<CanvasPointer> {
        match self {
            Self::Drag { origin, .. } | Self::Release { origin, .. } => Some(*origin),
            Self::Drop { origin, .. } => *origin,
            Self::Hover(_)
            | Self::Press { .. }
            | Self::DoubleClick { .. }
            | Self::Wheel { .. }
            | Self::FocusChanged(_) => None,
        }
    }

    /// Return the button associated with pointer-button gesture events.
    pub fn button(&self) -> Option<PointerButton> {
        match self {
            Self::Press { button, .. }
            | Self::Drag { button, .. }
            | Self::Release { button, .. }
            | Self::DoubleClick { button, .. }
            | Self::Drop { button, .. } => Some(*button),
            Self::Hover(_) | Self::Wheel { .. } | Self::FocusChanged(_) => None,
        }
    }

    /// Return the pointer modifiers associated with button gesture events.
    pub fn modifiers(&self) -> Option<PointerModifiers> {
        match self {
            Self::Press { modifiers, .. }
            | Self::Drag { modifiers, .. }
            | Self::Release { modifiers, .. }
            | Self::DoubleClick { modifiers, .. }
            | Self::Drop { modifiers, .. } => Some(*modifiers),
            Self::Hover(_) | Self::Wheel { .. } | Self::FocusChanged(_) => None,
        }
    }

    /// Return the logical movement delta associated with drag-like gesture events.
    pub fn delta(&self) -> Option<Vector2> {
        match self {
            Self::Drag { delta, .. } | Self::Release { delta, .. } | Self::Wheel { delta, .. } => {
                Some(*delta)
            }
            Self::Hover(_)
            | Self::Press { .. }
            | Self::DoubleClick { .. }
            | Self::Drop { .. }
            | Self::FocusChanged(_) => None,
        }
    }

    /// Return whether the event's current pointer is inside `bounds`.
    pub fn pointer_is_inside(&self, bounds: Rect) -> bool {
        self.pointer()
            .is_some_and(|pointer| pointer.is_inside(bounds))
    }
}
