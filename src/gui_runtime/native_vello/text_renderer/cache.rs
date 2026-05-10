//! Layout-cache and atom-cache helpers for the native text renderer.

use super::{TextLayout, TextLayoutKey, layout::compute_layout};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use vello::peniko::FontData;

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;
const TEXT_ATOM_CACHE_CAPACITY: usize = 4_096;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextLayoutProfileCounters {
    pub layout_hits: u64,
    pub layout_misses: u64,
    pub layout_evictions: u64,
    pub atom_hits: u64,
    pub atom_misses: u64,
    pub atom_evictions: u64,
}

pub(super) struct TextLayoutCache {
    layout_cache: HashMap<TextLayoutKey, TextLayout>,
    layout_cache_order: VecDeque<(TextLayoutKey, u64)>,
    layout_cache_stamps: HashMap<TextLayoutKey, u64>,
    layout_cache_clock: u64,
    atom_cache: HashMap<Arc<str>, u64>,
    atom_cache_order: VecDeque<(Arc<str>, u64)>,
    atom_cache_clock: u64,
    text_layout_hits: u64,
    text_layout_misses: u64,
    text_layout_evictions: u64,
    text_atom_hits: u64,
    text_atom_misses: u64,
    text_atom_evictions: u64,
}

impl TextLayoutCache {
    pub(super) fn new() -> Self {
        Self {
            layout_cache: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
            layout_cache_order: VecDeque::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY),
            layout_cache_stamps: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
            layout_cache_clock: 0,
            atom_cache: HashMap::with_capacity(TEXT_ATOM_CACHE_CAPACITY / 2),
            atom_cache_order: VecDeque::with_capacity(TEXT_ATOM_CACHE_CAPACITY),
            atom_cache_clock: 0,
            text_layout_hits: 0,
            text_layout_misses: 0,
            text_layout_evictions: 0,
            text_atom_hits: 0,
            text_atom_misses: 0,
            text_atom_evictions: 0,
        }
    }

    pub(super) fn layout_for<'a>(
        &'a mut self,
        font: &FontData,
        text: &str,
        font_size: f32,
    ) -> Option<&'a TextLayout> {
        let text_atom = self.intern_text(text);
        let key = TextLayoutKey {
            text: text_atom,
            font_size_bits: font_size.to_bits(),
        };

        // Split hit detection from the returned borrow so cache profiling can
        // stay safe without extending an immutable borrow across the miss path.
        if self.layout_cache.contains_key(&key) {
            self.touch_layout_cache_key(&key);
            self.text_layout_hits = self.text_layout_hits.saturating_add(1);
            return self.layout_cache.get(&key);
        }

        self.text_layout_misses = self.text_layout_misses.saturating_add(1);

        self.evict_stale_layouts();

        let layout = compute_layout(font, text, font_size)?;
        self.touch_layout_cache_key(&key);
        let cached_layout = self.layout_cache.entry(key).or_insert(layout);
        Some(cached_layout)
    }

    pub(super) fn take_profile_counters(&mut self) -> TextLayoutProfileCounters {
        let counters = TextLayoutProfileCounters {
            layout_hits: self.text_layout_hits,
            layout_misses: self.text_layout_misses,
            layout_evictions: self.text_layout_evictions,
            atom_hits: self.text_atom_hits,
            atom_misses: self.text_atom_misses,
            atom_evictions: self.text_atom_evictions,
        };
        self.text_layout_hits = 0;
        self.text_layout_misses = 0;
        self.text_layout_evictions = 0;
        self.text_atom_hits = 0;
        self.text_atom_misses = 0;
        self.text_atom_evictions = 0;
        counters
    }

    /// Intern text into a bounded atom cache so layout-key construction avoids
    /// hot-path `String` allocations on repeated runs.
    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.atom_cache_clock = self.atom_cache_clock.saturating_add(1);
        let stamp = self.atom_cache_clock;
        if let Some((cached, _)) = self.atom_cache.get_key_value(text) {
            let atom = Arc::clone(cached);
            if let Some(last_seen) = self.atom_cache.get_mut(text) {
                *last_seen = stamp;
            }
            self.atom_cache_order.push_back((Arc::clone(&atom), stamp));
            self.compact_atom_cache_order_if_needed();
            self.text_atom_hits = self.text_atom_hits.saturating_add(1);
            return atom;
        }

        self.text_atom_misses = self.text_atom_misses.saturating_add(1);
        let atom: Arc<str> = Arc::from(text);
        self.atom_cache.insert(Arc::clone(&atom), stamp);
        self.atom_cache_order.push_back((Arc::clone(&atom), stamp));
        self.evict_stale_atoms();
        atom
    }

    /// Record layout-cache recency without reallocating the cached layout.
    pub(super) fn touch_layout_cache_key(&mut self, key: &TextLayoutKey) {
        self.layout_cache_clock = self.layout_cache_clock.saturating_add(1);
        let stamp = self.layout_cache_clock;
        self.layout_cache_stamps.insert(key.clone(), stamp);
        self.layout_cache_order.push_back((key.clone(), stamp));
        self.compact_layout_cache_order_if_needed();
    }

    /// Compact queued layout-order metadata after repeated cache hits append stale stamps.
    fn compact_layout_cache_order_if_needed(&mut self) {
        if self.layout_cache_order.len() <= TEXT_LAYOUT_CACHE_CAPACITY.saturating_mul(2) {
            return;
        }
        let mut ordered_layouts: Vec<_> = self
            .layout_cache_stamps
            .iter()
            .map(|(key, stamp)| (key.clone(), *stamp))
            .collect();
        ordered_layouts.sort_by_key(|(_, stamp)| *stamp);
        self.layout_cache_order = ordered_layouts.into_iter().collect();
    }

    /// Evict stale layout-cache entries by last-use stamp so hot text survives churn.
    pub(super) fn evict_stale_layouts(&mut self) {
        while self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            let Some((candidate, queued_stamp)) = self.layout_cache_order.pop_front() else {
                break;
            };
            let Some(current_stamp) = self.layout_cache_stamps.get(&candidate) else {
                continue;
            };
            if *current_stamp != queued_stamp {
                continue;
            }
            self.layout_cache_stamps.remove(&candidate);
            if self.layout_cache.remove(&candidate).is_some() {
                self.text_layout_evictions = self.text_layout_evictions.saturating_add(1);
            }
        }
    }

    /// Compact queued atom-order metadata after repeated cache hits append stale stamps.
    fn compact_atom_cache_order_if_needed(&mut self) {
        if self.atom_cache_order.len() <= TEXT_ATOM_CACHE_CAPACITY.saturating_mul(2) {
            return;
        }
        let mut ordered_atoms: Vec<_> = self
            .atom_cache
            .iter()
            .map(|(atom, stamp)| (Arc::clone(atom), *stamp))
            .collect();
        ordered_atoms.sort_by_key(|(_, stamp)| *stamp);
        self.atom_cache_order = ordered_atoms.into_iter().collect();
    }

    /// Evict stale atom-cache entries using insertion stamps for bounded memory.
    pub(super) fn evict_stale_atoms(&mut self) {
        while self.atom_cache.len() > TEXT_ATOM_CACHE_CAPACITY {
            let Some((candidate, queued_stamp)) = self.atom_cache_order.pop_front() else {
                break;
            };
            let Some(current_stamp) = self.atom_cache.get(candidate.as_ref()) else {
                continue;
            };
            if *current_stamp != queued_stamp {
                continue;
            }
            if self.atom_cache.remove(candidate.as_ref()).is_some() {
                self.text_atom_evictions = self.text_atom_evictions.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_key(label: &str) -> TextLayoutKey {
        TextLayoutKey {
            text: Arc::from(label),
            font_size_bits: 12.0_f32.to_bits(),
        }
    }

    #[test]
    fn intern_text_reuses_cached_atom_and_tracks_hits() {
        let mut cache = TextLayoutCache::new();

        let first = cache.intern_text("content row");
        let second = cache.intern_text("content row");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(
            cache.take_profile_counters(),
            TextLayoutProfileCounters {
                layout_hits: 0,
                layout_misses: 0,
                layout_evictions: 0,
                atom_hits: 1,
                atom_misses: 1,
                atom_evictions: 0,
            }
        );
    }

    #[test]
    fn atom_cache_eviction_drops_old_entries_once_capacity_is_exceeded() {
        let mut cache = TextLayoutCache::new();
        for index in 0..=TEXT_ATOM_CACHE_CAPACITY {
            let _ = cache.intern_text(format!("label-{index}").as_str());
        }

        let counters = cache.take_profile_counters();
        assert_eq!(counters.atom_misses, (TEXT_ATOM_CACHE_CAPACITY as u64) + 1);
        assert!(counters.atom_evictions > 0);
        assert!(cache.atom_cache.len() <= TEXT_ATOM_CACHE_CAPACITY);
    }

    #[test]
    fn atom_cache_hit_queue_compacts_after_repeated_reuse() {
        let mut cache = TextLayoutCache::new();
        let _ = cache.intern_text("content row");
        for _ in 0..=TEXT_ATOM_CACHE_CAPACITY.saturating_mul(2) {
            let _ = cache.intern_text("content row");
        }

        assert_eq!(cache.atom_cache.len(), 1);
        assert!(cache.atom_cache_order.len() <= TEXT_ATOM_CACHE_CAPACITY);
    }

    #[test]
    fn layout_cache_eviction_keeps_recently_used_entries() {
        let mut cache = TextLayoutCache::new();
        for index in 0..TEXT_LAYOUT_CACHE_CAPACITY {
            let key = layout_key(&format!("label-{index}"));
            cache
                .layout_cache
                .insert(key.clone(), TextLayout::empty_for(key.text.as_ref()));
            cache.touch_layout_cache_key(&key);
        }

        let hot_key = layout_key("label-0");
        cache.touch_layout_cache_key(&hot_key);
        cache.evict_stale_layouts();

        let fresh_key = layout_key("label-fresh");
        cache.layout_cache.insert(
            fresh_key.clone(),
            TextLayout::empty_for(fresh_key.text.as_ref()),
        );
        cache.touch_layout_cache_key(&fresh_key);

        assert!(cache.layout_cache.contains_key(&hot_key));
        assert!(cache.layout_cache.contains_key(&fresh_key));
        assert!(cache.layout_cache.len() <= TEXT_LAYOUT_CACHE_CAPACITY);
        assert_eq!(cache.text_layout_evictions, 1);
    }

    #[test]
    fn layout_cache_hit_queue_compacts_after_repeated_reuse() {
        let mut cache = TextLayoutCache::new();
        let key = layout_key("content row");
        cache
            .layout_cache
            .insert(key.clone(), TextLayout::empty_for(key.text.as_ref()));
        cache.touch_layout_cache_key(&key);

        for _ in 0..=TEXT_LAYOUT_CACHE_CAPACITY.saturating_mul(2) {
            cache.touch_layout_cache_key(&key);
        }

        assert_eq!(cache.layout_cache.len(), 1);
        assert!(cache.layout_cache_order.len() <= TEXT_LAYOUT_CACHE_CAPACITY);
    }
}
