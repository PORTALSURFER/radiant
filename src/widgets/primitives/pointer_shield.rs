//! Transparent pointer interception primitive for modal and loading overlays.

use crate::gui::types::Rect;
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
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
}

impl Default for PointerShieldProps {
    fn default() -> Self {
        Self {
            active: true,
            pointer_move: true,
            pointer_press: true,
            pointer_release: true,
            pointer_drop: true,
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
    }

    /// Build a shield that only reports captured pointer drops.
    pub fn pointer_drop_only(active: bool) -> Self {
        Self::fill(active)
            .with_pointer_move(false)
            .with_pointer_press(false)
            .with_pointer_release(false)
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

    /// Route one backend-neutral pointer interaction into the shield.
    pub fn handle_input(&self, bounds: Rect, input: WidgetInput) -> Option<PointerShieldMessage> {
        if !self.props.active {
            return None;
        }
        match input {
            WidgetInput::PointerMove { position }
                if self.props.pointer_move && bounds.contains(position) =>
            {
                Some(PointerShieldMessage::PointerMove { position })
            }
            WidgetInput::PointerPress {
                position,
                button,
                modifiers,
            } if self.props.pointer_press && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerPress {
                    position,
                    button,
                    modifiers,
                })
            }
            WidgetInput::PointerRelease {
                position,
                button,
                modifiers,
            } if self.props.pointer_release && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerRelease {
                    position,
                    button,
                    modifiers,
                })
            }
            WidgetInput::PointerDrop {
                position,
                button,
                modifiers,
            } if self.props.pointer_drop && bounds.contains(position) => {
                Some(PointerShieldMessage::PointerDrop {
                    position,
                    button,
                    modifiers,
                })
            }
            _ => None,
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

    fn accepts_pointer_move(&self) -> bool {
        self.props.active && self.props.pointer_move
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
