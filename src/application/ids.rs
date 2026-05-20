use crate::layout::NodeId;
use std::collections::HashSet;

#[cfg(test)]
#[path = "ids/tests.rs"]
mod tests;

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
// Small surfaces keep the compact sorted cursor path; large projected trees avoid
// sorting every reserved-id set during app view lowering.
const HASH_RESERVED_IDS_THRESHOLD: usize = 256;

pub(in crate::application) struct IdGenerator {
    next: NodeId,
    reserved: ReservedIds,
    reserved_range: Option<(NodeId, NodeId)>,
}

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
        }
    }

    pub(in crate::application) fn next(&mut self) -> NodeId {
        self.skip_reserved_run();
        let id = self.next;
        self.next += 1;
        id
    }

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

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
