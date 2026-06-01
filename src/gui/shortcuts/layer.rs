use super::{ShortcutGesture, ShortcutResolution};
use crate::gui::input::{KeyCode, KeyPress};

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

    /// Build a modal layer that dispatches `action` for Escape and consumes other keys.
    pub fn modal_escape(action: Action) -> Self {
        Self::modal().bind(KeyPress::new(KeyCode::Escape), action)
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
