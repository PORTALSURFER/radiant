//! Reusable toggle primitive.

use crate::gui::types::Rect;

use super::support::{WidgetCommon, activate_on_keyboard};
use crate::widgets::contract::{FocusBehavior, WidgetId, WidgetSizing};
use crate::widgets::interaction::{PointerButton, ToggleMessage, WidgetInput};

/// Immutable public properties for a reusable toggle widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ToggleProps {
    /// User-visible toggle label.
    pub label: String,
}

/// Mutable interaction state for a reusable toggle widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ToggleState {
    /// Whether the toggle is currently checked/on.
    pub checked: bool,
    /// Whether a primary press started inside the toggle and is still armed.
    pub armed: bool,
}

/// Public toggle primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing toggle configuration.
    pub props: ToggleProps,
    /// Mutable interaction state owned by the toggle.
    pub state: ToggleState,
}

impl ToggleWidget {
    /// Build a toggle descriptor with value-change semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ToggleProps {
                label: label.into(),
            },
            state: ToggleState::default(),
        }
    }

    /// Return this toggle with an explicit checked value.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self.common.state.active = checked;
        self
    }

    /// Route one backend-neutral interaction into the toggle.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ToggleMessage> {
        if self.common.state.disabled {
            self.common.state.pressed = false;
            self.state.armed = false;
            return None;
        }
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.pressed {
                    self.state.armed = self.common.state.hovered;
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.focused = true;
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.state.armed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                let should_toggle =
                    self.common.state.pressed && self.state.armed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.state.armed = false;
                should_toggle.then(|| self.toggle())
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                    self.state.armed = false;
                }
                None
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused && activate_on_keyboard(key) =>
            {
                Some(self.toggle())
            }
            _ => None,
        }
    }

    fn toggle(&mut self) -> ToggleMessage {
        self.state.checked = !self.state.checked;
        self.common.state.active = self.state.checked;
        ToggleMessage::ValueChanged {
            checked: self.state.checked,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;
    use crate::widgets::interaction::WidgetKey;

    #[test]
    fn toggle_keyboard_activation_flips_active_state() {
        let mut toggle =
            ToggleWidget::new(8, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
        let _ = toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

        assert_eq!(
            toggle.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
            Some(ToggleMessage::ValueChanged { checked: true })
        );
        assert!(toggle.common.state.active);

        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
        assert_eq!(
            toggle.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(10.0, 10.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert_eq!(
            toggle.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(10.0, 10.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(ToggleMessage::ValueChanged { checked: false })
        );
    }
}
