//! Generic shortcut resolution primitives for host-owned command catalogs.

use crate::gui::input::{KeyCode, KeyPress};

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

/// Match policy for a shortcut modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutModifier {
    /// Modifier must be inactive.
    Off,
    /// Modifier must be active.
    On,
    /// Modifier can be active or inactive.
    Any,
}

impl ShortcutModifier {
    const fn matches(self, active: bool) -> bool {
        match self {
            Self::Off => !active,
            Self::On => active,
            Self::Any => true,
        }
    }
}

/// A key gesture matched against a normalized [`KeyPress`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutGesture {
    /// Primary key.
    pub key: KeyCode,
    /// Command/control modifier policy.
    pub command: ShortcutModifier,
    /// Shift modifier policy.
    pub shift: ShortcutModifier,
    /// Alt modifier policy.
    pub alt: ShortcutModifier,
}

impl ShortcutGesture {
    /// Build an exact unmodified key gesture.
    pub const fn new(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact command/control key gesture.
    pub const fn with_command(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::On,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact shift key gesture.
    pub const fn with_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            shift: ShortcutModifier::On,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact alt key gesture.
    pub const fn with_alt(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::On,
        }
    }

    /// Allow either shifted or unshifted presses for the same key.
    pub const fn any_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            shift: ShortcutModifier::Any,
            alt: ShortcutModifier::Off,
        }
    }

    /// Return whether this gesture accepts `press`.
    pub fn matches(self, press: KeyPress) -> bool {
        self.key == press.key
            && self.command.matches(press.command)
            && self.shift.matches(press.shift)
            && self.alt.matches(press.alt)
    }
}

impl From<KeyPress> for ShortcutGesture {
    fn from(press: KeyPress) -> Self {
        Self {
            key: press.key,
            command: if press.command {
                ShortcutModifier::On
            } else {
                ShortcutModifier::Off
            },
            shift: if press.shift {
                ShortcutModifier::On
            } else {
                ShortcutModifier::Off
            },
            alt: if press.alt {
                ShortcutModifier::On
            } else {
                ShortcutModifier::Off
            },
        }
    }
}

/// One resolved shortcut binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutBinding<Action> {
    /// Gesture that triggers this binding.
    pub gesture: ShortcutGesture,
    /// Host action emitted when the gesture matches.
    pub action: Action,
}

/// A small shortcut layer that can either pass through misses or consume them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutLayer<Action> {
    bindings: Vec<ShortcutBinding<Action>>,
    modal: bool,
}

impl<Action> Default for ShortcutLayer<Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Action> ShortcutLayer<Action> {
    /// Build a non-modal shortcut layer.
    pub const fn new() -> Self {
        Self {
            bindings: Vec::new(),
            modal: false,
        }
    }

    /// Build a modal shortcut layer that consumes unmatched keys.
    pub const fn modal() -> Self {
        Self {
            bindings: Vec::new(),
            modal: true,
        }
    }

    /// Add one binding to this layer.
    pub fn bind(mut self, gesture: impl Into<ShortcutGesture>, action: Action) -> Self {
        self.bindings.push(ShortcutBinding {
            gesture: gesture.into(),
            action,
        });
        self
    }

    /// Return whether this layer consumes unmatched keypresses.
    pub const fn is_modal(&self) -> bool {
        self.modal
    }

    /// Resolve `press` against this layer.
    pub fn resolve(&self, press: KeyPress) -> ShortcutResolution<Action>
    where
        Action: Clone,
    {
        self.resolve_or_else(press, ShortcutResolution::unhandled)
    }

    /// Resolve `press`, calling `fallback` only for non-modal misses.
    pub fn resolve_or_else(
        &self,
        press: KeyPress,
        fallback: impl FnOnce() -> ShortcutResolution<Action>,
    ) -> ShortcutResolution<Action>
    where
        Action: Clone,
    {
        if let Some(binding) = self
            .bindings
            .iter()
            .find(|binding| binding.gesture.matches(press))
        {
            ShortcutResolution::action(binding.action.clone())
        } else if self.modal {
            ShortcutResolution::handled()
        } else {
            fallback()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ShortcutGesture, ShortcutLayer, ShortcutResolution};
    use crate::gui::input::{KeyCode, KeyPress};

    #[test]
    fn shortcut_resolution_unhandled_has_no_action_or_chord() {
        let resolution = ShortcutResolution::<u8>::unhandled();

        assert_eq!(resolution.action, None);
        assert!(!resolution.handled);
        assert_eq!(resolution.pending_chord, None);
    }

    #[test]
    fn shortcut_resolution_constructors_preserve_action_handled_and_chord_state() {
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

    #[test]
    fn shortcut_gesture_matches_explicit_and_any_shift_modifiers() {
        assert!(ShortcutGesture::new(KeyCode::N).matches(KeyPress::new(KeyCode::N)));
        assert!(!ShortcutGesture::new(KeyCode::N).matches(KeyPress::with_shift(KeyCode::N)));
        assert!(ShortcutGesture::any_shift(KeyCode::N).matches(KeyPress::new(KeyCode::N)));
        assert!(ShortcutGesture::any_shift(KeyCode::N).matches(KeyPress::with_shift(KeyCode::N)));
        assert!(
            ShortcutGesture::with_command(KeyCode::A).matches(KeyPress::with_command(KeyCode::A))
        );
    }

    #[test]
    fn shortcut_layer_resolves_actions_and_modal_misses() {
        let layer = ShortcutLayer::new()
            .bind(KeyPress::new(KeyCode::Escape), 1)
            .bind(ShortcutGesture::with_command(KeyCode::A), 2);

        assert_eq!(
            layer.resolve(KeyPress::new(KeyCode::Escape)),
            ShortcutResolution::action(1)
        );
        assert_eq!(
            layer.resolve(KeyPress::with_command(KeyCode::A)),
            ShortcutResolution::action(2)
        );
        assert_eq!(
            layer.resolve(KeyPress::new(KeyCode::N)),
            ShortcutResolution::unhandled()
        );

        let modal = ShortcutLayer::modal().bind(KeyPress::new(KeyCode::Escape), 3);
        assert_eq!(
            modal.resolve(KeyPress::new(KeyCode::Escape)),
            ShortcutResolution::action(3)
        );
        assert_eq!(
            modal.resolve(KeyPress::new(KeyCode::N)),
            ShortcutResolution::handled()
        );
    }
}
