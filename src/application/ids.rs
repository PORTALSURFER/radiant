use crate::layout::NodeId;
use std::collections::HashSet;

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(in crate::application) struct IdGenerator {
    next: NodeId,
    reserved: HashSet<NodeId>,
}

impl IdGenerator {
    pub(in crate::application) fn new(reserved: HashSet<NodeId>) -> Self {
        Self { next: 1, reserved }
    }

    pub(in crate::application) fn next(&mut self) -> NodeId {
        while self.reserved.contains(&self.next) {
            self.next += 1;
        }
        let id = self.next;
        self.reserved.insert(id);
        self.next += 1;
        id
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
