//! Reusable drag-handle primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, push_drag_handle_widget_paint};
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{DragHandleMessage, PointerButton, WidgetInput, WidgetOutput};

/// Public drag handle primitive for pointer-driven reordering.
#[derive(Clone, Debug, PartialEq)]
pub struct DragHandleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
}

impl DragHandleWidget {
    /// Build a compact handle that emits drag lifecycle messages.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Pointer;
        Self { common }
    }

    /// Route one backend-neutral interaction into the handle.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<DragHandleMessage> {
        if self.common.state.disabled {
            return None;
        }

        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.common
                    .state
                    .pressed
                    .then_some(DragHandleMessage::Moved { position })
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.pressed = true;
                self.common.state.active = true;
                Some(DragHandleMessage::Started { position })
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                self.common.state.pressed = false;
                self.common.state.active = false;
                Some(DragHandleMessage::Ended { position })
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }
}

impl Widget for DragHandleWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        DragHandleWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_drag_handle_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a drag-handle-message mapper.
    pub fn drag_handle(map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a drag handle with a custom widget-to-host message mapper.
    pub fn drag_handle_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            DragHandleWidget::new(id, sizing),
            WidgetMessageMapper::drag_handle(map),
        )
    }
}
