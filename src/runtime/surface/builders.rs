use super::{
    SurfaceNode, SurfaceOverlay,
    widget::{SurfaceWidget, WidgetMessageMapper},
};
use crate::{
    gui::types::Rect,
    layout::NodeId,
    runtime::PaintText,
    widgets::{Widget, WidgetStyle},
};

mod container;

impl<Message> SurfaceNode<Message> {
    /// Build a widget leaf node.
    pub fn widget(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::Widget(SurfaceWidget::new(widget, messages))
    }

    /// Build a custom widget leaf node.
    pub fn custom_widget(
        widget: impl Widget + Clone + 'static,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::Widget(SurfaceWidget::custom(widget, messages))
    }

    /// Build a custom boxed widget leaf node.
    pub fn custom_widget_box(
        widget: Box<dyn Widget>,
        messages: WidgetMessageMapper<Message>,
    ) -> Self {
        Self::Widget(SurfaceWidget::custom_box(widget, messages))
    }

    /// Build a widget leaf node that does not emit host-defined messages.
    pub fn static_widget(widget: impl Widget + Clone + 'static) -> Self {
        Self::widget(widget, WidgetMessageMapper::none())
    }

    /// Build a non-interactive floating overlay panel in surface coordinates.
    pub fn overlay_panel(
        id: NodeId,
        rect: Rect,
        label: impl Into<String>,
        style: WidgetStyle,
    ) -> Self {
        Self::Overlay(SurfaceOverlay {
            id,
            rect,
            label: Some(PaintText::from(label.into())),
            style,
        })
    }

    /// Build a non-interactive floating overlay marker in surface coordinates.
    pub fn overlay_marker(id: NodeId, rect: Rect, style: WidgetStyle) -> Self {
        Self::Overlay(SurfaceOverlay {
            id,
            rect,
            label: None,
            style,
        })
    }
}
