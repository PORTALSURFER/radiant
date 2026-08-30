//! Transparent pointer interception primitive for modal and loading overlays.

use crate::gui::types::{Point, Rect};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetCapabilities, WidgetHitTest, WidgetHitTestResult,
    WidgetHitTestRevision, WidgetId, WidgetPointerMotion, WidgetPointerMotionRevision,
    WidgetSizing,
};
use crate::widgets::interaction::{PointerShieldMessage, WidgetInput, WidgetOutput};
use crate::widgets::primitives::support::WidgetCommon;

/// Transparent widget that consumes selected pointer interactions inside its bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerShieldWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable pointer interception policy.
    pub props: PointerShieldProps,
}

/// Immutable pointer interception policy for [`PointerShieldWidget`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointerShieldProps {
    /// Whether the shield currently intercepts pointer input.
    pub active: bool,
    /// Emit messages for pointer movement.
    pub pointer_move: bool,
    /// Emit messages for primary/secondary/auxiliary pointer press.
    pub pointer_press: bool,
    /// Emit messages for pointer release.
    pub pointer_release: bool,
    /// Emit messages for captured pointer drops.
    pub pointer_drop: bool,
    /// Emit messages for wheel input.
    pub wheel: bool,
}

impl Default for PointerShieldProps {
    fn default() -> Self {
        Self {
            active: true,
            pointer_move: true,
            pointer_press: true,
            pointer_release: true,
            pointer_drop: true,
            wheel: true,
        }
    }
}

/// Named construction fields for [`PointerShieldWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct PointerShieldWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Intrinsic pointer-shield sizing contract.
    pub sizing: WidgetSizing,
    /// Pointer interception policy.
    pub props: PointerShieldProps,
}

impl PointerShieldWidget {
    /// Build a pointer shield from named construction fields.
    pub fn from_parts(parts: PointerShieldWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        common.state.disabled = !parts.props.active;
        Self {
            common,
            props: parts.props,
        }
    }

    /// Build an active pointer shield with fixed sizing.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::from_parts(PointerShieldWidgetParts {
            id,
            sizing,
            props: PointerShieldProps::default(),
        })
    }

    /// Build a fill-style pointer shield with a generated runtime id.
    pub fn fill(active: bool) -> Self {
        Self::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0))).active(active)
    }

    /// Build a shield that only reports pointer movement.
    pub fn pointer_move_only(active: bool) -> Self {
        Self::fill(active)
            .with_pointer_press(false)
            .with_pointer_release(false)
            .with_pointer_drop(false)
            .with_wheel(false)
    }

    /// Build a shield that only reports captured pointer drops.
    pub fn pointer_drop_only(active: bool) -> Self {
        Self::fill(active)
            .with_pointer_move(false)
            .with_pointer_press(false)
            .with_pointer_release(false)
            .with_wheel(false)
    }

    /// Set whether the shield intercepts pointer input.
    pub fn active(mut self, active: bool) -> Self {
        self.props.active = active;
        self.common.state.disabled = !active;
        self
    }

    /// Set whether pointer movement is intercepted.
    pub fn with_pointer_move(mut self, enabled: bool) -> Self {
        self.props.pointer_move = enabled;
        self
    }

    /// Set whether pointer press is intercepted.
    pub fn with_pointer_press(mut self, enabled: bool) -> Self {
        self.props.pointer_press = enabled;
        self
    }

    /// Set whether pointer release is intercepted.
    pub fn with_pointer_release(mut self, enabled: bool) -> Self {
        self.props.pointer_release = enabled;
        self
    }

    /// Set whether captured pointer drops are intercepted.
    pub fn with_pointer_drop(mut self, enabled: bool) -> Self {
        self.props.pointer_drop = enabled;
        self
    }

    /// Set whether wheel input is intercepted.
    pub fn with_wheel(mut self, enabled: bool) -> Self {
        self.props.wheel = enabled;
        self
    }

    /// Route one backend-neutral pointer interaction into the shield.
    pub fn handle_input(&self, bounds: Rect, input: WidgetInput) -> Option<PointerShieldMessage> {
        if !self.props.active {
            return None;
        }
        match input {
            WidgetInput::PointerMove {
                position,
                timestamp,
                sequence_range,
                ..
            } if self.props.pointer_move && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerMove {
                    position,
                    timestamp,
                    sequence_range,
                })
            }
            WidgetInput::PointerPress {
                position,
                button,
                modifiers,
                timestamp,
            } if self.props.pointer_press && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerPress {
                    position,
                    button,
                    modifiers,
                    timestamp,
                })
            }
            WidgetInput::PointerRelease {
                position,
                button,
                modifiers,
                timestamp,
            } if self.props.pointer_release && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerRelease {
                    position,
                    button,
                    modifiers,
                    timestamp,
                })
            }
            WidgetInput::PointerDrop {
                position,
                button,
                modifiers,
                timestamp,
            } if self.props.pointer_drop && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerDrop {
                    position,
                    button,
                    modifiers,
                    timestamp,
                })
            }
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
            } if self.props.wheel && bounds.contains(position) => {
                Some(PointerShieldMessage::Wheel {
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                })
            }
            _ => None,
        }
    }
}

impl WidgetHitTest for PointerShieldWidget {
    fn revision(&self) -> WidgetHitTestRevision {
        WidgetHitTestRevision::exact(self.props)
    }

    fn hit_test(&self, _bounds: Rect, _point: Point, input: &WidgetInput) -> WidgetHitTestResult {
        if self.allows_pointer_event(input) {
            WidgetHitTestResult::Opaque
        } else {
            WidgetHitTestResult::PassThrough
        }
    }
}

impl WidgetPointerMotion for PointerShieldWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact((self.props.active, self.props.pointer_move))
    }

    fn accepts_pointer_move(&self) -> bool {
        self.props.active && self.props.pointer_move
    }
}

impl PointerShieldWidget {
    fn allows_pointer_event(&self, input: &WidgetInput) -> bool {
        if !self.props.active {
            return false;
        }
        match input {
            WidgetInput::PointerMove { .. } => self.props.pointer_move,
            WidgetInput::PointerPress { .. } => self.props.pointer_press,
            WidgetInput::PointerDoubleClick { .. } => self.props.pointer_press,
            WidgetInput::PointerRelease { .. } => self.props.pointer_release,
            WidgetInput::PointerDrop { .. } => self.props.pointer_drop,
            WidgetInput::Wheel { .. } => self.props.wheel,
            WidgetInput::PointerModifiersChanged { .. }
            | WidgetInput::FocusChanged(_)
            | WidgetInput::KeyPress { .. }
            | WidgetInput::KeyRelease { .. }
            | WidgetInput::Character { .. }
            | WidgetInput::TextEdit { .. } => true,
        }
    }
}

impl Widget for PointerShieldWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        PointerShieldWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new()
            .with_hit_test(self)
            .with_pointer_motion(self)
    }

    fn accepts_wheel_input(&self) -> bool {
        self.props.active && self.props.wheel
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::{
            input::{InputSequence, InputSequenceRange, InputTimestamp},
            types::Point,
        },
        layout::Vector2,
        widgets::{PointerButton, PointerModifiers},
    };

    #[test]
    fn preserves_normalized_pointer_metadata_in_messages() {
        let bounds = Rect::from_xy_size(0.0, 0.0, 120.0, 18.0);
        let position = Point::new(16.0, 8.0);
        let delta = Vector2::new(0.0, -18.0);
        let timestamp = Some(InputTimestamp::capture());
        let sequence_range = Some(InputSequenceRange::singleton(
            InputSequence::from_runtime_value(17),
        ));
        let shield = PointerShieldWidget::fill(true);

        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::pointer_move_with_metadata(
                    position,
                    PointerModifiers::default(),
                    timestamp,
                    sequence_range,
                ),
            ),
            Some(PointerShieldMessage::PointerMove {
                position,
                timestamp,
                sequence_range,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position,
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                    timestamp,
                },
            ),
            Some(PointerShieldMessage::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position,
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                    timestamp,
                },
            ),
            Some(PointerShieldMessage::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::PointerDrop {
                    position,
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                    timestamp,
                },
            ),
            Some(PointerShieldMessage::PointerDrop {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::wheel_with_metadata(
                    position,
                    delta,
                    PointerModifiers::default(),
                    timestamp,
                    sequence_range,
                ),
            ),
            Some(PointerShieldMessage::Wheel {
                position,
                delta,
                modifiers: PointerModifiers::default(),
                timestamp,
                sequence_range,
            })
        );
    }

    #[test]
    fn public_widget_input_constructors_keep_message_metadata_absent() {
        let bounds = Rect::from_xy_size(0.0, 0.0, 120.0, 18.0);
        let position = Point::new(16.0, 8.0);
        let delta = Vector2::new(0.0, -18.0);
        let shield = PointerShieldWidget::fill(true);

        assert_eq!(
            shield.handle_input(bounds, WidgetInput::pointer_move(position)),
            Some(PointerShieldMessage::PointerMove {
                position,
                timestamp: None,
                sequence_range: None,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::pointer_press(
                    position,
                    PointerButton::Primary,
                    PointerModifiers::default(),
                ),
            ),
            Some(PointerShieldMessage::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::pointer_release(
                    position,
                    PointerButton::Primary,
                    PointerModifiers::default(),
                ),
            ),
            Some(PointerShieldMessage::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::pointer_drop(
                    position,
                    PointerButton::Primary,
                    PointerModifiers::default(),
                ),
            ),
            Some(PointerShieldMessage::PointerDrop {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            })
        );
        assert_eq!(
            shield.handle_input(
                bounds,
                WidgetInput::wheel(position, delta, PointerModifiers::default()),
            ),
            Some(PointerShieldMessage::Wheel {
                position,
                delta,
                modifiers: PointerModifiers::default(),
                timestamp: None,
                sequence_range: None,
            })
        );
    }
}
