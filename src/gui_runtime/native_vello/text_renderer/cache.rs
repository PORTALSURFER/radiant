//! Layout-cache and atom-cache helpers for the native text renderer.

mod atom;

use super::{TextLayout, TextLayoutKey, layout::compute_layout};
use atom::TextAtomCache;
use std::collections::{HashMap, VecDeque, hash_map::Entry};
use std::mem;
use std::sync::Arc;
use vello::peniko::FontData;

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 2_048;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextLayoutProfileCounters {
    pub layout: TextCacheProfileCounters,
    pub atom: TextCacheProfileCounters,
    pub quality: TextQualityProfileCounters,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextCacheProfileCounters {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextQualityProfileCounters {
    pub unsupported_shaping_runs: u64,
    pub unsupported_shaping_scalars: u64,
    pub fallback_glyphs: u64,
    pub missing_glyphs: u64,
}

pub(super) struct TextLayoutCache {
    layout_cache: HashMap<TextLayoutKey, CachedTextLayout>,
    layout_cache_order: VecDeque<(TextLayoutKey, u64)>,
    layout_cache_clock: u64,
    atom_cache: TextAtomCache,
    layout_profile: TextCacheProfileCounters,
    quality_profile: TextQualityProfileCounters,
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
            layout_profile: TextCacheProfileCounters::default(),
            quality_profile: TextQualityProfileCounters::default(),
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

        self.compact_layout_cache_order_if_needed();
        if self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            self.evict_stale_layouts();
        }
        match self.layout_cache.entry(key) {
            Entry::Occupied(occupied) => {
                let stamp = record_layout_cache_access(
                    &mut self.layout_cache_clock,
                    &mut self.layout_cache_order,
                    occupied.key(),
                );
                let cached_layout = occupied.into_mut();
                cached_layout.stamp = stamp;
                self.layout_profile.hits = self.layout_profile.hits.saturating_add(1);
                self.quality_profile.record_layout(&cached_layout.layout);
                Some(&cached_layout.layout)
            }
            Entry::Vacant(vacant) => {
                self.layout_profile.misses = self.layout_profile.misses.saturating_add(1);
                let layout = compute_layout(font, text, font_size)?;
                self.quality_profile.record_layout(&layout);
                let stamp = record_layout_cache_access(
                    &mut self.layout_cache_clock,
                    &mut self.layout_cache_order,
                    vacant.key(),
                );
                let entry = vacant.insert(CachedTextLayout { layout, stamp });
                Some(&entry.layout)
            }
        }
    }

    pub(super) fn take_profile_counters(&mut self) -> TextLayoutProfileCounters {
        TextLayoutProfileCounters {
            layout: std::mem::take(&mut self.layout_profile),
            atom: self.atom_cache.take_profile_counters(),
            quality: std::mem::take(&mut self.quality_profile),
        }
    }

    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.atom_cache.intern_text(text)
    }

    #[cfg(test)]
    fn record_layout_cache_hit<'a>(&'a mut self, key: &TextLayoutKey) -> Option<&'a TextLayout> {
        self.compact_layout_cache_order_if_needed();
        let stamp = record_layout_cache_access(
            &mut self.layout_cache_clock,
            &mut self.layout_cache_order,
            key,
        );
        let cached_layout = self.layout_cache.get_mut(key)?;
        cached_layout.stamp = stamp;
        self.layout_profile.hits = self.layout_profile.hits.saturating_add(1);
        self.quality_profile.record_layout(&cached_layout.layout);
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

    #[cfg(test)]
    fn record_layout_cache_access(&mut self, key: TextLayoutKey) -> u64 {
        let stamp = record_layout_cache_access(
            &mut self.layout_cache_clock,
            &mut self.layout_cache_order,
            &key,
        );
        self.compact_layout_cache_order_if_needed();
        stamp
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
                self.layout_profile.evictions = self.layout_profile.evictions.saturating_add(1);
            }
        }
    }
}

fn record_layout_cache_access(
    clock: &mut u64,
    order: &mut VecDeque<(TextLayoutKey, u64)>,
    key: &TextLayoutKey,
) -> u64 {
    *clock = clock.saturating_add(1);
    let stamp = *clock;
    if let Some((queued_key, queued_stamp)) = order.back_mut()
        && queued_key == key
    {
        *queued_stamp = stamp;
        return stamp;
    }
    order.push_back((key.clone(), stamp));
    stamp
}

impl TextQualityProfileCounters {
    fn record_layout(&mut self, layout: &TextLayout) {
        self.unsupported_shaping_runs = self
            .unsupported_shaping_runs
            .saturating_add(layout.unsupported_shaping_runs);
        self.unsupported_shaping_scalars = self
            .unsupported_shaping_scalars
            .saturating_add(layout.unsupported_shaping_scalars);
        self.fallback_glyphs = self.fallback_glyphs.saturating_add(layout.fallback_glyphs);
        self.missing_glyphs = self.missing_glyphs.saturating_add(layout.missing_glyphs);
    }
}

#[cfg(test)]
#[path = "cache/tests.rs"]
mod tests;
