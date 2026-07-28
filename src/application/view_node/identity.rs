use super::{ViewNode, ViewNodeKind};
use crate::application::{
    ids::{StructuralKind, StructuralRole, structural_id},
    scoped_key_id,
};
use crate::layout::NodeId;
use std::fmt;

/// Owned application-facing identity for declarative view continuity.
///
/// A continuity key is scoped by its keyed or explicitly identified parent
/// during view lowering. Keeping the key as a distinct type prevents it from
/// being confused with numeric runtime ids or other application identities
/// while retaining ergonomic construction from ordinary strings.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ContinuityKey(String);

impl ContinuityKey {
    /// Construct a continuity key from owned or borrowed string data.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Borrow the stable key text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContinuityKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for ContinuityKey {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

impl From<&String> for ContinuityKey {
    fn from(key: &String) -> Self {
        Self::new(key.clone())
    }
}

impl From<String> for ContinuityKey {
    fn from(key: String) -> Self {
        Self(key)
    }
}

#[cfg(test)]
#[path = "identity/tests.rs"]
mod tests;

impl<Message> ViewNode<Message> {
    pub(super) fn collect_reserved_ids(&self, scope: u64, ids: &mut Vec<NodeId>) {
        self.collect_reserved_ids_at(scope, StructuralRole::Root, ids, true);
    }

    fn collect_reserved_ids_at(
        &self,
        scope: u64,
        role: StructuralRole,
        ids: &mut Vec<NodeId>,
        include_overlays: bool,
    ) {
        if !self.has_reserved_identity_in_subtree() {
            return;
        }
        if self.has_reserved_identity {
            match &self.kind {
                ViewNodeKind::Runtime(node) => {
                    if let Some(id) = self.resolved_id(scope) {
                        ids.push(id);
                    }
                    ids.push(node.id());
                }
                _ => {
                    if let Some(id) = self.resolved_id(scope) {
                        ids.push(id);
                    }
                }
            }
        }
        if !self.has_reserved_descendant_identity {
            return;
        }
        let child_scope = self.child_scope(scope, role);
        match &self.kind {
            ViewNodeKind::Scene { base, layers, .. } => {
                base.collect_reserved_ids_at(child_scope, StructuralRole::SceneBase, ids, false);
                let mut overlay_layers = Vec::new();
                base.collect_overlay_layers(&mut overlay_layers);
                for (index, layer) in overlay_layers.into_iter().chain(layers.iter()).enumerate() {
                    if let Some(input) = layer.input.as_ref() {
                        input.collect_reserved_ids_at(
                            child_scope,
                            StructuralRole::SceneInput(index),
                            ids,
                            true,
                        );
                    }
                    layer.view.collect_reserved_ids_at(
                        child_scope,
                        StructuralRole::SceneLayer(index),
                        ids,
                        true,
                    );
                }
            }
            ViewNodeKind::Container { children, .. } => {
                reserve_child_identity_capacity(children, ids);
                for (index, child) in children.iter().enumerate() {
                    child.collect_reserved_ids_at(
                        child_scope,
                        StructuralRole::ContainerChild(index),
                        ids,
                        include_overlays,
                    );
                }
            }
            ViewNodeKind::Scroll { child } => child.collect_reserved_ids_at(
                child_scope,
                StructuralRole::ScrollChild,
                ids,
                include_overlays,
            ),
            ViewNodeKind::VirtualScroll { child, .. } => child.collect_reserved_ids_at(
                child_scope,
                StructuralRole::VirtualScrollChild,
                ids,
                include_overlays,
            ),
            ViewNodeKind::FloatingLayer { child, .. } => child.collect_reserved_ids_at(
                child_scope,
                StructuralRole::FloatingLayerChild,
                ids,
                include_overlays,
            ),
            _ => {}
        }
        if include_overlays && !matches!(self.kind, ViewNodeKind::Scene { .. }) {
            for (index, layer) in self.overlay_layers.iter().enumerate() {
                if let Some(input) = layer.input.as_ref() {
                    input.collect_reserved_ids_at(
                        child_scope,
                        StructuralRole::SceneInput(index),
                        ids,
                        true,
                    );
                }
                layer.view.collect_reserved_ids_at(
                    child_scope,
                    StructuralRole::SceneLayer(index),
                    ids,
                    true,
                );
            }
        }
    }

    fn collect_overlay_layers<'a>(&'a self, layers: &mut Vec<&'a super::Layer<Message>>) {
        match &self.kind {
            ViewNodeKind::Scene {
                base,
                layers: scene_layers,
                ..
            } => {
                base.collect_overlay_layers(layers);
                for layer in scene_layers {
                    if let Some(input) = layer.input.as_ref() {
                        input.collect_overlay_layers(layers);
                    }
                    layer.view.collect_overlay_layers(layers);
                }
            }
            ViewNodeKind::Container { children, .. } => {
                for child in children {
                    child.collect_overlay_layers(layers);
                }
            }
            ViewNodeKind::Scroll { child }
            | ViewNodeKind::VirtualScroll { child, .. }
            | ViewNodeKind::FloatingLayer { child, .. } => child.collect_overlay_layers(layers),
            ViewNodeKind::Runtime(_)
            | ViewNodeKind::Widget(_)
            | ViewNodeKind::OverlayPanel { .. } => {}
        }
        layers.extend(self.overlay_layers.iter());
    }

    pub(super) fn resolved_id(&self, scope: u64) -> Option<NodeId> {
        self.id.or_else(|| {
            self.key
                .as_ref()
                .map(|key| scoped_key_id(scope, key.as_str()))
        })
    }

    fn child_scope(&self, parent_scope: u64, role: StructuralRole) -> u64 {
        self.resolved_id(parent_scope)
            .unwrap_or_else(|| structural_id(parent_scope, self.structural_kind(), role))
    }

    pub(super) fn structural_kind(&self) -> StructuralKind {
        match &self.kind {
            ViewNodeKind::Scene { .. } => StructuralKind::Scene,
            ViewNodeKind::Runtime(_) => StructuralKind::Runtime,
            ViewNodeKind::Widget(_) => StructuralKind::Widget,
            ViewNodeKind::Container { .. } => StructuralKind::Container,
            ViewNodeKind::Scroll { .. } => StructuralKind::Scroll,
            ViewNodeKind::VirtualScroll { .. } => StructuralKind::VirtualScroll,
            ViewNodeKind::OverlayPanel { .. } => StructuralKind::Overlay,
            ViewNodeKind::FloatingLayer { .. } => StructuralKind::FloatingLayer,
        }
    }
}

fn reserve_child_identity_capacity<Message>(children: &[ViewNode<Message>], ids: &mut Vec<NodeId>) {
    let mut reserved = 0;
    let mut nested_reserved = 0;
    for child in children {
        reserved += child.reserved_identity_capacity_hint();
        nested_reserved += usize::from(child.has_reserved_descendant_identity);
    }
    ids.reserve(reserved + nested_reserved);
}

impl<Message> ViewNode<Message> {
    fn reserved_identity_capacity_hint(&self) -> usize {
        if !self.has_reserved_identity {
            return 0;
        }
        match &self.kind {
            ViewNodeKind::Runtime(_) => 1 + usize::from(self.id.is_some() || self.key.is_some()),
            _ => 1,
        }
    }
}
