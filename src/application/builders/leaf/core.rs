use crate::application::{ViewNode, ViewNodeKind, WidgetView};

pub(in crate::application) fn view_node_from_widget<Message>(
    widget: impl WidgetView<Message> + 'static,
) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::Widget(Box::new(widget)))
}
