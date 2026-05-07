//! Reusable button primitive.

use crate::gui::types::Rect;

use super::support::{WidgetCommon, activate_on_keyboard};
use crate::widgets::contract::{FocusBehavior, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ButtonMessage, PointerButton, WidgetInput};

/// Immutable public properties for a reusable button widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ButtonProps {
    /// User-visible label rendered inside the button surface.
    pub label: String,
}

/// Mutable interaction state for a reusable button widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ButtonState {
    /// Whether a primary press started inside the button and is still armed.
    pub armed: bool,
}

/// Public button primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing button configuration.
    pub props: ButtonProps,
    /// Mutable interaction state owned by the button.
    pub state: ButtonState,
}

impl ButtonWidget {
    /// Build a button descriptor with keyboard focus and activation semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ButtonProps {
                label: label.into(),
            },
            state: ButtonState::default(),
        }
    }

    /// Route one backend-neutral interaction into the button.
    ///
    /// The button emits [`ButtonMessage::Activate`] when a primary press is
    /// released inside bounds or when the focused widget receives Enter/Space.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ButtonMessage> {
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
                let activated =
                    self.common.state.pressed && self.state.armed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.state.armed = false;
                activated.then_some(ButtonMessage::Activate)
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
                Some(ButtonMessage::Activate)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;
    use crate::widgets::interaction::{PointerButton, WidgetInput, WidgetKey};

    #[test]
    fn button_releases_inside_bounds_emit_activation() {
        let mut button =
            ButtonWidget::new(5, "Play", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(80.0, 28.0));

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(20.0, 30.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert!(button.common.state.pressed);

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(24.0, 32.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(ButtonMessage::Activate)
        );
        assert!(!button.common.state.pressed);
    }

    #[test]
    fn focused_button_space_emits_activation() {
        let mut button =
            ButtonWidget::new(6, "Stop", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

        let _ = button.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

        assert_eq!(
            button.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Space)),
            Some(ButtonMessage::Activate)
        );
    }
}
