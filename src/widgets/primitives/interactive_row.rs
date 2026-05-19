//! Reusable dense-list/tree row interaction primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, WidgetMessageMapper};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    DragHandleMessage, InteractiveRowMessage, PointerButton, WidgetInput, WidgetKey, WidgetOutput,
};
use crate::widgets::primitives::support::{WidgetCommon, push_control_chrome};

/// Public interactive row primitive for selectable, draggable, droppable rows.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveRowWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable row configuration.
    pub props: InteractiveRowProps,
    dragged: bool,
}

/// Immutable interactive row configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct InteractiveRowProps {
    /// Emit drag lifecycle messages after pointer movement while pressed.
    pub draggable: bool,
    /// Emit drop and hover-drop-target messages.
    pub droppable: bool,
    /// Whether another row drag is currently active in this container.
    pub drag_active: bool,
}

/// Named construction fields for [`InteractiveRowWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveRowWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Intrinsic interactive-row sizing contract.
    pub sizing: WidgetSizing,
}

impl InteractiveRowWidget {
    /// Build an interactive row descriptor from named identity and sizing fields.
    pub fn from_parts(parts: InteractiveRowWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        Self {
            common,
            props: InteractiveRowProps::default(),
            dragged: false,
        }
    }

    /// Build an interactive row descriptor.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::from_parts(InteractiveRowWidgetParts { id, sizing })
    }

    /// Enable drag lifecycle messages.
    pub fn with_drag(mut self) -> Self {
        self.props.draggable = true;
        self
    }

    /// Enable drop and drop-hover messages.
    pub fn with_drop_target(mut self, drag_active: bool) -> Self {
        self.props.droppable = true;
        self.props.drag_active = drag_active;
        self
    }

    /// Route one backend-neutral interaction into the row.
    pub fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<InteractiveRowMessage> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                if self.common.state.pressed && self.props.draggable {
                    let message = if self.dragged {
                        DragHandleMessage::Moved { position }
                    } else {
                        self.dragged = true;
                        DragHandleMessage::Started { position }
                    };
                    return Some(InteractiveRowMessage::Drag(message));
                }
                if self.common.state.hovered && self.props.droppable && self.props.drag_active {
                    return Some(InteractiveRowMessage::HoverDropTarget);
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                self.dragged = false;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated =
                    self.common.state.pressed && !self.dragged && bounds.contains(position);
                let dragged = self.common.state.pressed && self.dragged;
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.dragged = false;
                if dragged {
                    return Some(InteractiveRowMessage::Drag(DragHandleMessage::Ended {
                        position,
                    }));
                }
                activated.then_some(InteractiveRowMessage::Activate)
            }
            WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } if self.props.droppable && bounds.contains(position) => {
                Some(InteractiveRowMessage::Drop)
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
            WidgetInput::KeyPress(WidgetKey::Enter | WidgetKey::Space)
                if self.common.state.focused =>
            {
                Some(InteractiveRowMessage::Activate)
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
        }
    }
}

impl Widget for InteractiveRowWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        InteractiveRowWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.dragged = previous.dragged;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        if self.common.paint.paints_state_layers {
            push_control_chrome(primitives, &self.common, bounds, theme);
        }
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build an interactive-row message mapper.
    pub fn interactive_row(
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::typed(map)
    }
}
