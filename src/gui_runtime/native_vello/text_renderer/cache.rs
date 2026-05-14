//! Layout-cache and atom-cache helpers for the native text renderer.

mod atom;

use super::{TextLayout, TextLayoutKey, layout::compute_layout};
use atom::TextAtomCache;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::Arc;
use vello::peniko::FontData;

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;

#[cfg(test)]
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
        let stamp = self.record_layout_cache_access(key.clone());
        let entry = self
            .layout_cache
            .entry(key)
            .or_insert(CachedTextLayout { layout, stamp });
        Some(&entry.layout)
    }

    #[cfg(test)]
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
        let stamp = self.next_layout_cache_stamp();
        self.compact_layout_cache_order_if_needed();
        self.layout_cache_order.push_back((key.clone(), stamp));
        let cached_layout = self.layout_cache.get_mut(key)?;
        cached_layout.stamp = stamp;
        self.text_layout_hits = self.text_layout_hits.saturating_add(1);
        Some(&cached_layout.layout)
    }

    #[cfg(test)]
    /// Record layout-cache recency without reallocating the cached layout.
    pub(super) fn touch_layout_cache_key(&mut self, key: &TextLayoutKey) {
        let stamp = self.record_layout_cache_access(key.clone());
        if let Some(entry) = self.layout_cache.get_mut(key) {
            entry.stamp = stamp;
        }
    }

    fn record_layout_cache_access(&mut self, key: TextLayoutKey) -> u64 {
        let stamp = self.next_layout_cache_stamp();
        self.layout_cache_order.push_back((key, stamp));
        self.compact_layout_cache_order_if_needed();
        stamp
    }

    fn next_layout_cache_stamp(&mut self) -> u64 {
        self.layout_cache_clock = self.layout_cache_clock.saturating_add(1);
        self.layout_cache_clock
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
#[path = "cache/tests.rs"]
mod tests;
