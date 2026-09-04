#![allow(dead_code)]

use super::{LayerKind, SurfaceNode};
use crate::{
    application::{
        DeclarativeEffectOwner, DeclarativeIdentityOrigin, DeclarativeOverlaySource,
        DeclarativeSourceContext, SourceIdentitySeed,
    },
    layout::NodeId,
};
use std::{cell::Cell, rc::Rc};

/// Coarse runtime compatibility evidence retained beside declarative source
/// identity.  This intentionally excludes callbacks, mappers, captured
/// values, and widget behavior.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum SurfaceSourceKind {
    Unknown,
    Scene,
    Container,
    Widget,
    Overlay,
    FloatingLayer,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SourceCompatibility {
    pub(crate) surface_kind: SurfaceSourceKind,
    pub(crate) widget_compatibility_kind: Option<&'static str>,
}

impl SourceCompatibility {
    fn unknown() -> Self {
        Self {
            surface_kind: SurfaceSourceKind::Unknown,
            widget_compatibility_kind: None,
        }
    }

    pub(crate) fn from_surface_node<Message>(node: &SurfaceNode<Message>) -> Self {
        match node {
            SurfaceNode::Scene(_) => Self {
                surface_kind: SurfaceSourceKind::Scene,
                widget_compatibility_kind: None,
            },
            SurfaceNode::Container(_) => Self {
                surface_kind: SurfaceSourceKind::Container,
                widget_compatibility_kind: None,
            },
            SurfaceNode::Widget(widget) => Self {
                surface_kind: SurfaceSourceKind::Widget,
                widget_compatibility_kind: Some(widget.compatibility_kind()),
            },
            SurfaceNode::Overlay(_) => Self {
                surface_kind: SurfaceSourceKind::Overlay,
                widget_compatibility_kind: None,
            },
            SurfaceNode::FloatingLayer(_) => Self {
                surface_kind: SurfaceSourceKind::FloatingLayer,
                widget_compatibility_kind: None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SourceIdentity {
    pub(crate) resolved_id: NodeId,
    pub(crate) structural_scope: NodeId,
    pub(crate) origin: DeclarativeIdentityOrigin,
}

impl From<SourceIdentitySeed> for SourceIdentity {
    fn from(seed: SourceIdentitySeed) -> Self {
        Self {
            resolved_id: seed.resolved_id,
            structural_scope: seed.structural_scope,
            origin: seed.origin,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct OverlayIdentity {
    pub(crate) structural_scope: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OverlayEvidence {
    pub(crate) identity: OverlayIdentity,
    pub(crate) layer_kind: LayerKind,
    pub(crate) effect_owner: Option<DeclarativeEffectOwner>,
}

impl From<DeclarativeOverlaySource> for OverlayEvidence {
    fn from(source: DeclarativeOverlaySource) -> Self {
        Self {
            identity: OverlayIdentity {
                structural_scope: source.identity_scope,
            },
            layer_kind: source.layer_kind,
            effect_owner: source.effect_owner,
        }
    }
}

/// A shared keyed-node candidate.  Descendant metadata points at the same
/// record, so compatibility remains one independently observable fact while
/// keyed ancestry remains fully preserved under nesting.
#[derive(Debug)]
pub(crate) struct KeyedNodeEvidence {
    identity: Cell<SourceIdentity>,
    compatibility: Cell<SourceCompatibility>,
    effect_owner: Cell<Option<DeclarativeEffectOwner>>,
}

impl KeyedNodeEvidence {
    pub(crate) fn new(seed: SourceIdentitySeed) -> Self {
        Self {
            identity: Cell::new(seed.into()),
            compatibility: Cell::new(SourceCompatibility::unknown()),
            effect_owner: Cell::new(seed.effect_owner),
        }
    }

    pub(crate) fn identity(&self) -> SourceIdentity {
        self.identity.get()
    }

    pub(crate) fn set_identity(&self, identity: SourceIdentity) {
        self.identity.set(identity);
    }

    pub(crate) fn compatibility(&self) -> SourceCompatibility {
        self.compatibility.get()
    }

    pub(crate) fn set_compatibility(&self, compatibility: SourceCompatibility) {
        self.compatibility.set(compatibility);
    }

    pub(crate) fn effect_owner(&self) -> Option<DeclarativeEffectOwner> {
        self.effect_owner.get()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SourceTopology {
    pub(crate) keyed_nodes: Vec<Rc<KeyedNodeEvidence>>,
    pub(crate) overlays: Vec<OverlayEvidence>,
}

impl SourceTopology {
    pub(crate) fn from_context(
        context: &DeclarativeSourceContext,
        mut keyed_candidate: impl FnMut(SourceIdentitySeed) -> Rc<KeyedNodeEvidence>,
    ) -> Self {
        Self {
            keyed_nodes: context
                .keyed_nodes
                .iter()
                .copied()
                .map(&mut keyed_candidate)
                .collect(),
            overlays: context
                .overlays
                .iter()
                .copied()
                .map(OverlayEvidence::from)
                .collect(),
        }
    }
}

/// Complete private source metadata attached to one concrete surface node.
#[derive(Clone, Debug)]
pub(crate) struct SourceMetadata {
    pub(crate) identity: SourceIdentity,
    pub(crate) compatibility: SourceCompatibility,
    pub(crate) topology: SourceTopology,
}

impl SourceMetadata {
    pub(crate) fn new(
        identity: SourceIdentity,
        compatibility: SourceCompatibility,
        topology: SourceTopology,
    ) -> Self {
        Self {
            identity,
            compatibility,
            topology,
        }
    }
}

/// Compare all source evidence retained on one concrete surface node.
///
/// Reconciliation may retain the installed node's owner projections only when
/// the complete source witness is unchanged.  Keep this comparison beside the
/// metadata types so path-local admission and fresh-preparation validation
/// cannot drift apart.
pub(crate) fn source_metadata_matches(first: &SourceMetadata, second: &SourceMetadata) -> bool {
    first.identity == second.identity
        && first.compatibility == second.compatibility
        && source_topology_matches(&first.topology, &second.topology)
}

fn source_topology_matches(first: &SourceTopology, second: &SourceTopology) -> bool {
    first.keyed_nodes.len() == second.keyed_nodes.len()
        && first
            .keyed_nodes
            .iter()
            .zip(&second.keyed_nodes)
            .all(|(first, second)| {
                first.identity() == second.identity()
                    && first.compatibility() == second.compatibility()
                    && first.effect_owner() == second.effect_owner()
            })
        && first.overlays == second.overlays
}

#[derive(Clone, Debug)]
pub(crate) struct SourceTraversalRecord {
    pub(crate) node_id: NodeId,
    pub(crate) metadata: Option<Rc<SourceMetadata>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SourceTraversalIndex {
    pub(crate) records: Vec<SourceTraversalRecord>,
}

impl SourceTraversalIndex {
    pub(crate) fn with_stats(stats: super::traversal::SurfaceTraversalStats) -> Self {
        Self {
            records: Vec::with_capacity(stats.source_nodes),
        }
    }

    pub(crate) fn clear_for_stats(&mut self, stats: super::traversal::SurfaceTraversalStats) {
        self.records.clear();
        if self.records.capacity() < stats.source_nodes {
            self.records
                .reserve(stats.source_nodes - self.records.capacity());
        }
    }

    pub(crate) fn clear_for_reuse(&mut self) {
        self.records.clear();
    }

    pub(crate) fn record_node<Message>(&mut self, node: &SurfaceNode<Message>) {
        self.records.push(SourceTraversalRecord {
            node_id: node.id(),
            metadata: node.source_metadata_handle(),
        });
    }

    pub(crate) fn capacity(&self) -> usize {
        self.records.capacity()
    }
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn collect_source_traversal(&self, source: &mut SourceTraversalIndex) {
        source.record_node(self);
        match self {
            Self::Scene(scene) => {
                scene.base.collect_source_traversal(source);
                for layer in scene.ordered_layers() {
                    if let Some(input) = &layer.input {
                        input.collect_source_traversal(source);
                    }
                    layer.node.collect_source_traversal(source);
                }
            }
            Self::Container(container) => {
                for child in &container.children {
                    child.child.collect_source_traversal(source);
                }
            }
            Self::Widget(_) | Self::Overlay(_) => {}
            Self::FloatingLayer(layer) => {
                for child in &layer.container.children {
                    child.child.collect_source_traversal(source);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            DeclarativeEffectOwner, IntoView, Layer, column, for_each_by,
            lower_virtual_layout_item, overlays, scene, text,
        },
        layout::ContainerPolicy,
        runtime::{EventMapper, SurfaceNode, UiSurface},
    };
    use std::{collections::HashSet, rc::Rc};

    fn metadata(surface: &UiSurface<()>) -> Vec<Rc<SourceMetadata>> {
        surface
            .runtime_source_traversal_index()
            .records
            .into_iter()
            .filter_map(|record| record.metadata)
            .collect()
    }

    fn metadata_records(surface: &UiSurface<()>) -> Vec<SourceTraversalRecord> {
        surface.runtime_source_traversal_index().records
    }

    fn inferred_surface(order: &[u32]) -> UiSurface<()> {
        column(for_each_by(
            order.iter().copied(),
            |item| *item,
            |item| text::<()>(format!("item-{item}")),
        ))
        .into_surface()
    }

    #[test]
    fn source_traversal_reuse_clears_records_and_retains_capacity() {
        let stats = super::super::traversal::SurfaceTraversalStats {
            source_nodes: 4,
            ..Default::default()
        };
        let mut index = SourceTraversalIndex::with_stats(stats);
        index
            .records
            .extend((0..4).map(|node_id| SourceTraversalRecord {
                node_id,
                metadata: None,
            }));
        let capacity = index.capacity();

        index.clear_for_reuse();

        assert!(index.records.is_empty());
        assert_eq!(index.capacity(), capacity);
    }

    #[test]
    fn raw_public_surface_nodes_are_metadata_free() {
        let node = SurfaceNode::<()>::container(1, ContainerPolicy::default(), Vec::new());
        assert!(node.source_metadata_handle().is_none());

        let surface = UiSurface::new(node);
        let source = surface.runtime_source_traversal_index();
        assert!(
            source
                .records
                .iter()
                .all(|record| record.metadata.is_none())
        );
    }

    #[test]
    fn identity_origins_keep_only_keyed_forms_as_candidates() {
        let surface = column([
            text::<()>("generated"),
            text::<()>("numeric").id(17),
            text::<()>("explicit").key("explicit"),
        ])
        .into_surface();
        let records = metadata(&surface);

        assert!(records.iter().any(|record| {
            record.identity.origin == DeclarativeIdentityOrigin::GeneratedStructural
                && record.topology.keyed_nodes.is_empty()
        }));
        let numeric = records
            .iter()
            .find(|record| record.identity.resolved_id == 17)
            .expect("numeric identity should be retained");
        assert_eq!(
            numeric.identity.origin,
            DeclarativeIdentityOrigin::ExplicitNumericId
        );
        assert!(numeric.topology.keyed_nodes.is_empty());

        let explicit = records
            .iter()
            .find(|record| {
                record.identity.origin == DeclarativeIdentityOrigin::ExplicitContinuityKey
            })
            .expect("explicit continuity key should be retained");
        assert_eq!(explicit.topology.keyed_nodes.len(), 1);
        assert_eq!(
            explicit.compatibility.surface_kind,
            SurfaceSourceKind::Widget
        );
        assert_eq!(
            explicit.topology.keyed_nodes[0]
                .compatibility()
                .surface_kind,
            SurfaceSourceKind::Widget
        );

        let inferred = metadata(&inferred_surface(&[7]))
            .into_iter()
            .find(|record| {
                record.identity.origin == DeclarativeIdentityOrigin::InferredKeyedIdentity
            })
            .expect("inferred keyed identity should be retained");
        assert_eq!(inferred.topology.keyed_nodes.len(), 1);

        let direct_runtime: crate::application::ViewNode<()> =
            SurfaceNode::container(91, ContainerPolicy::default(), Vec::new()).into();
        let direct_runtime = metadata(&direct_runtime.into_surface())
            .into_iter()
            .next()
            .expect("direct runtime root metadata");
        assert_eq!(
            direct_runtime.identity.origin,
            DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot
        );
    }

    #[test]
    fn explicit_effect_owner_markers_follow_keyed_and_overlay_topology_only() {
        let keyed_owner = DeclarativeEffectOwner::new();
        let overlay_owner = DeclarativeEffectOwner::new();
        let ineligible_owner = DeclarativeEffectOwner::new();
        let surface = scene(
            text::<()>("keyed")
                .key("keyed")
                .effect_owner(keyed_owner)
                .overlays(
                    overlays().layer(Layer::modal(text("overlay")).effect_owner(overlay_owner)),
                ),
        )
        .into_view()
        .into_surface();

        let records = metadata(&surface);
        let keyed = records
            .iter()
            .find(|record| record.identity.origin.is_keyed())
            .expect("keyed source metadata");
        assert_eq!(keyed.topology.keyed_nodes.len(), 1);
        assert_eq!(
            keyed.topology.keyed_nodes[0].effect_owner(),
            Some(keyed_owner)
        );
        let overlay = records
            .iter()
            .find(|record| {
                record
                    .topology
                    .overlays
                    .iter()
                    .any(|candidate| candidate.effect_owner == Some(overlay_owner))
            })
            .expect("overlay source metadata");
        assert_eq!(overlay.topology.overlays.len(), 1);
        assert_eq!(
            overlay.topology.overlays[0].effect_owner,
            Some(overlay_owner)
        );

        let unkeyed = text::<()>("unkeyed")
            .effect_owner(ineligible_owner)
            .into_surface();
        assert!(metadata(&unkeyed).iter().all(|record| {
            record
                .topology
                .keyed_nodes
                .iter()
                .all(|candidate| candidate.effect_owner() != Some(ineligible_owner))
        }));
    }

    #[test]
    fn inferred_keyed_source_scopes_follow_items_across_sibling_reorder() {
        let before = metadata(&inferred_surface(&[1, 2, 3]))
            .into_iter()
            .filter(|record| {
                record.identity.origin == DeclarativeIdentityOrigin::InferredKeyedIdentity
            })
            .map(|record| {
                (
                    record.identity.resolved_id,
                    record.identity.structural_scope,
                )
            })
            .collect::<Vec<_>>();
        let after = metadata(&inferred_surface(&[3, 2, 1]))
            .into_iter()
            .filter(|record| {
                record.identity.origin == DeclarativeIdentityOrigin::InferredKeyedIdentity
            })
            .map(|record| {
                (
                    record.identity.resolved_id,
                    record.identity.structural_scope,
                )
            })
            .collect::<Vec<_>>();

        assert_ne!(
            before, after,
            "resolved keyed-node order should follow the reorder"
        );
        let before_scopes = before
            .iter()
            .map(|(_, scope)| *scope)
            .collect::<HashSet<_>>();
        let after_scopes = after
            .iter()
            .map(|(_, scope)| *scope)
            .collect::<HashSet<_>>();
        assert_eq!(before_scopes, after_scopes);
        assert_eq!(
            before.iter().map(|(_, scope)| *scope).collect::<Vec<_>>(),
            after
                .iter()
                .rev()
                .map(|(_, scope)| *scope)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn view_local_overlay_retains_keyed_ancestry_after_flattening() {
        let surface = scene(column(for_each_by(
            [1_u32],
            |item| *item,
            |_| text::<()>("row").overlays(overlays().modal(text("overlay"))),
        )))
        .into_view()
        .into_surface();
        let overlay = metadata(&surface)
            .into_iter()
            .find(|record| !record.topology.overlays.is_empty())
            .expect("flattened view-local overlay should retain its descriptor");

        assert_eq!(overlay.topology.overlays.len(), 1);
        assert_eq!(overlay.topology.keyed_nodes.len(), 1);
        assert_eq!(
            overlay.topology.keyed_nodes[0].identity().origin,
            DeclarativeIdentityOrigin::InferredKeyedIdentity
        );
    }

    #[test]
    fn flattened_local_overlay_keeps_original_scope_when_an_earlier_overlay_is_added() {
        fn surface(with_earlier_overlay: bool) -> UiSurface<()> {
            let target = text::<()>("target")
                .key("target-owner")
                .overlays(overlays().modal(text("target-overlay").key("target-overlay")));
            let base = if with_earlier_overlay {
                column([
                    text::<()>("earlier").overlays(overlays().modal(text("earlier-overlay"))),
                    target,
                ])
            } else {
                column([target])
            };
            scene(base).into_view().into_surface()
        }

        let before = metadata_records(&surface(false))
            .into_iter()
            .find(|record| {
                record.metadata.as_ref().is_some_and(|metadata| {
                    metadata.identity.origin == DeclarativeIdentityOrigin::ExplicitContinuityKey
                        && metadata.topology.keyed_nodes.len() == 2
                        && metadata.topology.overlays.len() == 1
                })
            })
            .expect("target overlay metadata before unrelated insertion");
        let after = metadata_records(&surface(true))
            .into_iter()
            .find(|record| {
                record.metadata.as_ref().is_some_and(|metadata| {
                    metadata.identity.origin == DeclarativeIdentityOrigin::ExplicitContinuityKey
                        && metadata.topology.keyed_nodes.len() == 2
                        && metadata.topology.overlays.len() == 1
                })
            })
            .expect("target overlay metadata after unrelated insertion");

        let before_identity = before.metadata.expect("before target metadata").identity;
        let after_identity = after.metadata.expect("after target metadata").identity;
        assert_eq!(
            before_identity.structural_scope,
            after_identity.structural_scope
        );
        assert_eq!(before_identity.resolved_id, before.node_id);
        assert_eq!(after_identity.resolved_id, after.node_id);
    }

    #[test]
    fn synthesized_layer_input_and_foreground_share_overlay_evidence() {
        let surface = scene::<()>(text("base"))
            .layer(Layer::modal(text("foreground")).block_input())
            .into_view()
            .into_surface();
        let overlay_records = metadata_records(&surface)
            .into_iter()
            .filter_map(|record| {
                let metadata = record.metadata?;
                (!metadata.topology.overlays.is_empty()).then_some((record.node_id, metadata))
            })
            .collect::<Vec<_>>();

        assert_eq!(
            overlay_records.len(),
            2,
            "input and foreground should both be recorded"
        );
        assert_eq!(
            overlay_records[0].1.topology.overlays,
            overlay_records[1].1.topology.overlays
        );
        assert_eq!(
            overlay_records[0].1.topology.overlays[0].layer_kind,
            crate::runtime::LayerKind::Modal
        );
        assert!(
            overlay_records
                .iter()
                .all(|(node_id, metadata)| { metadata.identity.resolved_id == *node_id })
        );
    }

    #[test]
    fn nested_keyed_overlay_topology_keeps_each_candidate_without_a_winner() {
        let surface = scene(column(for_each_by(
            [1_u32],
            |item| *item,
            |_| {
                crate::application::row([text::<()>("overlay-root")
                    .key("overlay-root")
                    .overlays(overlays().modal(text("nested").key("nested")))])
            },
        )))
        .into_view()
        .into_surface();
        let nested = metadata(&surface)
            .into_iter()
            .find(|record| {
                record
                    .topology
                    .overlays
                    .iter()
                    .any(|overlay| overlay.layer_kind == crate::runtime::LayerKind::Modal)
                    && record.topology.keyed_nodes.len() == 3
            })
            .expect("nested overlay should retain all keyed ancestry");

        let origins = nested
            .topology
            .keyed_nodes
            .iter()
            .map(|candidate| candidate.identity().origin)
            .collect::<Vec<_>>();
        assert_eq!(
            origins,
            vec![
                DeclarativeIdentityOrigin::InferredKeyedIdentity,
                DeclarativeIdentityOrigin::ExplicitContinuityKey,
                DeclarativeIdentityOrigin::ExplicitContinuityKey,
            ]
        );
    }

    #[test]
    fn virtual_layout_wrapper_does_not_promote_generation_identity_to_keyed_evidence() {
        let item = for_each_by([7_u32], |item| *item, |_| text::<()>("item"))
            .into_iter()
            .next()
            .expect("one virtual item");
        let lowered = lower_virtual_layout_item(item, 41, 7, 3, 1).expect("item should lower");
        let SurfaceNode::Container(wrapper) = &lowered else {
            panic!("virtual item should retain its private wrapper");
        };
        let child = &wrapper.children[0].child;
        let wrapper_metadata = wrapper.source.clone().expect("wrapper metadata");
        let child_metadata = child.source_metadata_handle().expect("item metadata");

        assert_eq!(
            wrapper_metadata.identity.origin,
            DeclarativeIdentityOrigin::ExplicitNumericId
        );
        assert!(wrapper_metadata.topology.keyed_nodes.is_empty());
        assert_eq!(
            child_metadata.identity.origin,
            DeclarativeIdentityOrigin::InferredKeyedIdentity
        );
        assert_eq!(child_metadata.topology.keyed_nodes.len(), 1);

        let cloned = lowered.clone();
        let SurfaceNode::Container(cloned_wrapper) = &cloned else {
            panic!("cloned virtual item should retain its wrapper");
        };
        assert!(Rc::ptr_eq(
            &wrapper_metadata,
            cloned_wrapper
                .source
                .as_ref()
                .expect("cloned wrapper metadata")
        ));

        let rewritten = lowered.with_native_file_drop_mapped(EventMapper::new(|_| ()));
        let SurfaceNode::Container(rewritten_wrapper) = rewritten else {
            panic!("rewritten virtual item should retain its wrapper");
        };
        assert!(Rc::ptr_eq(
            &wrapper_metadata,
            rewritten_wrapper
                .source
                .as_ref()
                .expect("rewritten wrapper metadata")
        ));
        assert!(
            rewritten_wrapper.children[0]
                .child
                .source_metadata_handle()
                .is_some()
        );
    }

    #[test]
    fn canonical_source_projection_matches_stats_and_reuses_capacity() {
        use crate::runtime::surface::SurfaceTraversalIndex;

        let surface = inferred_surface(&[1, 2, 3]);
        let stats = surface.root.runtime_traversal_stats();
        let mut traversal = SurfaceTraversalIndex::with_stats(stats);
        let mut source = SourceTraversalIndex::with_stats(stats);
        let mut scroll_stack = Vec::new();
        let mut child_path = Vec::new();

        surface.runtime_projection_reusing_with_scratch_and_source(
            &mut traversal,
            &mut scroll_stack,
            &mut child_path,
            &mut source,
        );
        let first_count = source.records.len();
        let first_capacity = source.capacity();
        source.records.push(SourceTraversalRecord {
            node_id: 999,
            metadata: None,
        });

        surface.runtime_projection_reusing_with_scratch_and_source(
            &mut traversal,
            &mut scroll_stack,
            &mut child_path,
            &mut source,
        );

        assert_eq!(first_count, stats.source_nodes);
        assert_eq!(source.records.len(), first_count);
        assert!(source.capacity() >= first_capacity);
    }
}
