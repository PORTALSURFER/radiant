use crate::gui::input::{KeyCode, KeyPress};

/// Keyboard modifier state preserved at the backend-neutral widget boundary.
///
/// At the native widget boundary, `command` represents physical Super/Meta and
/// `control` represents physical Control. Host shortcut `KeyPress` values may
/// project Control into `command` on non-macOS platforms; widget delivery keeps
/// these physical modifiers separate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyboardModifiers {
    /// Whether the platform command modifier is held.
    pub command: bool,
    /// Whether physical Control is held separately from the command modifier.
    pub control: bool,
    /// Whether Shift is held.
    pub shift: bool,
    /// Whether Alt/Option is held.
    pub alt: bool,
}

impl From<KeyPress> for KeyboardModifiers {
    fn from(press: KeyPress) -> Self {
        Self {
            command: press.command,
            control: press.control,
            shift: press.shift,
            alt: press.alt,
        }
    }
}

/// Backend-neutral key intents consumed by reusable widget primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKey {
    /// Activate or submit the focused widget.
    Enter,
    /// Cancel the active edit in the focused widget.
    Escape,
    /// Request completion for the focused widget.
    Tab,
    /// Activate the focused widget.
    Space,
    /// Move one logical position toward the leading edge.
    ArrowLeft,
    /// Move one logical position toward the trailing edge.
    ArrowRight,
    /// Move one logical position upward.
    ArrowUp,
    /// Move one logical position downward.
    ArrowDown,
    /// Move to the start of the value or range.
    Home,
    /// Move to the end of the value or range.
    End,
    /// Move one viewport toward the leading edge.
    PageUp,
    /// Move one viewport toward the trailing edge.
    PageDown,
    /// Delete the codepoint before the caret.
    Backspace,
    /// Delete the codepoint after the caret.
    Delete,
}

impl WidgetKey {
    /// Convert a backend-neutral GUI key code into a widget-edit key when supported.
    pub fn from_key_code(key: KeyCode) -> Option<Self> {
        Some(match key {
            KeyCode::Enter => Self::Enter,
            KeyCode::Escape => Self::Escape,
            KeyCode::Tab => Self::Tab,
            KeyCode::Space => Self::Space,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::ArrowLeft => Self::ArrowLeft,
            KeyCode::ArrowRight => Self::ArrowRight,
            KeyCode::ArrowUp => Self::ArrowUp,
            KeyCode::ArrowDown => Self::ArrowDown,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::Delete => Self::Delete,
            _ => return None,
        })
    }

    /// Convert a normalized widget key back to the host shortcut key code.
    pub const fn to_key_code(self) -> KeyCode {
        match self {
            Self::Enter => KeyCode::Enter,
            Self::Escape => KeyCode::Escape,
            Self::Tab => KeyCode::Tab,
            Self::Space => KeyCode::Space,
            Self::ArrowLeft => KeyCode::ArrowLeft,
            Self::ArrowRight => KeyCode::ArrowRight,
            Self::ArrowUp => KeyCode::ArrowUp,
            Self::ArrowDown => KeyCode::ArrowDown,
            Self::Home => KeyCode::Home,
            Self::End => KeyCode::End,
            Self::PageUp => KeyCode::PageUp,
            Self::PageDown => KeyCode::PageDown,
            Self::Backspace => KeyCode::Backspace,
            Self::Delete => KeyCode::Delete,
        }
    }
}

/// Return whether a focused key may be offered to the runtime scroll fallback.
pub const fn is_scroll_fallback_key(key: WidgetKey) -> bool {
    matches!(
        key,
        WidgetKey::PageUp | WidgetKey::PageDown | WidgetKey::Home | WidgetKey::End
    )
}

impl From<WidgetKey> for KeyCode {
    fn from(key: WidgetKey) -> Self {
        key.to_key_code()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_modifiers_preserve_distinct_command_and_control_state() {
        let modifiers = KeyboardModifiers::from(KeyPress {
            key: KeyCode::G,
            command: true,
            control: true,
            shift: true,
            alt: false,
        });

        assert_eq!(
            modifiers,
            KeyboardModifiers {
                command: true,
                control: true,
                shift: true,
                alt: false,
            }
        );
    }

    #[test]
    fn widget_key_maps_supported_gui_key_codes() {
        assert_eq!(
            WidgetKey::from_key_code(KeyCode::Enter),
            Some(WidgetKey::Enter)
        );
        assert_eq!(
            WidgetKey::from_key_code(KeyCode::Escape),
            Some(WidgetKey::Escape)
        );
        assert_eq!(WidgetKey::from_key_code(KeyCode::Tab), Some(WidgetKey::Tab));
        assert_eq!(
            WidgetKey::from_key_code(KeyCode::ArrowLeft),
            Some(WidgetKey::ArrowLeft)
        );
        assert_eq!(
            WidgetKey::from_key_code(KeyCode::Delete),
            Some(WidgetKey::Delete)
        );
    }

    #[test]
    fn widget_key_ignores_non_widget_key_codes() {
        assert_eq!(WidgetKey::from_key_code(KeyCode::Num0), None);
    }

    #[test]
    fn widget_key_round_trips_supported_key_codes() {
        for key in [
            WidgetKey::Enter,
            WidgetKey::Escape,
            WidgetKey::Tab,
            WidgetKey::Space,
            WidgetKey::ArrowLeft,
            WidgetKey::ArrowRight,
            WidgetKey::ArrowUp,
            WidgetKey::ArrowDown,
            WidgetKey::Home,
            WidgetKey::End,
            WidgetKey::Backspace,
            WidgetKey::Delete,
        ] {
            assert_eq!(WidgetKey::from_key_code(key.to_key_code()), Some(key));
            assert_eq!(KeyCode::from(key), key.to_key_code());
        }
    }
}
