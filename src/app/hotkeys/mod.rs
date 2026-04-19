//! Shared hotkey catalog for native runtime routing and host presentation.
//!
//! The native runtime and host-side help/automation surfaces must agree on the
//! exact hotkey contract. This module keeps the public gesture and binding
//! types together while delegating catalog ownership and routing helpers to
//! focused siblings.

mod catalog;
mod resolution;

use super::{FocusContextModel, UiAction};
use crate::gui::input::KeyCode;

pub(crate) use catalog::HOTKEY_BINDINGS;
pub(crate) use resolution::{HotkeyResolution, resolve_hotkey_press};

/// Logical section scope that owns a hotkey binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyScope {
    /// Binding is always active regardless of section focus.
    Global,
    /// Binding is active only when the matching section owns focus.
    Focus(FocusContextModel),
}

impl HotkeyScope {
    /// Return whether this scope is active for the provided focus context.
    pub fn matches(self, focus: FocusContextModel) -> bool {
        match self {
            Self::Global => true,
            Self::Focus(target) => target == focus,
        }
    }
}

/// One physical keypress plus modifier state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    /// Physical key identity.
    pub key: KeyCode,
    /// Whether the platform command modifier is held.
    pub command: bool,
    /// Whether Shift is held.
    pub shift: bool,
    /// Whether Alt is held.
    pub alt: bool,
}

impl KeyPress {
    /// Build an unmodified keypress.
    pub const fn new(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: false,
        }
    }

    /// Build a command-modified keypress.
    pub const fn with_command(key: KeyCode) -> Self {
        Self {
            key,
            command: true,
            shift: false,
            alt: false,
        }
    }

    /// Build a shift-modified keypress.
    pub const fn with_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: true,
            alt: false,
        }
    }

    /// Build an alt-modified keypress.
    pub const fn with_alt(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: true,
        }
    }
}

/// Keyboard gesture used to trigger one binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeyGesture {
    /// First keypress in the gesture.
    pub first: KeyPress,
    /// Optional chord follow-up keypress.
    pub chord: Option<KeyPress>,
}

impl HotkeyGesture {
    /// Build a single-key gesture.
    pub const fn new(key: KeyCode) -> Self {
        Self {
            first: KeyPress::new(key),
            chord: None,
        }
    }

    /// Build a single-key command gesture.
    pub const fn with_command(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_command(key),
            chord: None,
        }
    }

    /// Build a single-key shift gesture.
    pub const fn with_shift(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_shift(key),
            chord: None,
        }
    }

    /// Build a single-key alt gesture.
    pub const fn with_alt(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_alt(key),
            chord: None,
        }
    }

    /// Build a two-step chord gesture.
    pub const fn with_chord(first: KeyPress, second: KeyPress) -> Self {
        Self {
            first,
            chord: Some(second),
        }
    }
}

/// One cataloged hotkey binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyBinding {
    /// Stable binding identifier for tests and overlays.
    pub id: &'static str,
    /// Human-readable label shown in help surfaces.
    pub label: &'static str,
    /// Keyboard gesture that triggers the binding.
    pub gesture: HotkeyGesture,
    /// Section scope that owns the binding.
    pub scope: HotkeyScope,
    /// Action emitted when the gesture resolves.
    pub action: UiAction,
}

impl HotkeyBinding {
    /// Return whether the binding is active for the provided focus context.
    pub fn is_active(&self, focus: FocusContextModel) -> bool {
        self.scope.matches(focus)
    }
}

pub(crate) const GLOBAL_SCOPE: HotkeyScope = HotkeyScope::Global;
pub(crate) const BROWSER_SCOPE: HotkeyScope = HotkeyScope::Focus(FocusContextModel::SampleBrowser);
pub(crate) const WAVEFORM_SCOPE: HotkeyScope = HotkeyScope::Focus(FocusContextModel::Waveform);
pub(crate) const FOLDERS_SCOPE: HotkeyScope = HotkeyScope::Focus(FocusContextModel::SourceFolders);
pub(crate) const SOURCES_SCOPE: HotkeyScope = HotkeyScope::Focus(FocusContextModel::SourcesList);

/// Iterate over every shared hotkey binding in stable presentation order.
pub fn iter_hotkey_bindings() -> impl Iterator<Item = &'static HotkeyBinding> {
    HOTKEY_BINDINGS.iter()
}
