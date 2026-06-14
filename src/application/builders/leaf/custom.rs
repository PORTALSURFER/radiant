use super::core::view_node_from_widget;
use crate::{
    application::{DynamicWidget, MappedWidget, ViewNode, WidgetView},
    runtime::WidgetMessageMapper,
    widgets::{Widget, WidgetOutput},
};

/// Build a view node from any application widget view.
pub fn widget<Message>(widget: impl WidgetView<Message> + 'static) -> ViewNode<Message> {
    view_node_from_widget(widget)
}

/// Build a custom widget view with generated identity and an output mapper.
pub fn custom_widget<Message: 'static>(
    widget: impl Widget + Clone + 'static,
    map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static,
) -> ViewNode<Message> {
    view_node_from_widget(DynamicWidget::new(widget, map))
}

/// Build a custom widget view with a typed output mapper.
///
/// This is the application-builder companion to
/// [`WidgetMessageMapper::typed`]. Use it when a custom widget emits one
/// concrete output payload with [`WidgetOutput::typed`] or
/// [`WidgetOutput::custom`] and every matching output should become a host
/// message.
pub fn custom_widget_mapped<Output, Message>(
    widget: impl Widget + Clone + 'static,
    map: impl Fn(Output) -> Message + Send + Sync + 'static,
) -> ViewNode<Message>
where
    Output: Clone + Send + Sync + 'static,
    Message: 'static,
{
    view_node_from_widget(MappedWidget::new(widget, WidgetMessageMapper::typed(map)))
}

/// Build a custom widget view whose typed output is already the host message.
///
/// Use this when a custom widget emits [`WidgetOutput::typed`] payloads with
/// the same concrete type as the surrounding application message, avoiding an
/// identity mapper at every call site.
pub fn custom_widget_direct<Message>(widget: impl Widget + Clone + 'static) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    view_node_from_widget(MappedWidget::new(
        widget,
        WidgetMessageMapper::typed(|message: Message| message),
    ))
}
