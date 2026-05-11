use crate::layout::NodeId;
use std::collections::HashSet;

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(in crate::application) struct IdGenerator {
    next: NodeId,
    reserved: HashSet<NodeId>,
    sorted_reserved: Option<Vec<NodeId>>,
    sorted_reserved_cursor: usize,
}

impl IdGenerator {
    pub(in crate::application) fn new(reserved: HashSet<NodeId>) -> Self {
        Self {
            next: 1,
            reserved,
            sorted_reserved: None,
            sorted_reserved_cursor: 0,
        }
    }

    pub(in crate::application) fn next(&mut self) -> NodeId {
        while self.reserved.contains(&self.next) {
            self.skip_reserved_run();
        }
        let id = self.next;
        self.reserved.insert(id);
        self.next += 1;
        id
    }

    fn skip_reserved_run(&mut self) {
        let sorted_reserved = self.sorted_reserved.get_or_insert_with(|| {
            let mut ids: Vec<NodeId> = self.reserved.iter().copied().collect();
            ids.sort_unstable();
            ids
        });
        while sorted_reserved
            .get(self.sorted_reserved_cursor)
            .is_some_and(|reserved| *reserved < self.next)
        {
            self.sorted_reserved_cursor += 1;
        }
        while sorted_reserved
            .get(self.sorted_reserved_cursor)
            .is_some_and(|reserved| *reserved == self.next)
        {
            self.next = self.next.saturating_add(1);
            self.sorted_reserved_cursor += 1;
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
        let reserved = [8, 20].into_iter().collect();
        let mut ids = IdGenerator::new(reserved);

        assert_eq!(
            (0..7).map(|_| ids.next()).collect::<Vec<_>>(),
            (1..=7).collect::<Vec<_>>()
        );
        assert_eq!(ids.next(), 9);
    }
}
