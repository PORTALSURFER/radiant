//! Bounded text atom interning for native text layout cache keys.

use super::TextCacheProfileCounters;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::Arc;

const TEXT_ATOM_CACHE_CAPACITY: usize = 4_096;

pub(super) struct TextAtomCache {
    cache: HashMap<Arc<str>, u64>,
    order: VecDeque<(Arc<str>, u64)>,
    clock: u64,
    profile: TextCacheProfileCounters,
}

impl TextAtomCache {
    pub(super) fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(TEXT_ATOM_CACHE_CAPACITY / 2),
            order: VecDeque::with_capacity(TEXT_ATOM_CACHE_CAPACITY),
            clock: 0,
            profile: TextCacheProfileCounters::default(),
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
            record_atom_cache_access(&mut self.order, Arc::clone(&atom), stamp);
            self.compact_order_if_needed();
            self.profile.hits = self.profile.hits.saturating_add(1);
            return atom;
        }

        self.profile.misses = self.profile.misses.saturating_add(1);
        let atom: Arc<str> = Arc::from(text);
        self.cache.insert(Arc::clone(&atom), stamp);
        record_atom_cache_access(&mut self.order, Arc::clone(&atom), stamp);
        self.evict_stale_atoms();
        atom
    }

    pub(super) fn take_profile_counters(&mut self) -> TextCacheProfileCounters {
        std::mem::take(&mut self.profile)
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
                self.profile.evictions = self.profile.evictions.saturating_add(1);
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

fn record_atom_cache_access(order: &mut VecDeque<(Arc<str>, u64)>, atom: Arc<str>, stamp: u64) {
    if let Some((queued_atom, queued_stamp)) = order.back_mut()
        && queued_atom.as_ref() == atom.as_ref()
    {
        *queued_stamp = stamp;
        return;
    }
    order.push_back((atom, stamp));
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
            TextCacheProfileCounters {
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

    #[test]
    fn consecutive_atom_cache_hits_coalesce_recency_queue_entry() {
        let mut cache = TextAtomCache::new();

        let first = cache.intern_text("content row");
        let second = cache.intern_text("content row");
        let third = cache.intern_text("content row");

        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(&second, &third));
        assert_eq!(cache.order_len(), 1);
        assert_eq!(cache.order[0].0.as_ref(), "content row");
        assert_eq!(cache.order[0].1, 3);
    }
}
