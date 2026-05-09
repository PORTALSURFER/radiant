//! Generic shortcut resolution DTOs for host-owned command catalogs.

use crate::gui::input::KeyPress;

/// Result of resolving one keypress against a host shortcut catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutResolution<Action> {
    /// Action produced by this keypress, if any.
    pub action: Option<Action>,
    /// Whether the keypress was consumed by the shortcut system.
    pub handled: bool,
    /// Pending chord starter to carry into the next keypress, if any.
    pub pending_chord: Option<KeyPress>,
}

impl<Action> ShortcutResolution<Action> {
    /// Build an unhandled result with no pending chord.
    pub fn unhandled() -> Self {
        Self {
            action: None,
            handled: false,
            pending_chord: None,
        }
    }

    /// Build a handled result that dispatches one host action.
    pub fn action(action: Action) -> Self {
        Self {
            action: Some(action),
            handled: true,
            pending_chord: None,
        }
    }

    /// Build a handled result without dispatching an action.
    pub fn handled() -> Self {
        Self {
            action: None,
            handled: true,
            pending_chord: None,
        }
    }

    /// Build a handled result that waits for the next chord keypress.
    pub fn pending_chord(pending_chord: KeyPress) -> Self {
        Self {
            action: None,
            handled: true,
            pending_chord: Some(pending_chord),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShortcutResolution;

    #[test]
    fn shortcut_resolution_unhandled_has_no_action_or_chord() {
        let resolution = ShortcutResolution::<u8>::unhandled();

        assert_eq!(resolution.action, None);
        assert!(!resolution.handled);
        assert_eq!(resolution.pending_chord, None);
    }

    #[test]
    fn shortcut_resolution_constructors_preserve_action_handled_and_chord_state() {
        use crate::gui::input::{KeyCode, KeyPress};

        let action = ShortcutResolution::action(7);
        assert_eq!(action.action, Some(7));
        assert!(action.handled);
        assert_eq!(action.pending_chord, None);

        let handled = ShortcutResolution::<u8>::handled();
        assert_eq!(handled.action, None);
        assert!(handled.handled);

        let chord = ShortcutResolution::<u8>::pending_chord(KeyPress::new(KeyCode::G));
        assert_eq!(chord.action, None);
        assert!(chord.handled);
        assert_eq!(chord.pending_chord, Some(KeyPress::new(KeyCode::G)));
    }
}
