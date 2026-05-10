//! Reusable drag-handle primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{DragHandleMessage, WidgetInput, WidgetOutput};

mod input;
mod paint;

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
        input::handle_drag_handle_input(self, bounds, input)
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
        paint::push_drag_handle_widget_paint(primitives, self, bounds, theme);
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
