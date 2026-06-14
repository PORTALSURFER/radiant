use std::collections::BTreeSet;

const KEY_MEMBERSHIP_LINEAR_SCAN_LIMIT: usize = 64;

pub(super) fn ordered_key_index<K: PartialEq>(key: &K, ordered_keys: &[K]) -> Option<usize> {
    ordered_keys.iter().position(|candidate| candidate == key)
}

pub(super) struct OrderedKeyMembership<'a, K> {
    ordered_keys: &'a [K],
    indexed_keys: Option<BTreeSet<&'a K>>,
}

impl<'a, K> OrderedKeyMembership<'a, K>
where
    K: Ord,
{
    pub(super) fn new(ordered_keys: &'a [K], expected_lookups: usize) -> Self {
        let indexed_keys = (ordered_keys.len().saturating_mul(expected_lookups)
            > KEY_MEMBERSHIP_LINEAR_SCAN_LIMIT)
            .then(|| ordered_keys.iter().collect());
        Self {
            ordered_keys,
            indexed_keys,
        }
    }

    pub(super) fn contains(&self, key: &K) -> bool {
        self.indexed_keys.as_ref().map_or_else(
            || self.ordered_keys.contains(key),
            |keys| keys.contains(key),
        )
    }
}
