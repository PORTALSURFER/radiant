//! Runtime builder helpers for custom canvas primitives.

use crate::runtime::{SurfaceNode, WidgetMessageMapper};
use crate::widgets::contract::{WidgetId, WidgetSizing};
use crate::widgets::interaction::CanvasMessage;

use super::{CanvasWidget, RetainedSurfaceDescriptor};

impl<Message> WidgetMessageMapper<Message> {
    /// Build a canvas-message mapper.
    pub fn canvas(map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting canvas leaf node for custom paint or routed input surfaces.
    pub fn canvas(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(CanvasWidget::new(id, sizing))
    }

    /// Build a canvas leaf node with a custom widget-to-host message mapper.
    pub fn canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            CanvasWidget::new(id, sizing),
            WidgetMessageMapper::canvas(map),
        )
    }

    /// Build a custom canvas with retained-surface metadata and a host-message mapper.
    pub fn retained_canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        retained: RetainedSurfaceDescriptor,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            CanvasWidget::new(id, sizing).with_retained_surface(retained),
            WidgetMessageMapper::canvas(map),
        )
    }
}
