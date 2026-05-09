use super::*;

impl<Message> SurfaceNode<Message> {
    fn collect_widget_paint_order(&self, order: &mut Vec<WidgetId>) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.collect_widget_paint_order(order);
                }
            }
            Self::Widget(widget) => order.push(widget.id()),
            Self::Overlay(_) => {}
        }
    }

    fn collect_styled_container_order(&self, order: &mut Vec<NodeId>) {
        match self {
            Self::Container(container) => {
                if container.style.is_some() && container.hoverable {
                    order.push(container.id);
                }
                for child in &container.children {
                    child.child.collect_styled_container_order(order);
                }
            }
            Self::Widget(_) => {}
            Self::Overlay(_) => {}
        }
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn widget_paint_order(&self) -> Vec<WidgetId> {
        let mut order = Vec::new();
        self.root.collect_widget_paint_order(&mut order);
        order
    }

    pub(in crate::runtime) fn styled_container_order(&self) -> Vec<NodeId> {
        let mut order = Vec::new();
        self.root.collect_styled_container_order(&mut order);
        order
    }
}
