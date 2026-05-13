//! Layout-cache and atom-cache helpers for the native text renderer.

mod atom;

use super::{TextLayout, TextLayoutKey, layout::compute_layout};
use atom::TextAtomCache;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::Arc;
use vello::peniko::FontData;

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;

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
    layout_cache: HashMap<TextLayoutKey, CachedTextLayout>,
    layout_cache_order: VecDeque<(TextLayoutKey, u64)>,
    layout_cache_clock: u64,
    atom_cache: TextAtomCache,
    text_layout_hits: u64,
    text_layout_misses: u64,
    text_layout_evictions: u64,
}

#[derive(Clone, Debug)]
struct CachedTextLayout {
    layout: TextLayout,
    stamp: u64,
}

impl TextLayoutCache {
    pub(super) fn new() -> Self {
        Self {
            layout_cache: HashMap::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY / 2),
            layout_cache_order: VecDeque::with_capacity(TEXT_LAYOUT_CACHE_CAPACITY),
            layout_cache_clock: 0,
            atom_cache: TextAtomCache::new(),
            text_layout_hits: 0,
            text_layout_misses: 0,
            text_layout_evictions: 0,
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

        if self.layout_cache.contains_key(&key) {
            return self.record_layout_cache_hit(&key);
        }

        self.text_layout_misses = self.text_layout_misses.saturating_add(1);

        self.evict_stale_layouts();

        let layout = compute_layout(font, text, font_size)?;
        self.touch_layout_cache_key(&key);
        self.layout_cache.insert(
            key.clone(),
            CachedTextLayout {
                layout,
                stamp: self.layout_cache_clock,
            },
        );
        self.layout_cache.get(&key).map(|entry| &entry.layout)
    }

    pub(super) fn take_profile_counters(&mut self) -> TextLayoutProfileCounters {
        let atom_counters = self.atom_cache.take_profile_counters();
        let counters = TextLayoutProfileCounters {
            layout_hits: self.text_layout_hits,
            layout_misses: self.text_layout_misses,
            layout_evictions: self.text_layout_evictions,
            atom_hits: atom_counters.hits,
            atom_misses: atom_counters.misses,
            atom_evictions: atom_counters.evictions,
        };
        self.text_layout_hits = 0;
        self.text_layout_misses = 0;
        self.text_layout_evictions = 0;
        counters
    }

    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.atom_cache.intern_text(text)
    }

    fn record_layout_cache_hit<'a>(&'a mut self, key: &TextLayoutKey) -> Option<&'a TextLayout> {
        self.layout_cache_clock = self.layout_cache_clock.saturating_add(1);
        let stamp = self.layout_cache_clock;
        self.compact_layout_cache_order_if_needed();
        self.layout_cache_order.push_back((key.clone(), stamp));
        let cached_layout = self.layout_cache.get_mut(key)?;
        cached_layout.stamp = stamp;
        self.text_layout_hits = self.text_layout_hits.saturating_add(1);
        Some(&cached_layout.layout)
    }

    /// Record layout-cache recency without reallocating the cached layout.
    pub(super) fn touch_layout_cache_key(&mut self, key: &TextLayoutKey) {
        self.layout_cache_clock = self.layout_cache_clock.saturating_add(1);
        let stamp = self.layout_cache_clock;
        if let Some(entry) = self.layout_cache.get_mut(key) {
            entry.stamp = stamp;
        }
        self.layout_cache_order.push_back((key.clone(), stamp));
        self.compact_layout_cache_order_if_needed();
    }

    /// Compact queued layout-order metadata after repeated cache hits append stale stamps.
    fn compact_layout_cache_order_if_needed(&mut self) {
        if self.layout_cache_order.len() <= TEXT_LAYOUT_CACHE_CAPACITY.saturating_mul(2) {
            return;
        }
        let mut compacted = VecDeque::with_capacity(self.layout_cache.len());
        for (key, queued_stamp) in mem::take(&mut self.layout_cache_order) {
            if self
                .layout_cache
                .get(&key)
                .is_some_and(|entry| entry.stamp == queued_stamp)
            {
                compacted.push_back((key, queued_stamp));
            }
        }
        self.layout_cache_order = compacted;
    }

    /// Evict stale layout-cache entries by last-use stamp so hot text survives churn.
    pub(super) fn evict_stale_layouts(&mut self) {
        while self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            let Some((candidate, queued_stamp)) = self.layout_cache_order.pop_front() else {
                break;
            };
            let Some(entry) = self.layout_cache.get(&candidate) else {
                continue;
            };
            if entry.stamp != queued_stamp {
                continue;
            }
            if self.layout_cache.remove(&candidate).is_some() {
                self.text_layout_evictions = self.text_layout_evictions.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cached_layout(text: &str, stamp: u64) -> CachedTextLayout {
        CachedTextLayout {
            layout: TextLayout::empty_for(text),
            stamp,
        }
    }

    fn layout_key(label: &str) -> TextLayoutKey {
        TextLayoutKey {
            text: Arc::from(label),
            font_size_bits: 12.0_f32.to_bits(),
        }
    }

    #[test]
    fn layout_cache_eviction_keeps_recently_used_entries() {
        let mut cache = TextLayoutCache::new();
        for index in 0..TEXT_LAYOUT_CACHE_CAPACITY {
            let key = layout_key(&format!("label-{index}"));
            cache
                .layout_cache
                .insert(key.clone(), cached_layout(key.text.as_ref(), 0));
            cache.touch_layout_cache_key(&key);
        }

        let hot_key = layout_key("label-0");
        cache.touch_layout_cache_key(&hot_key);
        cache.evict_stale_layouts();

        let fresh_key = layout_key("label-fresh");
        cache
            .layout_cache
            .insert(fresh_key.clone(), cached_layout(fresh_key.text.as_ref(), 0));
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
            .insert(key.clone(), cached_layout(key.text.as_ref(), 0));
        cache.touch_layout_cache_key(&key);

        for _ in 0..=TEXT_LAYOUT_CACHE_CAPACITY.saturating_mul(2) {
            cache.touch_layout_cache_key(&key);
        }

        assert_eq!(cache.layout_cache.len(), 1);
        assert!(cache.layout_cache_order.len() <= TEXT_LAYOUT_CACHE_CAPACITY);
    }
}
