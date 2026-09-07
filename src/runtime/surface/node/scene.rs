use super::super::source::SourceMetadata;
use super::{LayerKind, SurfaceLayer, SurfaceLayerChildKind, SurfaceNode};
use crate::{layout::NodeId, UiAffinity};
use std::rc::Rc;

/// A root scene with base content plus typed transient layers.
pub struct SurfaceScene<Message> {
    pub(in crate::runtime::surface) _ui_affinity: UiAffinity,
    pub(in crate::runtime::surface) id: NodeId,
    pub(in crate::runtime::surface) has_animation: bool,
    pub(in crate::runtime::surface) base: Box<SurfaceNode<Message>>,
    pub(in crate::runtime::surface) layers: Vec<SurfaceLayer<Message>>,
    overlay_order: Option<Vec<usize>>,
    pub(in crate::runtime::surface) has_resource_view_demand: bool,
    pub(in crate::runtime::surface) has_notice_demand: bool,
    pub(in crate::runtime::surface) source: Option<Rc<SourceMetadata>>,
    pub(in crate::runtime::surface) command_scope:
        Option<crate::application::CommandScopeAttachment>,
}

impl<Message> SurfaceScene<Message> {
    /// Build a surface scene.
    pub fn new(id: NodeId, base: SurfaceNode<Message>, layers: Vec<SurfaceLayer<Message>>) -> Self {
        let has_animation = base.has_animation()
            || layers.iter().any(|l| {
                l.node.has_animation() || l.input.as_ref().is_some_and(SurfaceNode::has_animation)
            });
        let has_resource_view_demand = Self::has_resource_view_demand_in(&base, &layers);
        let has_notice_demand = Self::has_notice_demand_in(&base, &layers);
        let overlay_order = ordered_nested_overlay_indices(&layers);
        Self {
            has_animation,
            _ui_affinity: UiAffinity::new(),
            id,
            base: Box::new(base),
            layers,
            overlay_order,
            has_resource_view_demand,
            has_notice_demand,
            source: None,
            command_scope: None,
        }
    }

    fn has_resource_view_demand_in(
        base: &SurfaceNode<Message>,
        layers: &[SurfaceLayer<Message>],
    ) -> bool {
        base.has_resource_view_demand()
            || layers.iter().any(|layer| {
                layer
                    .input
                    .as_ref()
                    .is_some_and(SurfaceNode::has_resource_view_demand)
                    || layer.node.has_resource_view_demand()
            })
    }

    pub(in crate::runtime::surface) fn refresh_resource_view_demand(&mut self) {
        self.has_resource_view_demand = Self::has_resource_view_demand_in(&self.base, &self.layers);
    }

    fn has_notice_demand_in(base: &SurfaceNode<Message>, layers: &[SurfaceLayer<Message>]) -> bool {
        base.has_notice_demand()
            || layers.iter().any(|layer| {
                layer
                    .input
                    .as_ref()
                    .is_some_and(SurfaceNode::has_notice_demand)
                    || layer.node.has_notice_demand()
            })
    }

    pub(in crate::runtime::surface) fn refresh_notice_demand(&mut self) {
        self.has_notice_demand = Self::has_notice_demand_in(&self.base, &self.layers);
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

    pub(in crate::runtime) fn ordered_layer_indices(&self) -> OrderedLayerIndices<'_, Message> {
        match self.overlay_order.as_deref() {
            Some(indices) => OrderedLayerIndices::Nested(indices.iter()),
            None => OrderedLayerIndices::Kind {
                layers: &self.layers,
                kind: 0,
                index: 0,
            },
        }
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

pub(in crate::runtime) enum OrderedLayerIndices<'a, Message> {
    Nested(std::slice::Iter<'a, usize>),
    Kind {
        layers: &'a [SurfaceLayer<Message>],
        kind: usize,
        index: usize,
    },
}

impl<Message> Iterator for OrderedLayerIndices<'_, Message> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Nested(indices) => indices.next().copied(),
            Self::Kind {
                layers,
                kind,
                index,
            } => loop {
                let current = *index;
                *index += 1;
                if current >= layers.len() {
                    *kind += 1;
                    *index = 0;
                    if *kind >= LayerKind::ORDER.len() {
                        return None;
                    }
                    continue;
                }
                if layers[current].kind == LayerKind::ORDER[*kind] {
                    return Some(current);
                }
            },
        }
    }
}

fn ordered_nested_overlay_indices<Message>(layers: &[SurfaceLayer<Message>]) -> Option<Vec<usize>> {
    if layers.is_empty() || layers.len() > 64 {
        return None;
    }
    let mut overlays = Vec::with_capacity(layers.len());
    let mut parents = Vec::with_capacity(layers.len());
    for layer in layers {
        let metadata = layer.node.source_metadata_handle()?;
        let evidence = metadata.topology.overlays.as_slice();
        let final_evidence = evidence.last()?;
        if final_evidence.layer_kind != layer.kind
            || overlays
                .iter()
                .any(|overlay| overlay.identity == final_evidence.identity)
        {
            return None;
        }
        for (index, candidate) in evidence.iter().enumerate() {
            if evidence[..index]
                .iter()
                .any(|previous| previous.identity == candidate.identity)
            {
                return None;
            }
        }
        overlays.push(*final_evidence);
        parents.push(
            evidence
                .len()
                .checked_sub(2)
                .and_then(|index| evidence.get(index))
                .copied(),
        );
    }
    let mut parent_indices = Vec::with_capacity(layers.len());
    for parent in parents {
        let parent_index = match parent {
            None => None,
            Some(parent) => Some(overlays.iter().position(|overlay| *overlay == parent)?),
        };
        parent_indices.push(parent_index);
    }
    let mut ordered = Vec::with_capacity(layers.len());
    while ordered.len() < layers.len() {
        let candidate = (0..layers.len())
            .filter(|index| !ordered.contains(index))
            .filter(|index| parent_indices[*index].is_none_or(|parent| ordered.contains(&parent)))
            .min_by_key(|index| (layers[*index].kind.z_order(), *index));
        ordered.push(candidate?);
    }
    Some(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{overlays, scene, text, IntoView, ResourceInterestKind, SharedResourceTasks},
        layout::ContainerPolicy,
    };
    use std::rc::Rc;

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
    fn nested_declarative_popover_renders_after_its_modal_parent() {
        let surface =
            scene(text::<()>("base").overlays(
                overlays().modal(
                    text("modal parent").overlays(overlays().popover(text("popover child"))),
                ),
            ))
            .into_view()
            .into_surface();
        let SurfaceNode::Scene(scene) = surface.root() else {
            panic!("declarative scene should lower to a surface scene");
        };

        let parent = scene
            .layers
            .iter()
            .position(|layer| {
                layer.kind == LayerKind::Modal
                    && layer
                        .node
                        .source_metadata_handle()
                        .is_some_and(|metadata| metadata.topology.overlays.len() == 1)
            })
            .expect("modal parent layer");
        let child = scene
            .layers
            .iter()
            .position(|layer| {
                layer.kind == LayerKind::Popover
                    && layer
                        .node
                        .source_metadata_handle()
                        .is_some_and(|metadata| metadata.topology.overlays.len() == 2)
            })
            .expect("popover child layer");

        assert!(child < parent, "nested layers are extracted inner-first");
        let ordered = scene.ordered_layer_indices().collect::<Vec<_>>();
        let parent_position = ordered
            .iter()
            .position(|index| *index == parent)
            .expect("ordered modal parent");
        let child_position = ordered
            .iter()
            .position(|index| *index == child)
            .expect("ordered popover child");
        assert!(
            parent_position < child_position,
            "the modal parent must paint before its popover child"
        );
    }

    #[test]
    fn nested_declarative_same_kind_parent_renders_before_child() {
        let surface = scene(text::<()>("base").overlays(
            overlays().modal(text("outer modal").overlays(overlays().modal(text("inner modal")))),
        ))
        .into_view()
        .into_surface();
        let SurfaceNode::Scene(scene) = surface.root() else {
            panic!("declarative scene should lower to a surface scene");
        };

        let parent = scene
            .layers
            .iter()
            .position(|layer| {
                layer
                    .node
                    .source_metadata_handle()
                    .is_some_and(|metadata| metadata.topology.overlays.len() == 1)
            })
            .expect("outer modal layer");
        let child = scene
            .layers
            .iter()
            .position(|layer| {
                layer
                    .node
                    .source_metadata_handle()
                    .is_some_and(|metadata| metadata.topology.overlays.len() == 2)
            })
            .expect("inner modal layer");

        assert!(child < parent, "nested layers are extracted inner-first");
        assert_eq!(
            scene.ordered_layer_indices().collect::<Vec<_>>(),
            [parent, child]
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

    #[test]
    fn nested_scene_caches_resource_view_demand() {
        let demand = Rc::new(
            crate::application::resource_view::demand::ResourceViewDemand {
                tasks: SharedResourceTasks::new(),
                key: crate::runtime::ResourceKey::scoped("resource-view", "scene"),
                kind: ResourceInterestKind::Visible,
                interest_id: 1,
            },
        );
        let leaf = SurfaceNode::<()>::container(3, ContainerPolicy::default(), Vec::new())
            .with_resource_view_demand(Some(demand));
        let nested = SurfaceNode::scene(2, leaf, Vec::new());
        let scene = SurfaceScene::new(1, nested, Vec::new());

        assert!(scene.has_resource_view_demand);
        assert!(SurfaceNode::Scene(scene.clone()).has_resource_view_demand());
    }
}
