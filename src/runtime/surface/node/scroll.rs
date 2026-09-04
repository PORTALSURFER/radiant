use super::{SurfaceContainer, SurfaceNode, SurfaceScene};
use crate::runtime::ScrollUpdate;

impl<Message> SurfaceNode<Message> {
    pub(in crate::runtime) fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        match self {
            Self::Scene(scene) => scene.scroll_message(update),
            Self::Container(container) => container.scroll_message(update),
            Self::FloatingLayer(layer) => layer.container.scroll_message(update),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }

    pub(in crate::runtime) fn offset_settled(
        &self,
        node_id: crate::layout::NodeId,
        offset: crate::gui::types::Vector2,
    ) -> Option<Message> {
        match self {
            Self::Scene(scene) => scene.offset_settled(node_id, offset),
            Self::Container(container) => container.offset_settled(node_id, offset),
            Self::FloatingLayer(layer) => layer.container.offset_settled(node_id, offset),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }
}

impl<Message> SurfaceContainer<Message> {
    fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        if self.id == update.node_id
            && let Some(message) = &self.scroll_message
        {
            return message.invoke(update);
        }
        self.children
            .iter()
            .find_map(|child| child.child.scroll_message(update))
    }

    fn offset_settled(
        &self,
        node_id: crate::layout::NodeId,
        offset: crate::gui::types::Vector2,
    ) -> Option<Message> {
        if self.id == node_id {
            return self.offset_settled.as_ref().map(|map| map(offset));
        }
        self.children
            .iter()
            .find_map(|child| child.child.offset_settled(node_id, offset))
    }
}

impl<Message> SurfaceScene<Message> {
    fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        self.base.scroll_message(update).or_else(|| {
            self.ordered_layers().find_map(|layer| {
                layer
                    .input
                    .as_ref()
                    .and_then(|input| input.scroll_message(update))
                    .or_else(|| layer.node.scroll_message(update))
            })
        })
    }

    fn offset_settled(
        &self,
        node_id: crate::layout::NodeId,
        offset: crate::gui::types::Vector2,
    ) -> Option<Message> {
        self.base.offset_settled(node_id, offset).or_else(|| {
            self.ordered_layers().find_map(|layer| {
                layer
                    .input
                    .as_ref()
                    .and_then(|input| input.offset_settled(node_id, offset))
                    .or_else(|| layer.node.offset_settled(node_id, offset))
            })
        })
    }
}
