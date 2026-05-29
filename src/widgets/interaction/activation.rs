//! Shared activation-state handling for custom widgets.

use crate::{
    gui::types::Rect,
    widgets::{
        contract::WidgetState,
        interaction::{PointerButton, WidgetInput, WidgetKey},
    },
};

#[cfg(test)]
#[path = "activation/tests.rs"]
mod tests;

/// Pointer and keyboard policy for reusable activation handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationInputPolicy {
    /// Whether a primary pointer press should move keyboard focus to the widget.
    pub focus_on_press: bool,
    /// Whether focused Enter and Space key presses should activate the widget.
    pub keyboard: bool,
}

impl ActivationInputPolicy {
    /// Pointer-only activation without focus changes or keyboard activation.
    pub const fn pointer_only() -> Self {
        Self {
            focus_on_press: false,
            keyboard: false,
        }
    }

    /// Pointer activation plus keyboard activation for focusable controls.
    pub const fn focusable() -> Self {
        Self {
            focus_on_press: true,
            keyboard: true,
        }
    }
}

/// Result of routing one input through activation handling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivationInputResult {
    /// Input did not activate the widget.
    #[default]
    None,
    /// Input activated the widget.
    Activated,
}

impl ActivationInputResult {
    /// Returns true when the routed input activated the widget.
    pub const fn activated(self) -> bool {
        matches!(self, Self::Activated)
    }
}

/// Apply common hover, pressed, focus, and activation transitions for a widget.
pub fn handle_activation_input(
    state: &mut WidgetState,
    bounds: Rect,
    input: &WidgetInput,
    policy: ActivationInputPolicy,
) -> ActivationInputResult {
    if state.disabled {
        state.pressed = false;
        return ActivationInputResult::None;
    }
    match input {
        WidgetInput::PointerMove { position } => {
            state.hovered = bounds.contains(*position);
            ActivationInputResult::None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(*position) => {
            state.hovered = true;
            state.pressed = true;
            if policy.focus_on_press {
                state.focused = true;
            }
            ActivationInputResult::None
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            ..
        } => {
            let activated = state.pressed && bounds.contains(*position);
            state.pressed = false;
            state.hovered = bounds.contains(*position);
            if activated {
                ActivationInputResult::Activated
            } else {
                ActivationInputResult::None
            }
        }
        WidgetInput::PointerRelease { .. } => {
            state.pressed = false;
            ActivationInputResult::None
        }
        WidgetInput::FocusChanged(focused) => {
            state.focused = *focused;
            if !focused {
                state.pressed = false;
            }
            ActivationInputResult::None
        }
        WidgetInput::KeyPress(key)
            if policy.keyboard && state.focused && activate_on_keyboard(*key) =>
        {
            ActivationInputResult::Activated
        }
        _ => ActivationInputResult::None,
    }
}

fn activate_on_keyboard(key: WidgetKey) -> bool {
    matches!(key, WidgetKey::Enter | WidgetKey::Space)
}
