use super::{ViewNode, ViewNodeKind};
use crate::application::{
    ids::{StructuralKind, StructuralRole, structural_id},
    scoped_key_id,
};
use crate::layout::NodeId;
use crate::runtime::LayerKind;
use std::collections::HashSet;
use std::{
    any::type_name,
    fmt,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DECLARATIVE_EFFECT_OWNER: AtomicU64 = AtomicU64::new(1);

/// Typed identity metadata attached to a root produced by a keyed collection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::application) struct KeyedIdentity {
    pub(in crate::application) key_type: u64,
    pub(in crate::application) key_fingerprint: u64,
}

/// Origin of the identity attached to one declarative view node.
///
/// This is crate-private source evidence only.  In particular, generated and
/// numeric identities remain observational and cannot become keyed-node
/// candidates merely because they reach runtime lowering.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DeclarativeIdentityOrigin {
    GeneratedStructural,
    ExplicitNumericId,
    ExplicitContinuityKey,
    InferredKeyedIdentity,
    UnreidentifiedDirectRuntimeRoot,
}

/// Opaque application-owned identity for one declarative delayed-work owner.
///
/// The handle is intentionally independent of a runtime instance and carries
/// no view-tree, traversal, callback, or controller state. Keep it in
/// application state and attach the same value to the corresponding keyed view
/// or overlay declaration.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DeclarativeEffectOwner(u64);

impl DeclarativeEffectOwner {
    /// Allocate one process-unique declarative effect-owner handle.
    pub fn new() -> Self {
        Self(NEXT_DECLARATIVE_EFFECT_OWNER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for DeclarativeEffectOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl DeclarativeIdentityOrigin {
    pub(crate) const fn is_keyed(self) -> bool {
        matches!(
            self,
            Self::ExplicitContinuityKey | Self::InferredKeyedIdentity
        )
    }
}

/// Identity evidence captured before a declarative node is lowered.
///
/// `structural_scope` is always the deterministic, unprobed structural
/// candidate.  `resolved_id` is the id used by the current lowered surface;
/// generated keyed candidates use the structural candidate until the id
/// generator has supplied the final (possibly probed) id.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SourceIdentitySeed {
    pub(crate) resolved_id: NodeId,
    pub(crate) structural_scope: NodeId,
    pub(crate) origin: DeclarativeIdentityOrigin,
    pub(crate) effect_owner: Option<DeclarativeEffectOwner>,
}

/// One stable identity for a declarative overlay declaration.
///
/// The layer kind deliberately lives beside this identity rather than being
/// folded into it, so identity and compatibility evidence remain distinct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeclarativeOverlaySource {
    pub(crate) identity_scope: NodeId,
    pub(crate) layer_kind: LayerKind,
    pub(crate) effect_owner: Option<DeclarativeEffectOwner>,
}

/// Source ancestry retained while the application tree is being lowered.
#[derive(Clone, Debug, Default)]
pub(crate) struct DeclarativeSourceContext {
    pub(crate) keyed_nodes: Vec<SourceIdentitySeed>,
    pub(crate) overlays: Vec<DeclarativeOverlaySource>,
}

impl DeclarativeSourceContext {
    pub(crate) fn with_node(&self, seed: SourceIdentitySeed) -> Self {
        let mut next = self.clone();
        if seed.origin.is_keyed() {
            next.keyed_nodes.push(seed);
        }
        next
    }

    pub(crate) fn with_overlay(&self, overlay: DeclarativeOverlaySource) -> Self {
        let mut next = self.clone();
        next.overlays.push(overlay);
        next
    }
}

impl KeyedIdentity {
    pub(in crate::application) fn from_key<Key: Hash + ?Sized + 'static>(key: &Key) -> Self {
        Self {
            key_type: fingerprint(type_name::<Key>()),
            key_fingerprint: fingerprint_value(key),
        }
    }
}

fn fingerprint(value: &str) -> u64 {
    let mut hasher = FnvHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

fn fingerprint_value<Key: Hash + ?Sized>(value: &Key) -> u64 {
    let mut hasher = FnvHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

#[derive(Default)]
struct FnvHasher(u64);

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf2_9ce4_8422_2325;
        }
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

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

/// Apply an explicit continuity key to the root of a view subtree.
///
/// This is the opt-in escape hatch for a conditional replacement or another
/// structural change whose compatible transient state should remain attached to
/// the view. It has the same last-modifier-wins behavior as [`ViewNode::key`]:
/// a later `.id(...)` or `.key(...)` call replaces this identity.
pub fn preserve_state<Message>(key: ContinuityKey, view: ViewNode<Message>) -> ViewNode<Message> {
    view.key(key)
}

#[cfg(test)]
#[path = "identity/tests.rs"]
mod tests;

impl<Message> ViewNode<Message> {
    pub(super) fn collect_explicit_identity_collisions(
        &self,
        scope: u64,
        key_ids: &mut HashSet<NodeId>,
        explicit_ids: &mut HashSet<NodeId>,
    ) -> Result<(), ()> {
        self.collect_explicit_identity_collisions_at(
            scope,
            StructuralRole::Root,
            key_ids,
            explicit_ids,
            true,
        )
    }

    fn collect_explicit_identity_collisions_at(
        &self,
        scope: u64,
        role: StructuralRole,
        key_ids: &mut HashSet<NodeId>,
        explicit_ids: &mut HashSet<NodeId>,
        include_overlays: bool,
    ) -> Result<(), ()> {
        if let Some(key) = self.key.as_ref() {
            let id = scoped_key_id(scope, key.as_str());
            if explicit_ids.contains(&id) || !key_ids.insert(id) {
                return Err(());
            }
        }
        if let Some(id) = self.id
            && (key_ids.contains(&id) || !explicit_ids.insert(id))
        {
            return Err(());
        }
        if let ViewNodeKind::Runtime(node) = &self.kind
            && self.id.is_none()
            && self.key.is_none()
            && (key_ids.contains(&node.id()) || !explicit_ids.insert(node.id()))
        {
            return Err(());
        }

        let child_scope = self.child_scope(scope, role);
        match &self.kind {
            ViewNodeKind::Scene { base, layers, .. } => {
                base.collect_explicit_identity_collisions_at(
                    child_scope,
                    StructuralRole::SceneBase,
                    key_ids,
                    explicit_ids,
                    false,
                )?;
                let mut overlay_layers = Vec::new();
                base.collect_overlay_layers(&mut overlay_layers);
                for (index, layer) in overlay_layers.into_iter().chain(layers.iter()).enumerate() {
                    if let Some(input) = layer.input.as_ref() {
                        input.collect_explicit_identity_collisions_at(
                            child_scope,
                            StructuralRole::SceneInput(index),
                            key_ids,
                            explicit_ids,
                            true,
                        )?;
                    }
                    layer.view.collect_explicit_identity_collisions_at(
                        child_scope,
                        StructuralRole::SceneLayer(index),
                        key_ids,
                        explicit_ids,
                        true,
                    )?;
                }
            }
            ViewNodeKind::Container { children, .. } => {
                for (index, child) in children.iter().enumerate() {
                    child.collect_explicit_identity_collisions_at(
                        child_scope,
                        StructuralRole::ContainerChild(index),
                        key_ids,
                        explicit_ids,
                        include_overlays,
                    )?;
                }
            }
            ViewNodeKind::Scroll { child } => child.collect_explicit_identity_collisions_at(
                child_scope,
                StructuralRole::ScrollChild,
                key_ids,
                explicit_ids,
                include_overlays,
            )?,
            ViewNodeKind::VirtualScroll { child, .. } => child
                .collect_explicit_identity_collisions_at(
                    child_scope,
                    StructuralRole::VirtualScrollChild,
                    key_ids,
                    explicit_ids,
                    include_overlays,
                )?,
            ViewNodeKind::FloatingLayer { child, .. } => child
                .collect_explicit_identity_collisions_at(
                    child_scope,
                    StructuralRole::FloatingLayerChild,
                    key_ids,
                    explicit_ids,
                    include_overlays,
                )?,
            ViewNodeKind::Runtime(_)
            | ViewNodeKind::VirtualLayout(_)
            | ViewNodeKind::Widget(_)
            | ViewNodeKind::OverlayPanel { .. } => {}
        }
        if include_overlays && !matches!(self.kind, ViewNodeKind::Scene { .. }) {
            for (index, layer) in self.overlay_layers.iter().enumerate() {
                if let Some(input) = layer.input.as_ref() {
                    input.collect_explicit_identity_collisions_at(
                        child_scope,
                        StructuralRole::SceneInput(index),
                        key_ids,
                        explicit_ids,
                        true,
                    )?;
                }
                layer.view.collect_explicit_identity_collisions_at(
                    child_scope,
                    StructuralRole::SceneLayer(index),
                    key_ids,
                    explicit_ids,
                    true,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn collect_keyed_collisions(
        &self,
        scope: u64,
        candidates: &mut HashSet<NodeId>,
    ) -> Result<(), ()> {
        self.collect_keyed_collisions_at(scope, StructuralRole::Root, candidates, true)
    }

    fn collect_keyed_collisions_at(
        &self,
        scope: u64,
        role: StructuralRole,
        candidates: &mut HashSet<NodeId>,
        include_overlays: bool,
    ) -> Result<(), ()> {
        if let Some(identity) = self.keyed_identity {
            let candidate = crate::application::ids::keyed_structural_id(
                scope,
                self.structural_kind(),
                identity.key_type,
                identity.key_fingerprint,
            );
            if !candidates.insert(candidate) {
                return Err(());
            }
        }
        let child_scope = self.child_scope(scope, role);
        match &self.kind {
            ViewNodeKind::Scene { base, layers, .. } => {
                base.collect_keyed_collisions_at(
                    child_scope,
                    StructuralRole::SceneBase,
                    candidates,
                    false,
                )?;
                let mut overlay_layers = Vec::new();
                base.collect_overlay_layers(&mut overlay_layers);
                for (index, layer) in overlay_layers.into_iter().chain(layers.iter()).enumerate() {
                    if let Some(input) = layer.input.as_ref() {
                        input.collect_keyed_collisions_at(
                            child_scope,
                            StructuralRole::SceneInput(index),
                            candidates,
                            true,
                        )?;
                    }
                    layer.view.collect_keyed_collisions_at(
                        child_scope,
                        StructuralRole::SceneLayer(index),
                        candidates,
                        true,
                    )?;
                }
            }
            ViewNodeKind::Container { children, .. } => {
                for (index, child) in children.iter().enumerate() {
                    child.collect_keyed_collisions_at(
                        child_scope,
                        StructuralRole::ContainerChild(index),
                        candidates,
                        include_overlays,
                    )?;
                }
            }
            ViewNodeKind::Scroll { child } => child.collect_keyed_collisions_at(
                child_scope,
                StructuralRole::ScrollChild,
                candidates,
                include_overlays,
            )?,
            ViewNodeKind::VirtualScroll { child, .. } => child.collect_keyed_collisions_at(
                child_scope,
                StructuralRole::VirtualScrollChild,
                candidates,
                include_overlays,
            )?,
            ViewNodeKind::FloatingLayer { child, .. } => child.collect_keyed_collisions_at(
                child_scope,
                StructuralRole::FloatingLayerChild,
                candidates,
                include_overlays,
            )?,
            ViewNodeKind::Runtime(_)
            | ViewNodeKind::VirtualLayout(_)
            | ViewNodeKind::Widget(_)
            | ViewNodeKind::OverlayPanel { .. } => {}
        }
        if include_overlays && !matches!(self.kind, ViewNodeKind::Scene { .. }) {
            for (index, layer) in self.overlay_layers.iter().enumerate() {
                if let Some(input) = layer.input.as_ref() {
                    input.collect_keyed_collisions_at(
                        child_scope,
                        StructuralRole::SceneInput(index),
                        candidates,
                        true,
                    )?;
                }
                layer.view.collect_keyed_collisions_at(
                    child_scope,
                    StructuralRole::SceneLayer(index),
                    candidates,
                    true,
                )?;
            }
        }
        Ok(())
    }

    pub(in crate::application) fn with_inferred_keyed_identity(
        mut self,
        identity: KeyedIdentity,
    ) -> Self {
        if self.id.is_none() && self.key.is_none() {
            self.keyed_identity = Some(identity);
            self.has_reserved_identity = true;
        }
        self
    }

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
            | ViewNodeKind::VirtualLayout(_)
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

    pub(crate) fn source_identity_origin(&self) -> DeclarativeIdentityOrigin {
        if self.id.is_some() {
            DeclarativeIdentityOrigin::ExplicitNumericId
        } else if self.key.is_some() {
            DeclarativeIdentityOrigin::ExplicitContinuityKey
        } else if self.keyed_identity.is_some() {
            DeclarativeIdentityOrigin::InferredKeyedIdentity
        } else if matches!(self.kind, ViewNodeKind::Runtime(_)) {
            DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot
        } else {
            DeclarativeIdentityOrigin::GeneratedStructural
        }
    }

    pub(in crate::application) fn unprobed_structural_scope(
        &self,
        parent_scope: u64,
        role: StructuralRole,
    ) -> NodeId {
        self.resolved_id(parent_scope)
            .or_else(|| {
                self.keyed_identity.map(|identity| {
                    crate::application::ids::keyed_structural_id(
                        parent_scope,
                        self.structural_kind(),
                        identity.key_type,
                        identity.key_fingerprint,
                    )
                })
            })
            .unwrap_or_else(|| structural_id(parent_scope, self.structural_kind(), role))
    }

    pub(in crate::application) fn source_identity_seed(
        &self,
        parent_scope: u64,
        role: StructuralRole,
    ) -> SourceIdentitySeed {
        let origin = self.source_identity_origin();
        let structural_scope = self.unprobed_structural_scope(parent_scope, role);
        let resolved_id = match (&self.kind, origin) {
            (
                ViewNodeKind::Runtime(node),
                DeclarativeIdentityOrigin::UnreidentifiedDirectRuntimeRoot,
            ) => node.id(),
            _ => self.resolved_id(parent_scope).unwrap_or(structural_scope),
        };
        SourceIdentitySeed {
            resolved_id,
            structural_scope,
            origin,
            effect_owner: self.effect_owner,
        }
    }

    fn child_scope(&self, parent_scope: u64, role: StructuralRole) -> u64 {
        self.unprobed_structural_scope(parent_scope, role)
    }

    pub(super) fn structural_kind(&self) -> StructuralKind {
        match &self.kind {
            ViewNodeKind::Scene { .. } => StructuralKind::Scene,
            ViewNodeKind::Runtime(_) => StructuralKind::Runtime,
            ViewNodeKind::VirtualLayout(_) => StructuralKind::VirtualLayout,
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
