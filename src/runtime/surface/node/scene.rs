use super::{LayerKind, SurfaceLayer, SurfaceLayerChildKind, SurfaceNode};
use crate::layout::NodeId;

/// A root scene with base content plus typed transient layers.
pub struct SurfaceScene<Message> {
    pub(in crate::runtime::surface) id: NodeId,
    pub(in crate::runtime::surface) base: Box<SurfaceNode<Message>>,
    pub(in crate::runtime::surface) layers: Vec<SurfaceLayer<Message>>,
}

impl<Message> SurfaceScene<Message> {
    /// Build a surface scene.
    pub fn new(id: NodeId, base: SurfaceNode<Message>, layers: Vec<SurfaceLayer<Message>>) -> Self {
        Self {
            id,
            base: Box::new(base),
            layers,
        }
    }

    pub(in crate::runtime) fn ordered_layers(
        &self,
    ) -> impl Iterator<Item = &SurfaceLayer<Message>> {
        self.ordered_layer_indices()
            .map(|layer_index| &self.layers[layer_index])
    }

    pub(in crate::runtime) fn has_layers(&self) -> bool {
        !self.layers.is_empty()
    }

    pub(in crate::runtime) fn ordered_layer_indices(&self) -> impl Iterator<Item = usize> + '_ {
        LayerKind::ORDER.into_iter().flat_map(|kind| {
            self.layers
                .iter()
                .enumerate()
                .filter_map(move |(index, layer)| (layer.kind == kind).then_some(index))
        })
    }

    pub(in crate::runtime) fn ordered_layer_child_for_child(
        &self,
        child_index: usize,
    ) -> Option<(usize, SurfaceLayerChildKind)> {
        let mut remaining = child_index;
        for layer_index in self.ordered_layer_indices() {
            let layer = &self.layers[layer_index];
            if layer.input.is_some() {
                if remaining == 0 {
                    return Some((layer_index, SurfaceLayerChildKind::Input));
                }
                remaining -= 1;
            }
            if remaining == 0 {
                return Some((layer_index, SurfaceLayerChildKind::Foreground));
            }
            remaining -= 1;
        }
        None
    }

    pub(in crate::runtime) fn ordered_layer_child_count(&self) -> usize {
        self.layers.iter().map(SurfaceLayer::child_count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ContainerPolicy;

    #[test]
    fn ordered_layer_indices_group_by_layer_kind_order() {
        let scene = SurfaceScene::new(
            1,
            SurfaceNode::<()>::container(2, ContainerPolicy::default(), Vec::new()),
            vec![
                SurfaceLayer::new(
                    LayerKind::Tooltip,
                    SurfaceNode::container(3, ContainerPolicy::default(), Vec::new()),
                ),
                SurfaceLayer::new(
                    LayerKind::Floating,
                    SurfaceNode::container(4, ContainerPolicy::default(), Vec::new()),
                ),
                SurfaceLayer::new(
                    LayerKind::Modal,
                    SurfaceNode::container(5, ContainerPolicy::default(), Vec::new()),
                ),
            ],
        );

        assert_eq!(
            scene.ordered_layer_indices().collect::<Vec<_>>(),
            vec![1, 2, 0]
        );
    }

    #[test]
    fn ordered_layer_child_for_child_counts_input_before_foreground() {
        let input = SurfaceNode::<()>::container(10, ContainerPolicy::default(), Vec::new());
        let foreground = SurfaceNode::<()>::container(11, ContainerPolicy::default(), Vec::new());
        let scene = SurfaceScene::new(
            1,
            SurfaceNode::container(2, ContainerPolicy::default(), Vec::new()),
            vec![SurfaceLayer::with_input(
                LayerKind::Popover,
                Some(input),
                foreground,
            )],
        );

        assert_eq!(
            scene.ordered_layer_child_for_child(0),
            Some((0, SurfaceLayerChildKind::Input))
        );
        assert_eq!(
            scene.ordered_layer_child_for_child(1),
            Some((0, SurfaceLayerChildKind::Foreground))
        );
        assert_eq!(scene.ordered_layer_child_for_child(2), None);
        assert_eq!(scene.ordered_layer_child_count(), 2);
    }
}
