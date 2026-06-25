use crate::gui::input::{KeyCode, KeyPress};

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
    pub(super) const fn matches(self, active: bool) -> bool {
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
    /// Platform command modifier policy.
    pub command: ShortcutModifier,
    /// Physical Control modifier policy.
    pub control: ShortcutModifier,
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
            control: ShortcutModifier::Off,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact platform command key gesture.
    pub const fn with_command(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::On,
            control: ShortcutModifier::Off,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact physical Control key gesture.
    pub const fn with_control(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            control: ShortcutModifier::On,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact shift key gesture.
    pub const fn with_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            control: ShortcutModifier::Off,
            shift: ShortcutModifier::On,
            alt: ShortcutModifier::Off,
        }
    }

    /// Build an exact alt key gesture.
    pub const fn with_alt(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            control: ShortcutModifier::Off,
            shift: ShortcutModifier::Off,
            alt: ShortcutModifier::On,
        }
    }

    /// Allow either shifted or unshifted presses for the same key.
    pub const fn any_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: ShortcutModifier::Off,
            control: ShortcutModifier::Off,
            shift: ShortcutModifier::Any,
            alt: ShortcutModifier::Off,
        }
    }

    /// Return whether this gesture accepts `press`.
    pub fn matches(self, press: KeyPress) -> bool {
        self.key == press.key
            && self.command.matches(press.command)
            && self.control.matches(press.control)
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
            control: if press.control {
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
