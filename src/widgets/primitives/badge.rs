//! Reusable badge and pill primitive.

use crate::gui::types::Rect;

use super::support::{WidgetCommon, activate_on_keyboard};
use crate::widgets::contract::{
    FocusBehavior, WidgetId, WidgetKind, WidgetMessageKind, WidgetProminence, WidgetSizing,
    WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{BadgeMessage, PointerButton, WidgetInput};

/// Immutable public properties for a reusable badge or pill widget.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BadgeProps {
    /// User-visible badge label.
    pub label: String,
}

/// Mutable interaction state for a reusable badge or pill widget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BadgeState {
    /// Whether a primary press started inside the badge and is still armed.
    pub armed: bool,
}

/// Public badge/pill primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct BadgeWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing badge configuration.
    pub props: BadgeProps,
    /// Mutable interaction state owned by the badge.
    pub state: BadgeState,
}

impl BadgeWidget {
    /// Build a badge descriptor with optional activation semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::Badge, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        common.emitted_messages.push(WidgetMessageKind::Activate);
        Self {
            common,
            props: BadgeProps {
                label: label.into(),
            },
            state: BadgeState::default(),
        }
    }

    /// Route one backend-neutral interaction into the badge.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<BadgeMessage> {
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
                activated.then_some(BadgeMessage::Activate)
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
                Some(BadgeMessage::Activate)
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
    fn badge_releases_inside_bounds_emit_activation() {
        let mut badge =
            BadgeWidget::new(5, "Filter", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(72.0, 24.0));

        assert_eq!(
            badge.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(20.0, 30.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert!(badge.common.state.pressed);

        assert_eq!(
            badge.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(24.0, 32.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(BadgeMessage::Activate)
        );
        assert!(!badge.common.state.pressed);
    }

    #[test]
    fn focused_badge_enter_emits_activation() {
        let mut badge =
            BadgeWidget::new(6, "Active", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));

        let _ = badge.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

        assert_eq!(
            badge.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
            Some(BadgeMessage::Activate)
        );
    }
}
