//! Runtime builder helpers for drag-handle primitives.

use crate::runtime::{SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::DragHandleMessage;

use super::DragHandleWidget;

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
