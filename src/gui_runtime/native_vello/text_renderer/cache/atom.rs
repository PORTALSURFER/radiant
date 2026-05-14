//! Bounded text atom interning for native text layout cache keys.

use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::Arc;

const TEXT_ATOM_CACHE_CAPACITY: usize = 4_096;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct TextAtomProfileCounters {
    pub(super) hits: u64,
    pub(super) misses: u64,
    pub(super) evictions: u64,
}

pub(super) struct TextAtomCache {
    cache: HashMap<Arc<str>, u64>,
    order: VecDeque<(Arc<str>, u64)>,
    clock: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl TextAtomCache {
    pub(super) fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(TEXT_ATOM_CACHE_CAPACITY / 2),
            order: VecDeque::with_capacity(TEXT_ATOM_CACHE_CAPACITY),
            clock: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Intern text into a bounded atom cache so layout-key construction avoids
    /// hot-path `String` allocations on repeated runs.
    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.clock = self.clock.saturating_add(1);
        let stamp = self.clock;
        if let Some((cached, _)) = self.cache.get_key_value(text) {
            let atom = Arc::clone(cached);
            if let Some(last_seen) = self.cache.get_mut(text) {
                *last_seen = stamp;
            }
            self.order.push_back((Arc::clone(&atom), stamp));
            self.compact_order_if_needed();
            self.hits = self.hits.saturating_add(1);
            return atom;
        }

        self.misses = self.misses.saturating_add(1);
        let atom: Arc<str> = Arc::from(text);
        self.cache.insert(Arc::clone(&atom), stamp);
        self.order.push_back((Arc::clone(&atom), stamp));
        self.evict_stale_atoms();
        atom
    }

    #[cfg(test)]
    pub(super) fn take_profile_counters(&mut self) -> TextAtomProfileCounters {
        let counters = TextAtomProfileCounters {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        };
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
        counters
    }

    /// Compact queued atom-order metadata after repeated cache hits append stale stamps.
    fn compact_order_if_needed(&mut self) {
        if self.order.len() <= TEXT_ATOM_CACHE_CAPACITY.saturating_mul(2) {
            return;
        }
        let mut compacted = VecDeque::with_capacity(self.cache.len());
        for (atom, queued_stamp) in mem::take(&mut self.order) {
            if self
                .cache
                .get(atom.as_ref())
                .is_some_and(|current_stamp| *current_stamp == queued_stamp)
            {
                compacted.push_back((atom, queued_stamp));
            }
        }
        self.order = compacted;
    }

    /// Evict stale atom-cache entries using insertion stamps for bounded memory.
    fn evict_stale_atoms(&mut self) {
        while self.cache.len() > TEXT_ATOM_CACHE_CAPACITY {
            let Some((candidate, queued_stamp)) = self.order.pop_front() else {
                break;
            };
            let Some(current_stamp) = self.cache.get(candidate.as_ref()) else {
                continue;
            };
            if *current_stamp != queued_stamp {
                continue;
            }
            if self.cache.remove(candidate.as_ref()).is_some() {
                self.evictions = self.evictions.saturating_add(1);
            }
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.cache.len()
    }

    #[cfg(test)]
    fn order_len(&self) -> usize {
        self.order.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_text_reuses_cached_atom_and_tracks_hits() {
        let mut cache = TextAtomCache::new();

        let first = cache.intern_text("content row");
        let second = cache.intern_text("content row");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            cache.take_profile_counters(),
            TextAtomProfileCounters {
                hits: 1,
                misses: 1,
                evictions: 0,
            }
        );
    }

    #[test]
    fn eviction_drops_old_entries_once_capacity_is_exceeded() {
        let mut cache = TextAtomCache::new();
        for index in 0..=TEXT_ATOM_CACHE_CAPACITY {
            let _ = cache.intern_text(format!("label-{index}").as_str());
        }

        let counters = cache.take_profile_counters();
        assert_eq!(counters.misses, (TEXT_ATOM_CACHE_CAPACITY as u64) + 1);
        assert!(counters.evictions > 0);
        assert!(cache.len() <= TEXT_ATOM_CACHE_CAPACITY);
    }

    #[test]
    fn hit_queue_compacts_after_repeated_reuse() {
        let mut cache = TextAtomCache::new();
        let _ = cache.intern_text("content row");
        for _ in 0..=TEXT_ATOM_CACHE_CAPACITY.saturating_mul(2) {
            let _ = cache.intern_text("content row");
        }

        assert_eq!(cache.len(), 1);
        assert!(cache.order_len() <= TEXT_ATOM_CACHE_CAPACITY);
    }
}
