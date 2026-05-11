use crate::layout::NodeId;
use std::collections::HashSet;

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
// Small surfaces keep the compact sorted cursor path; large projected trees avoid
// sorting every reserved-id set during app view lowering.
const HASH_RESERVED_IDS_THRESHOLD: usize = 256;

pub(in crate::application) struct IdGenerator {
    next: NodeId,
    reserved: ReservedIds,
}

enum ReservedIds {
    Sorted { ids: Vec<NodeId>, cursor: usize },
    Hashed(HashSet<NodeId>),
}

impl IdGenerator {
    pub(in crate::application) fn new(mut reserved: Vec<NodeId>) -> Self {
        let reserved = if reserved.len() >= HASH_RESERVED_IDS_THRESHOLD {
            ReservedIds::Hashed(reserved.into_iter().collect())
        } else {
            reserved.sort_unstable();
            reserved.dedup();
            ReservedIds::Sorted {
                ids: reserved,
                cursor: 0,
            }
        };
        Self { next: 1, reserved }
    }

    pub(in crate::application) fn next(&mut self) -> NodeId {
        self.skip_reserved_run();
        let id = self.next;
        self.next += 1;
        id
    }

    fn skip_reserved_run(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_generator_skips_dense_reserved_runs_after_collision() {
        let reserved = (4..=10_000).collect();
        let mut ids = IdGenerator::new(reserved);

        assert_eq!(ids.next(), 1);
        assert_eq!(ids.next(), 2);
        assert_eq!(ids.next(), 3);
        assert_eq!(ids.next(), 10_001);
        assert_eq!(ids.next(), 10_002);
    }

    #[test]
    fn id_generator_preserves_sparse_generation_before_collision() {
        let reserved = vec![8, 20];
        let mut ids = IdGenerator::new(reserved);

        assert_eq!(
            (0..7).map(|_| ids.next()).collect::<Vec<_>>(),
            (1..=7).collect::<Vec<_>>()
        );
        assert_eq!(ids.next(), 9);
    }

    #[test]
    fn id_generator_deduplicates_reserved_ids_before_generation() {
        let mut ids = IdGenerator::new(vec![1, 1, 2, 4]);

        assert_eq!(ids.next(), 3);
        assert_eq!(ids.next(), 5);
    }

    #[test]
    fn id_generator_uses_hashed_reserved_ids_for_large_sets() {
        let ids = IdGenerator::new((10_000..=10_512).rev().collect());

        assert!(matches!(ids.reserved, ReservedIds::Hashed(_)));
    }

    #[test]
    fn id_generator_keeps_sorted_reserved_ids_for_small_sets() {
        let ids = IdGenerator::new(vec![8, 4, 4]);

        match ids.reserved {
            ReservedIds::Sorted { ids, cursor } => {
                assert_eq!(ids, vec![4, 8]);
                assert_eq!(cursor, 0);
            }
            ReservedIds::Hashed(_) => panic!("small reserved sets should stay sorted vectors"),
        }
    }
}
