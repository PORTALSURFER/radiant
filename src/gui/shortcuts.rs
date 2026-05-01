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
}
