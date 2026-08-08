use crate::gui::input::{KeyCode, KeyPress};

/// Keyboard modifier state preserved at the backend-neutral widget boundary.
///
/// The command and control flags remain distinct so a numeric consumer can
/// apply the platform's semantic Fine/Coarse policy without guessing which
/// native modifier was pressed.
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
            KeyCode::Tab => Self::Tab,
            KeyCode::Space => Self::Space,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::ArrowLeft => Self::ArrowLeft,
            KeyCode::ArrowRight => Self::ArrowRight,
            KeyCode::ArrowUp => Self::ArrowUp,
            KeyCode::ArrowDown => Self::ArrowDown,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::Delete => Self::Delete,
            _ => return None,
        })
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
}
