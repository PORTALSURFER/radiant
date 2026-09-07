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
    /// This record, or one of its declaration ancestors, depends on anchored
    /// geometry and is therefore admitted after the accepted layout pass.
    deferred: bool,
    active: bool,
}

impl OverlayFocusRecord {
    pub(crate) const fn root(&self) -> WidgetId {
        self.root
    }
    pub(crate) fn key(&self) -> &OverlayFocusKey {
        &self.key
    }
    pub(crate) const fn policy(&self) -> OverlayFocusPolicy {
        self.policy
    }
    #[cfg(test)]
    pub(crate) const fn parent(&self) -> Option<usize> {
        self.parent
    }
    pub(crate) const fn active(&self) -> bool {
        self.active
    }
    pub(crate) const fn deferred(&self) -> bool {
        self.deferred
    }
}

/// Bounded source-derived overlay membership evidence.
#[derive(Default)]
pub(crate) struct OverlayFocusProjection {
    records: Vec<OverlayFocusRecord>,
    members: HashMap<WidgetId, usize>,
    faulted: bool,
    invalid: bool,
    deferred_suppressed: bool,
    has_virtual_content: bool,
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
            self.records[index].active = !self.deferred_suppressed || !self.records[index].deferred;
            self.records[index].active = self.records[index].active
                && parent_active
                && layout.rects.contains_key(&self.records[index].root);
        }
    }

    pub(crate) const fn is_valid(&self) -> bool {
        !self.invalid
    }
    pub(crate) const fn is_invalid(&self) -> bool {
        self.invalid
    }
    #[cfg(test)]
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
    pub(crate) fn has_deferred_records(&self) -> bool {
        self.records.iter().any(OverlayFocusRecord::deferred)
    }
    pub(crate) const fn has_virtual_content(&self) -> bool {
        self.has_virtual_content
    }

    /// Keep anchor-driven groups out of the ordinary pre-publication focus
    /// transaction. Their activation is decided from the accepted layout.
    pub(crate) fn suppress_deferred(&mut self) {
        self.deferred_suppressed = true;
        for record in &mut self.records {
            if record.deferred {
                record.active = false;
            }
        }
    }
    pub(crate) fn top_active(&self) -> Option<&OverlayFocusRecord> {
        (!self.faulted)
            .then(|| self.records.iter().rev().find(|record| record.active))
            .flatten()
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
    pub(crate) fn contains_scope(&self, ancestor: usize, mut scope: usize) -> bool {
        if self.invalid {
            return false;
        }
        loop {
            let Some(record) = self.records.get(scope) else {
                return false;
            };
            if !record.active {
                return false;
            }
            if scope == ancestor {
                return true;
            }
            let Some(parent) = record.parent else {
                return false;
            };
            scope = parent;
        }
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
        // Bound retained evidence, but still inspect descendants after the
        // budget is exhausted or an identity collides: a later modal must not
        // accidentally turn incomplete evidence into an unrestricted surface.
        if seen.len() == MAX_SOURCE_NODES || !seen.insert(node.id()) {
            self.faulted = true;
        }
        let current = self.enter_node(node, inherited, layer_kind, false);
        if let Some(record) = current
            && (self.members.len() == MAX_SOURCE_NODES
                || self.members.insert(node.id(), record).is_some())
        {
            self.faulted = true;
        }
        match node {
            SurfaceNode::Scene(scene) => {
                self.collect_node(&scene.base, current, None, seen);
                for layer in scene.ordered_layers() {
                    // The visible body establishes identity and materialization;
                    // its synthesized input shield never substitutes for it.
                    let owner = self.enter_node(&layer.node, current, Some(layer.kind), true);
                    if let Some(input) = &layer.input {
                        self.collect_node(input, owner, Some(layer.kind), seen);
                    }
                    self.collect_node(&layer.node, owner, Some(layer.kind), seen);
                }
            }
            SurfaceNode::Container(container) => {
                self.has_virtual_content |= container.virtual_layout.is_some();
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

    fn enter_node<Message>(
        &mut self,
        node: &SurfaceNode<Message>,
        inherited: Option<usize>,
        layer_kind: Option<LayerKind>,
        layer_root: bool,
    ) -> Option<usize> {
        let mut current = inherited;
        if let Some(metadata) = node.source_metadata_handle() {
            for (position, evidence) in metadata.topology.overlays.iter().enumerate() {
                current = self.declarative_record(
                    evidence,
                    metadata.identity,
                    metadata.compatibility,
                    layout_identity(node),
                    current,
                    layer_root && position + 1 == metadata.topology.overlays.len(),
                );
            }
            if let Some(marker) = &metadata.overlay_focus {
                let kind = layer_kind.unwrap_or(LayerKind::Floating);
                current = self.raw_record(marker, kind, layout_identity(node), current);
            }
        }
        current
    }

    fn declarative_record(
        &mut self,
        evidence: &OverlayEvidence,
        root: SourceIdentity,
        compatibility: SourceCompatibility,
        node: WidgetId,
        parent: Option<usize>,
        strict_root: bool,
    ) -> Option<usize> {
        let deferred =
            evidence.anchored || parent.is_some_and(|index| self.records[index].deferred);
        if let Some((index, existing)) = self.records.iter().enumerate().find(|(_, record)| {
            matches!(&record.key, OverlayFocusKey::Declarative { identity, layer_kind, .. } if *identity == evidence.identity && *layer_kind == evidence.layer_kind)
        }) {
            let root_changed = strict_root && matches!(&existing.key,
                OverlayFocusKey::Declarative { root: old_root, compatibility: old_compatibility, .. }
                if *old_root != root || *old_compatibility != compatibility);
            if root_changed
                || existing.policy != evidence.focus_policy
                || existing.layer_kind != evidence.layer_kind
                || existing.deferred != deferred
            {
                self.faulted = true;
                return None;
            }
            return Some(index);
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
            evidence.anchored,
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
            false,
        )
    }

    fn push(
        &mut self,
        key: OverlayFocusKey,
        policy: OverlayFocusPolicy,
        layer_kind: LayerKind,
        root: WidgetId,
        parent: Option<usize>,
        anchored: bool,
    ) -> Option<usize> {
        if self.records.len() == MAX_OVERLAYS {
            self.faulted = true;
            return None;
        }
        let index = self.records.len();
        let deferred = anchored || parent.is_some_and(|parent| self.records[parent].deferred);
        self.records.push(OverlayFocusRecord {
            key,
            policy,
            layer_kind,
            root,
            parent,
            deferred,
            active: true,
        });
        Some(index)
    }
}

fn layout_identity<Message>(node: &SurfaceNode<Message>) -> WidgetId {
    match node {
        SurfaceNode::Scene(scene) if !scene.has_layers() => layout_identity(&scene.base),
        _ => node.id(),
    }
}

#[cfg(test)]
#[path = "overlay_focus/tests.rs"]
mod tests;

impl<Message> SurfaceNode<Message> {
    pub(crate) fn with_overlay_escape_dismissals(
        mut self,
        callbacks: Vec<Option<std::rc::Rc<dyn Fn() -> Message>>>,
    ) -> Self {
        if let Self::Scene(scene) = &mut self
            && callbacks.len() == scene.layers.len()
        {
            scene.escape_dismissals = callbacks
                .iter()
                .any(Option::is_some)
                .then(|| std::rc::Rc::new(callbacks));
        }
        self
    }

    pub(crate) fn overlay_escape_callback(
        &self,
        key: &OverlayFocusKey,
    ) -> Option<std::rc::Rc<dyn Fn() -> Message>> {
        match self {
            Self::Scene(scene) => {
                for index in scene.ordered_layer_indices() {
                    let layer = &scene.layers[index];
                    if let OverlayFocusKey::Declarative {
                        identity,
                        layer_kind,
                        root,
                        compatibility,
                    } = key
                        && let Some(source) = layer.node.source_metadata_handle()
                        && source.identity == *root
                        && source.compatibility == *compatibility
                        && source.topology.overlays.last().is_some_and(|evidence| {
                            evidence.identity == *identity && evidence.layer_kind == *layer_kind
                        })
                    {
                        return scene
                            .escape_dismissals
                            .as_ref()?
                            .get(index)
                            .cloned()
                            .flatten();
                    }
                    if let Some(callback) = layer.node.overlay_escape_callback(key) {
                        return Some(callback);
                    }
                }
                scene.base.overlay_escape_callback(key)
            }
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.overlay_escape_callback(key)),
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .find_map(|child| child.child.overlay_escape_callback(key)),
            Self::Widget(_) | Self::Overlay(_) => None,
        }
    }
}
