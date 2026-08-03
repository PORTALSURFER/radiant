use super::super::FocusTraversal;
use crate::{
    gui::input::InputTimestamp,
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
        /// Modifier state captured with this pointer sample.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer modifier state changed while the pointer remains active.
    PointerModifiersChanged {
        /// Latest platform-neutral pointer modifier state.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer press started at the given surface position.
    PointerPress {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that started the press.
        button: PointerButton,
        /// Modifier state when the press started.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer button was pressed twice in quick succession.
    PointerDoubleClick {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that completed the double-click.
        button: PointerButton,
        /// Modifier state when the double-click completed.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer press ended at the given surface position.
    PointerRelease {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that ended the press.
        button: PointerButton,
        /// Modifier state when the press ended.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// One non-text key intent should route to the focused widget.
    KeyPress {
        /// Normalized key identity.
        key: WidgetKey,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// One printable character should route to the focused widget.
    Character {
        /// Character produced by the active keyboard layout.
        character: char,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
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
        /// Modifier state captured with this scroll sample.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
}

impl Event {
    /// Build a viewport resize event.
    pub fn resize(viewport: Vector2) -> Self {
        Self::Resize { viewport }
    }

    /// Build a pointer-move event at `position`.
    pub fn pointer_move(position: Point) -> Self {
        Self::pointer_move_with_metadata(position, PointerModifiers::default(), None)
    }

    /// Build a pointer-move event with native sample metadata.
    pub(crate) fn pointer_move_with_metadata(
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerMove {
            position,
            modifiers,
            timestamp,
        }
    }

    /// Build a pointer-modifier state change event.
    pub fn pointer_modifiers_changed(modifiers: PointerModifiers) -> Self {
        Self::pointer_modifiers_changed_with_timestamp(modifiers, None)
    }

    /// Build a pointer-modifier state change event with an optional native input timestamp.
    pub(crate) fn pointer_modifiers_changed_with_timestamp(
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerModifiersChanged {
            modifiers,
            timestamp,
        }
    }

    /// Build a pointer-press event with explicit button and modifiers.
    pub fn pointer_press(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::pointer_press_with_timestamp(position, button, modifiers, None)
    }

    /// Build a pointer-press event with an optional native input timestamp.
    pub(crate) fn pointer_press_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerPress {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::pointer_double_click_with_timestamp(position, button, modifiers, None)
    }

    /// Build a pointer double-click event with an optional native input timestamp.
    pub(crate) fn pointer_double_click_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerDoubleClick {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::pointer_release_with_timestamp(position, button, modifiers, None)
    }

    /// Build a pointer-release event with an optional native input timestamp.
    pub(crate) fn pointer_release_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerRelease {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::key_press_with_timestamp(key, None)
    }

    /// Build a focused key-press event with an optional native input timestamp.
    pub(crate) fn key_press_with_timestamp(
        key: WidgetKey,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::KeyPress { key, timestamp }
    }

    /// Build a focused character-input event.
    pub fn character(character: char) -> Self {
        Self::character_with_timestamp(character, None)
    }

    /// Build a focused character-input event with an optional native input timestamp.
    pub(crate) fn character_with_timestamp(
        character: char,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::Character {
            character,
            timestamp,
        }
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
        Self::scroll_with_metadata(position, delta, PointerModifiers::default(), None)
    }

    /// Build a pointer-positioned scroll event with native sample metadata.
    pub(crate) fn scroll_with_metadata(
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::Scroll {
            position,
            delta,
            modifiers,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_pointer_move_constructor_omits_sample_metadata() {
        let position = Point::new(12.0, 18.0);

        assert_eq!(
            Event::pointer_move(position),
            Event::PointerMove {
                position,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            }
        );
    }

    #[test]
    fn public_scroll_constructor_omits_sample_metadata() {
        let position = Point::new(12.0, 18.0);
        let delta = Vector2::new(0.0, -24.0);

        assert_eq!(
            Event::scroll(position, delta),
            Event::Scroll {
                position,
                delta,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            }
        );
    }

    #[test]
    fn public_keyboard_constructors_omit_sample_metadata() {
        assert_eq!(
            Event::key_press(WidgetKey::Enter),
            Event::KeyPress {
                key: WidgetKey::Enter,
                timestamp: None,
            }
        );
        assert_eq!(
            Event::character('a'),
            Event::Character {
                character: 'a',
                timestamp: None,
            }
        );
    }
}
