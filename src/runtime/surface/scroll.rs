use super::*;
use crate::layout::ContainerKind;

impl<Message> SurfaceNode<Message> {
    fn collect_scroll_container_order(&self, order: &mut Vec<NodeId>) {
        match self {
            Self::Container(container) => {
                if container.policy.kind == ContainerKind::ScrollView {
                    order.push(container.id);
                }
                for child in &container.children {
                    child.child.collect_scroll_container_order(order);
                }
            }
            Self::Widget(_) => {}
            Self::Overlay(_) => {}
        }
    }

    fn scroll_content_id(&self, scroll_id: NodeId) -> Option<NodeId> {
        match self {
            Self::Container(container) => {
                if container.id == scroll_id && container.policy.kind == ContainerKind::ScrollView {
                    return container.children.first().map(|child| child.child.id());
                }
                container
                    .children
                    .iter()
                    .find_map(|child| child.child.scroll_content_id(scroll_id))
            }
            Self::Widget(_) => None,
            Self::Overlay(_) => None,
        }
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn scroll_container_order(&self) -> Vec<NodeId> {
        let mut order = Vec::new();
        self.root.collect_scroll_container_order(&mut order);
        order
    }

    pub(in crate::runtime) fn scroll_content_id(&self, scroll_id: NodeId) -> Option<NodeId> {
        self.root.scroll_content_id(scroll_id)
    }
}
