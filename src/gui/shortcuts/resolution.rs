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
