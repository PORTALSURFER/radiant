use super::super::FocusTraversal;
use crate::{
    gui::types::{Point, Vector2},
    widgets::{PointerButton, PointerModifiers, WidgetKey},
};

/// Backend-neutral runtime event routed through a
/// [`SurfaceRuntime`](crate::runtime::controller::SurfaceRuntime).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Viewport size changed and layout should be recomputed.
    Resize {
        /// New logical viewport size.
        viewport: Vector2,
    },
    /// Pointer hover moved across the surface.
    PointerMove {
        /// Pointer position in surface logical coordinates.
        position: Point,
    },
    /// Pointer modifier state changed while the pointer remains active.
    PointerModifiersChanged {
        /// Latest platform-neutral pointer modifier state.
        modifiers: PointerModifiers,
    },
    /// Pointer press started at the given surface position.
    PointerPress {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that started the press.
        button: PointerButton,
        /// Modifier state when the press started.
        modifiers: PointerModifiers,
    },
    /// Pointer button was pressed twice in quick succession.
    PointerDoubleClick {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that completed the double-click.
        button: PointerButton,
        /// Modifier state when the double-click completed.
        modifiers: PointerModifiers,
    },
    /// Pointer press ended at the given surface position.
    PointerRelease {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that ended the press.
        button: PointerButton,
        /// Modifier state when the press ended.
        modifiers: PointerModifiers,
    },
    /// One non-text key intent should route to the focused widget.
    KeyPress(WidgetKey),
    /// One printable character should route to the focused widget.
    Character(char),
    /// Move keyboard focus in declarative tree order.
    TraverseFocus(FocusTraversal),
    /// Clear current runtime focus ownership.
    ClearFocus,
    /// Scroll the scrollable container under the pointer by logical pixels.
    Scroll {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Logical scroll delta. Positive values move content right/down.
        delta: Vector2,
    },
}

impl Event {
    /// Build a viewport resize event.
    pub fn resize(viewport: Vector2) -> Self {
        Self::Resize { viewport }
    }

    /// Build a pointer-move event at `position`.
    pub fn pointer_move(position: Point) -> Self {
        Self::PointerMove { position }
    }

    /// Build a pointer-modifier state change event.
    pub fn pointer_modifiers_changed(modifiers: PointerModifiers) -> Self {
        Self::PointerModifiersChanged { modifiers }
    }

    /// Build a pointer-press event with explicit button and modifiers.
    pub fn pointer_press(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerPress {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer press with no keyboard modifiers.
    pub fn primary_press(position: Point) -> Self {
        Self::pointer_press(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a secondary-button pointer press with no keyboard modifiers.
    pub fn secondary_press(position: Point) -> Self {
        Self::pointer_press(
            position,
            PointerButton::Secondary,
            PointerModifiers::default(),
        )
    }

    /// Build a pointer double-click event with explicit button and modifiers.
    pub fn pointer_double_click(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerDoubleClick {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer double-click with no keyboard modifiers.
    pub fn primary_double_click(position: Point) -> Self {
        Self::pointer_double_click(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a pointer-release event with explicit button and modifiers.
    pub fn pointer_release(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerRelease {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer release with no keyboard modifiers.
    pub fn primary_release(position: Point) -> Self {
        Self::pointer_release(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a secondary-button pointer release with no keyboard modifiers.
    pub fn secondary_release(position: Point) -> Self {
        Self::pointer_release(
            position,
            PointerButton::Secondary,
            PointerModifiers::default(),
        )
    }

    /// Build a focused key-press event.
    pub fn key_press(key: WidgetKey) -> Self {
        Self::KeyPress(key)
    }

    /// Build a focused character-input event.
    pub fn character(character: char) -> Self {
        Self::Character(character)
    }

    /// Build a focus-traversal event.
    pub fn traverse_focus(direction: FocusTraversal) -> Self {
        Self::TraverseFocus(direction)
    }

    /// Build a focus-clear event.
    pub fn clear_focus() -> Self {
        Self::ClearFocus
    }

    /// Build a pointer-positioned scroll event.
    pub fn scroll(position: Point, delta: Vector2) -> Self {
        Self::Scroll { position, delta }
    }
}
