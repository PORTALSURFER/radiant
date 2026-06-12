use crate::gui::input::KeyCode;

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
