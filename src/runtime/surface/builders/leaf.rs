use super::super::{
    SurfaceNode,
    widget::{SurfaceWidget, WidgetMessageMapper},
};
use crate::widgets::Widget;

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
}
