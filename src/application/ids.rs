use crate::layout::NodeId;
use std::collections::HashSet;

#[cfg(test)]
#[path = "ids/tests.rs"]
mod tests;

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
// Small surfaces keep the compact sorted cursor path; large projected trees avoid
// sorting every reserved-id set during app view lowering.
const HASH_RESERVED_IDS_THRESHOLD: usize = 256;

#[allow(dead_code)]
pub(in crate::application) struct IdGenerator {
    next: NodeId,
    reserved: ReservedIds,
    reserved_range: Option<(NodeId, NodeId)>,
    claimed: HashSet<NodeId>,
}

/// Coarse private node kinds used as part of generated structural identity.
///
/// This intentionally does not expose widget type names or any application
/// data.  The kind only prevents an incompatible static replacement from
/// accidentally retaining the same generated identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::application) enum StructuralKind {
    Scene,
    Runtime,
    Widget,
    Container,
    Scroll,
    VirtualScroll,
    Overlay,
    FloatingLayer,
}

/// Typed static child positions used by application-view lowering.
///
/// Keeping roles typed prevents scene-layer/input positions from sharing the
/// same identity namespace as ordinary container children.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::application) enum StructuralRole {
    Root,
    SceneBase,
    SceneLayer(usize),
    SceneInput(usize),
    ContainerChild(usize),
    ScrollChild,
    VirtualScrollChild,
    FloatingLayerChild,
}

impl StructuralRole {
    fn tag(self) -> u64 {
        match self {
            Self::Root => 0,
            Self::SceneBase => 1,
            Self::SceneLayer(_) => 2,
            Self::SceneInput(_) => 3,
            Self::ContainerChild(_) => 4,
            Self::ScrollChild => 5,
            Self::VirtualScrollChild => 6,
            Self::FloatingLayerChild => 7,
        }
    }

    fn index(self) -> u64 {
        match self {
            Self::SceneLayer(index) | Self::SceneInput(index) | Self::ContainerChild(index) => {
                index as u64
            }
            Self::Root
            | Self::SceneBase
            | Self::ScrollChild
            | Self::VirtualScrollChild
            | Self::FloatingLayerChild => 0,
        }
    }
}

impl StructuralKind {
    fn tag(self) -> u64 {
        match self {
            Self::Scene => 0,
            Self::Runtime => 1,
            Self::Widget => 2,
            Self::Container => 3,
            Self::Scroll => 4,
            Self::VirtualScroll => 5,
            Self::Overlay => 6,
            Self::FloatingLayer => 7,
        }
    }
}

/// Generated identity plus the private scope used by its descendants.
///
/// The scope remains the un-probed structural candidate.  If a candidate
/// collides with an explicit/runtime id, only the externally visible node id
/// is probed; descendants retain their deterministic structural scope.
pub(in crate::application) struct StructuralIdentity {
    pub(in crate::application) id: NodeId,
    pub(in crate::application) scope: NodeId,
}

#[allow(dead_code)]
enum ReservedIds {
    Sorted { ids: Vec<NodeId>, cursor: usize },
    Hashed(HashSet<NodeId>),
}

impl IdGenerator {
    pub(in crate::application) fn new(reserved: Vec<NodeId>) -> Self {
        let reserved_range = reserved_id_range(&reserved);
        let mut reserved = reserved;
        let reserved = if reserved.len() >= HASH_RESERVED_IDS_THRESHOLD {
            ReservedIds::Hashed(hashed_reserved_ids(reserved))
        } else {
            reserved.sort_unstable();
            reserved.dedup();
            ReservedIds::Sorted {
                ids: reserved,
                cursor: 0,
            }
        };
        Self {
            next: 1,
            reserved,
            reserved_range,
            claimed: HashSet::new(),
        }
    }

    #[allow(dead_code)]
    pub(in crate::application) fn next(&mut self) -> NodeId {
        self.skip_reserved_run();
        let id = self.next;
        self.next += 1;
        id
    }

    pub(in crate::application) fn claim_explicit(&mut self, id: NodeId) -> bool {
        self.claimed.insert(id)
    }

    pub(in crate::application) fn next_structural(
        &mut self,
        scope: NodeId,
        kind: StructuralKind,
        role: StructuralRole,
    ) -> StructuralIdentity {
        let structural_scope = structural_id(scope, kind, role);
        let mut id = structural_scope;
        let mut probe = 0_u64;
        while id == 0 || self.is_reserved(id) || !self.claimed.insert(id) {
            probe = probe.wrapping_add(1);
            id = structural_probe(structural_scope, probe);
        }
        StructuralIdentity {
            id,
            scope: structural_scope,
        }
    }

    fn is_reserved(&self, id: NodeId) -> bool {
        match &self.reserved {
            ReservedIds::Sorted { ids, .. } => ids.binary_search(&id).is_ok(),
            ReservedIds::Hashed(ids) => ids.contains(&id),
        }
    }

    #[allow(dead_code)]
    fn skip_reserved_run(&mut self) {
        if self.next_is_outside_reserved_range() {
            return;
        }
        match &mut self.reserved {
            ReservedIds::Sorted { ids, cursor } => {
                while ids
                    .get(*cursor)
                    .is_some_and(|reserved| *reserved < self.next)
                {
                    *cursor += 1;
                }
                while ids
                    .get(*cursor)
                    .is_some_and(|reserved| *reserved == self.next)
                {
                    self.next = self.next.saturating_add(1);
                    *cursor += 1;
                }
            }
            ReservedIds::Hashed(ids) => {
                while ids.contains(&self.next) {
                    self.next = self.next.saturating_add(1);
                }
            }
        }
    }

    #[allow(dead_code)]
    fn next_is_outside_reserved_range(&self) -> bool {
        self.reserved_range
            .is_none_or(|(min, max)| self.next < min || self.next > max)
    }
}

fn hashed_reserved_ids(reserved: Vec<NodeId>) -> HashSet<NodeId> {
    let mut ids = HashSet::with_capacity(reserved.len());
    ids.extend(reserved);
    ids
}

fn reserved_id_range(reserved: &[NodeId]) -> Option<(NodeId, NodeId)> {
    let mut ids = reserved.iter().copied();
    let first = ids.next()?;
    let (mut min, mut max) = (first, first);
    for id in ids {
        min = min.min(id);
        max = max.max(id);
    }
    Some((min, max))
}

pub(in crate::application) fn scoped_key_id(scope: u64, key: &str) -> NodeId {
    let mut hash = super::ROOT_KEY_SCOPE;
    hash = hash_bytes(hash, &scope.to_le_bytes());
    hash = hash_bytes(hash, key.as_bytes());
    if hash == 0 { 1 } else { hash }
}

pub(in crate::application) fn structural_id(
    scope: NodeId,
    kind: StructuralKind,
    role: StructuralRole,
) -> NodeId {
    let mut hash = super::ROOT_KEY_SCOPE;
    hash = hash_bytes(hash, &scope.to_le_bytes());
    hash = hash_bytes(hash, &kind.tag().to_le_bytes());
    hash = hash_bytes(hash, &role.tag().to_le_bytes());
    hash = hash_bytes(hash, &role.index().to_le_bytes());
    if hash == 0 { 1 } else { hash }
}

fn structural_probe(base: NodeId, probe: u64) -> NodeId {
    let mut hash = hash_bytes(super::ROOT_KEY_SCOPE, &base.to_le_bytes());
    hash = hash_bytes(hash, &probe.to_le_bytes());
    if hash == 0 { 1 } else { hash }
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
