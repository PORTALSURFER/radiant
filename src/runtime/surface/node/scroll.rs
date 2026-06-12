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
}

impl<Message> SurfaceContainer<Message> {
    fn scroll_message(&self, update: ScrollUpdate) -> Option<Message> {
        if self.id == update.node_id
            && let Some(message) = &self.scroll_message
        {
            return Some(message(update));
        }
        self.children
            .iter()
            .find_map(|child| child.child.scroll_message(update))
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
}
