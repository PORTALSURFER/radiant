use super::{SurfaceContainer, SurfaceNode, SurfaceScene};
use crate::runtime::ScrollUpdate;

impl<Message> SurfaceNode<Message> {
    pub(in crate::runtime) fn scroll_edit_source(
        &self,
        node_id: crate::layout::NodeId,
    ) -> Option<crate::runtime::FrozenSourceMetadata> {
        if self.id() == node_id {
            return self
                .source_metadata_handle()
                .as_deref()
                .map(super::super::SourceMetadata::freeze);
        }
        match self {
            Self::Scene(scene) => scene.base.scroll_edit_source(node_id).or_else(|| {
                scene.ordered_layers().find_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .and_then(|input| input.scroll_edit_source(node_id))
                        .or_else(|| layer.node.scroll_edit_source(node_id))
                })
            }),
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.scroll_edit_source(node_id)),
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .find_map(|child| child.child.scroll_edit_source(node_id)),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }

    pub(in crate::runtime) fn scroll_edit_mapper(
        &self,
        node_id: crate::layout::NodeId,
    ) -> Option<crate::runtime::ScrollEditMessageMapper<Message>> {
        match self {
            Self::Scene(scene) => scene.base.scroll_edit_mapper(node_id).or_else(|| {
                scene.ordered_layers().find_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .and_then(|input| input.scroll_edit_mapper(node_id))
                        .or_else(|| layer.node.scroll_edit_mapper(node_id))
                })
            }),
            Self::Container(container) => container.scroll_edit_mapper(node_id),
            Self::FloatingLayer(layer) => layer.container.scroll_edit_mapper(node_id),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }

    pub(in crate::runtime) fn scroll_edit_message(
        &self,
        batch: crate::runtime::ScrollEditBatch,
    ) -> Option<Message> {
        match self {
            Self::Scene(scene) => scene.scroll_edit_message(batch),
            Self::Container(container) => container.scroll_edit_message(batch),
            Self::FloatingLayer(layer) => layer.container.scroll_edit_message(batch),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }

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
    fn scroll_edit_mapper(
        &self,
        node_id: crate::layout::NodeId,
    ) -> Option<crate::runtime::ScrollEditMessageMapper<Message>> {
        if self.id == node_id {
            return self.scroll_edit.clone();
        }
        self.children
            .iter()
            .find_map(|child| child.child.scroll_edit_mapper(node_id))
    }

    fn scroll_edit_message(&self, batch: crate::runtime::ScrollEditBatch) -> Option<Message> {
        if self.id == batch.node_id()
            && let Some(map) = &self.scroll_edit
        {
            return map(batch);
        }
        self.children
            .iter()
            .find_map(|child| child.child.scroll_edit_message(batch))
    }

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
    fn scroll_edit_message(&self, batch: crate::runtime::ScrollEditBatch) -> Option<Message> {
        self.base.scroll_edit_message(batch).or_else(|| {
            self.ordered_layers().find_map(|layer| {
                layer
                    .input
                    .as_ref()
                    .and_then(|input| input.scroll_edit_message(batch))
                    .or_else(|| layer.node.scroll_edit_message(batch))
            })
        })
    }

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
