//! Bounded, non-authoritative overlay focus source projection.

use super::{
    LayerKind, OverlayEvidence, OverlayIdentity, SourceCompatibility, SourceIdentity, SurfaceNode,
    UiSurface,
};
use crate::{layout::LayoutOutput, runtime::OverlayFocusPolicy, widgets::WidgetId};
use std::collections::{HashMap, HashSet};

const MAX_OVERLAYS: usize = 64;
const MAX_SOURCE_NODES: usize = 65_536;

/// One qualified overlay source key retained for later focus admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OverlayFocusKey {
    Declarative {
        identity: OverlayIdentity,
        layer_kind: LayerKind,
        root: SourceIdentity,
        compatibility: SourceCompatibility,
    },
    Raw(crate::runtime::OverlayFocusOwner),
}

/// One ordered overlay record and its nearest qualified parent.
#[derive(Clone, Debug)]
pub(crate) struct OverlayFocusRecord {
    key: OverlayFocusKey,
    policy: OverlayFocusPolicy,
    layer_kind: LayerKind,
    root: WidgetId,
    parent: Option<usize>,
    active: bool,
}

impl OverlayFocusRecord {
    pub(crate) fn key(&self) -> &OverlayFocusKey {
        &self.key
    }
    pub(crate) const fn policy(&self) -> OverlayFocusPolicy {
        self.policy
    }
    pub(crate) const fn layer_kind(&self) -> LayerKind {
        self.layer_kind
    }
    pub(crate) const fn root(&self) -> WidgetId {
        self.root
    }
    pub(crate) const fn parent(&self) -> Option<usize> {
        self.parent
    }
    pub(crate) const fn active(&self) -> bool {
        self.active
    }
}

/// Bounded source-derived overlay membership evidence.
#[derive(Default)]
pub(crate) struct OverlayFocusProjection {
    records: Vec<OverlayFocusRecord>,
    members: HashMap<WidgetId, usize>,
    faulted: bool,
    invalid: bool,
}

impl OverlayFocusProjection {
    /// Collect policy and ancestry from one immutable surface snapshot.
    pub(crate) fn collect<Message>(surface: &UiSurface<Message>) -> Self {
        let mut projection = Self::default();
        let mut seen = HashSet::new();
        projection.collect_node(surface.root(), None, None, &mut seen);
        projection.invalid = projection.faulted
            && projection
                .records
                .iter()
                .any(|record| record.policy != OverlayFocusPolicy::None);
        projection
    }

    /// Mark records whose declared roots were omitted from final layout inactive.
    pub(crate) fn qualify(&mut self, layout: &LayoutOutput) {
        if self.invalid {
            return;
        }
        for index in 0..self.records.len() {
            let parent_active = self.records[index]
                .parent
                .is_none_or(|parent| self.records[parent].active);
            self.records[index].active =
                parent_active && layout.rects.contains_key(&self.records[index].root);
        }
    }

    pub(crate) const fn is_valid(&self) -> bool {
        !self.invalid
    }
    pub(crate) const fn is_invalid(&self) -> bool {
        self.invalid
    }
    pub(crate) fn has_authority(&self) -> bool {
        !self.invalid
            && self
                .records
                .iter()
                .any(|record| record.policy != OverlayFocusPolicy::None)
    }
    pub(crate) fn records(&self) -> &[OverlayFocusRecord] {
        &self.records
    }
    pub(crate) fn top_modal(&self) -> Option<usize> {
        (!self.invalid)
            .then(|| {
                self.records
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(index, record)| {
                        (record.active && record.policy == OverlayFocusPolicy::Modal)
                            .then_some(index)
                    })
            })
            .flatten()
    }
    pub(crate) fn contains_top_modal(&self, node: WidgetId) -> bool {
        self.top_modal()
            .is_some_and(|record| self.contains(record, node))
    }
    pub(crate) fn contains(&self, record: usize, node: WidgetId) -> bool {
        if self.invalid || record >= self.records.len() {
            return false;
        }
        let mut member = self.members.get(&node).copied();
        while let Some(index) = member {
            if index == record {
                return self.records[index].active;
            }
            member = self.records[index].parent;
        }
        false
    }

    fn collect_node<Message>(
        &mut self,
        node: &SurfaceNode<Message>,
        inherited: Option<usize>,
        layer_kind: Option<LayerKind>,
        seen: &mut HashSet<WidgetId>,
    ) {
        if seen.len() == MAX_SOURCE_NODES || !seen.insert(node.id()) {
            self.faulted = true;
        }
        let mut current = inherited;
        if let Some(metadata) = node.source_metadata_handle() {
            for evidence in &metadata.topology.overlays {
                current = self.declarative_record(
                    evidence,
                    metadata.identity,
                    metadata.compatibility,
                    node.id(),
                    current,
                );
            }
            if let Some(marker) = &metadata.overlay_focus {
                let kind = layer_kind.unwrap_or(LayerKind::Floating);
                current = self.raw_record(marker, kind, node.id(), current);
            }
        }
        if let Some(record) = current {
            if self.members.insert(node.id(), record).is_some() {
                self.faulted = true;
                return;
            }
        }
        match node {
            SurfaceNode::Scene(scene) => {
                self.collect_node(&scene.base, current, None, seen);
                for layer in scene.ordered_layers() {
                    if let Some(input) = &layer.input {
                        self.collect_node(input, current, Some(layer.kind), seen);
                    }
                    self.collect_node(&layer.node, current, Some(layer.kind), seen);
                }
            }
            SurfaceNode::Container(container) => {
                for child in container.children.iter() {
                    self.collect_node(&child.child, current, layer_kind, seen);
                }
            }
            SurfaceNode::FloatingLayer(layer) => {
                for child in layer.container.children.iter() {
                    self.collect_node(&child.child, current, layer_kind, seen);
                }
            }
            SurfaceNode::Widget(_) | SurfaceNode::Overlay(_) => {}
        }
    }

    fn declarative_record(
        &mut self,
        evidence: &OverlayEvidence,
        root: SourceIdentity,
        compatibility: SourceCompatibility,
        node: WidgetId,
        parent: Option<usize>,
    ) -> Option<usize> {
        if parent.is_some_and(|index| matches!(&self.records[index].key, OverlayFocusKey::Declarative { identity, layer_kind, .. } if *identity == evidence.identity && *layer_kind == evidence.layer_kind)) {
            return parent;
        }
        if self.records.iter().any(|record| matches!(&record.key, OverlayFocusKey::Declarative { identity, layer_kind, root: existing_root, compatibility: existing_compatibility } if *identity == evidence.identity && *layer_kind == evidence.layer_kind && (*existing_root != root || *existing_compatibility != compatibility))) {
            self.faulted = true; return None;
        }
        self.push(
            OverlayFocusKey::Declarative {
                identity: evidence.identity,
                layer_kind: evidence.layer_kind,
                root,
                compatibility,
            },
            evidence.focus_policy,
            evidence.layer_kind,
            node,
            parent,
        )
    }

    fn raw_record(
        &mut self,
        marker: &crate::runtime::overlay_focus::OverlayFocusMarker,
        layer_kind: LayerKind,
        node: WidgetId,
        parent: Option<usize>,
    ) -> Option<usize> {
        if parent.is_some_and(|index| matches!(&self.records[index].key, OverlayFocusKey::Raw(owner) if *owner == marker.owner)) { return parent; }
        if self.records.iter().any(
            |record| matches!(&record.key, OverlayFocusKey::Raw(owner) if *owner == marker.owner),
        ) {
            self.faulted = true;
            return None;
        }
        self.push(
            OverlayFocusKey::Raw(marker.owner.clone()),
            marker.policy,
            layer_kind,
            node,
            parent,
        )
    }

    fn push(
        &mut self,
        key: OverlayFocusKey,
        policy: OverlayFocusPolicy,
        layer_kind: LayerKind,
        root: WidgetId,
        parent: Option<usize>,
    ) -> Option<usize> {
        if self.records.len() == MAX_OVERLAYS {
            self.faulted = true;
            return None;
        }
        let index = self.records.len();
        self.records.push(OverlayFocusRecord {
            key,
            policy,
            layer_kind,
            root,
            parent,
            active: true,
        });
        Some(index)
    }
}
