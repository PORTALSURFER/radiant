//! Bounded retained paragraph shaping and width/view projection caches.

mod atom;

use super::{
    ParagraphSnapshot, TextLayout, TextLayoutKey, TextViewKey,
    font::NativeFontStack,
    layout::{compute_compatibility_paragraph, compute_shaped_paragraph},
    model::LINE_BREAK_POLICY_ID,
};
use crate::gui::paint::TextAlign;
use crate::widgets::TextWrap;
use atom::TextAtomCache;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::sync::Arc;

const SHAPE_CACHE_ENTRY_BUDGET: usize = 512;
const SHAPE_CACHE_BYTE_BUDGET: usize = 8 * 1024 * 1024;
const VIEW_CACHE_ENTRY_BUDGET: usize = 1_024;
const VIEW_CACHE_BYTE_BUDGET: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextLayoutProfileCounters {
    pub shape: TextCacheProfileCounters,
    pub width: TextCacheProfileCounters,
    pub view: TextCacheProfileCounters,
    /// Compatibility alias for the historical no-width layout stage.
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
    shape_cache: HashMap<TextLayoutKey, CachedShape>,
    shape_cache_order: VecDeque<(TextLayoutKey, u64)>,
    shape_cache_clock: u64,
    shape_cache_bytes: usize,
    view_cache: HashMap<TextViewKey, CachedTextLayout>,
    view_cache_order: VecDeque<(TextViewKey, u64)>,
    view_cache_clock: u64,
    view_cache_bytes: usize,
    // A result larger than the bounded view budget is retained only for the
    // current borrowing operation. It is not part of the cache or its byte
    // accounting, so a pathological paragraph cannot grow retained state.
    transient_layout: Option<TextLayout>,
    #[cfg(test)]
    view_cache_byte_budget_override: Option<usize>,
    atom_cache: TextAtomCache,
    shape_profile: TextCacheProfileCounters,
    width_profile: TextCacheProfileCounters,
    view_profile: TextCacheProfileCounters,
    layout_profile: TextCacheProfileCounters,
    quality_profile: TextQualityProfileCounters,
}

#[derive(Clone, Debug)]
struct CachedShape {
    shape: Arc<super::ShapedParagraph>,
    stamp: u64,
    bytes: usize,
}

#[derive(Clone, Debug)]
struct CachedTextLayout {
    layout: TextLayout,
    stamp: u64,
    bytes: usize,
}

impl TextLayoutCache {
    pub(super) fn new() -> Self {
        Self {
            shape_cache: HashMap::with_capacity(SHAPE_CACHE_ENTRY_BUDGET / 2),
            shape_cache_order: VecDeque::with_capacity(SHAPE_CACHE_ENTRY_BUDGET),
            shape_cache_clock: 0,
            shape_cache_bytes: 0,
            view_cache: HashMap::with_capacity(VIEW_CACHE_ENTRY_BUDGET / 2),
            view_cache_order: VecDeque::with_capacity(VIEW_CACHE_ENTRY_BUDGET),
            view_cache_clock: 0,
            view_cache_bytes: 0,
            transient_layout: None,
            #[cfg(test)]
            view_cache_byte_budget_override: None,
            atom_cache: TextAtomCache::new(),
            shape_profile: TextCacheProfileCounters::default(),
            width_profile: TextCacheProfileCounters::default(),
            view_profile: TextCacheProfileCounters::default(),
            layout_profile: TextCacheProfileCounters::default(),
            quality_profile: TextQualityProfileCounters::default(),
        }
    }

    pub(super) fn layout_for<'a>(
        &'a mut self,
        font_stack: &mut NativeFontStack,
        text: &str,
        font_size: f32,
    ) -> Option<&'a TextLayout> {
        self.layout_for_view(
            font_stack,
            text,
            font_size,
            None,
            TextAlign::Left,
            TextWrap::None,
        )
    }

    pub(super) fn layout_for_view<'a>(
        &'a mut self,
        font_stack: &mut NativeFontStack,
        text: &str,
        font_size: f32,
        available_width: Option<f32>,
        align: TextAlign,
        wrap: TextWrap,
    ) -> Option<&'a TextLayout> {
        if !font_size.is_finite() || font_size <= 0.0 {
            return None;
        }
        self.transient_layout = None;
        let text_atom = self.intern_text(text);
        let initial_key = TextLayoutKey {
            text: text_atom,
            font_size_bits: font_size.to_bits(),
            font_generation: font_stack.generation(),
        };
        let (shape_key, shape) = self.shape_for(font_stack, initial_key, font_size);
        let no_width =
            available_width.is_none() && align == TextAlign::Left && wrap == TextWrap::None;
        let width_bits = match available_width {
            None => u32::MAX,
            Some(width) if width.is_finite() && width >= 0.0 => width.to_bits(),
            Some(_) => 0,
        };
        let key = TextViewKey {
            shape: shape_key,
            width_bits,
            align,
            wrap,
            break_policy_id: LINE_BREAK_POLICY_ID,
        };
        self.compact_view_cache_order_if_needed();
        if self.view_cache.contains_key(&key) {
            let stamp =
                record_key_access(&mut self.view_cache_clock, &mut self.view_cache_order, &key);
            let entry = self.view_cache.get_mut(&key)?;
            entry.stamp = stamp;
            self.view_profile.hits = self.view_profile.hits.saturating_add(1);
            self.width_profile.hits = self.width_profile.hits.saturating_add(1);
            if no_width {
                self.layout_profile.hits = self.layout_profile.hits.saturating_add(1);
            }
            let entry = self.view_cache.get(&key)?;
            let quality = quality_values(&entry.layout);
            self.quality_profile.record_values(quality);
            return Some(&entry.layout);
        }

        self.view_profile.misses = self.view_profile.misses.saturating_add(1);
        self.width_profile.misses = self.width_profile.misses.saturating_add(1);
        if no_width {
            self.layout_profile.misses = self.layout_profile.misses.saturating_add(1);
        }
        let snapshot = ParagraphSnapshot::from_shaped(shape, available_width, align);
        let layout = TextLayout::from_snapshot(snapshot);
        self.quality_profile.record_layout(&layout);
        let bytes = layout.estimated_bytes();
        if bytes <= self.view_cache_byte_budget() {
            self.evict_view_cache_until(bytes);
            let stamp =
                record_key_access(&mut self.view_cache_clock, &mut self.view_cache_order, &key);
            self.view_cache_bytes = self.view_cache_bytes.saturating_add(bytes);
            let entry = self
                .view_cache
                .entry(key.clone())
                .or_insert(CachedTextLayout {
                    layout,
                    stamp,
                    bytes,
                });
            entry.stamp = stamp;
            return self.view_cache.get(&key).map(|entry| &entry.layout);
        }
        self.transient_layout = Some(layout);
        self.transient_layout.as_ref()
    }

    fn shape_for(
        &mut self,
        font_stack: &mut NativeFontStack,
        initial_key: TextLayoutKey,
        font_size: f32,
    ) -> (TextLayoutKey, Arc<super::ShapedParagraph>) {
        self.compact_shape_cache_order_if_needed();
        if let Some(entry) = self.shape_cache.get_mut(&initial_key) {
            let stamp = record_key_access(
                &mut self.shape_cache_clock,
                &mut self.shape_cache_order,
                &initial_key,
            );
            entry.stamp = stamp;
            self.shape_profile.hits = self.shape_profile.hits.saturating_add(1);
            return (initial_key, entry.shape.clone());
        }
        self.shape_profile.misses = self.shape_profile.misses.saturating_add(1);
        let source = initial_key.text.clone();
        let shape = compute_shaped_paragraph(font_stack, source.clone(), font_size)
            .unwrap_or_else(|_| compute_compatibility_paragraph(font_stack, source, font_size));
        let key = TextLayoutKey {
            text: initial_key.text,
            font_size_bits: initial_key.font_size_bits,
            font_generation: font_stack.generation(),
        };
        let bytes = shape.estimated_bytes();
        if bytes <= SHAPE_CACHE_BYTE_BUDGET {
            self.evict_shape_cache_until(bytes);
            let stamp = record_key_access(
                &mut self.shape_cache_clock,
                &mut self.shape_cache_order,
                &key,
            );
            self.shape_cache_bytes = self.shape_cache_bytes.saturating_add(bytes);
            self.shape_cache.insert(
                key.clone(),
                CachedShape {
                    shape: shape.clone(),
                    stamp,
                    bytes,
                },
            );
        }
        (key, shape)
    }

    pub(super) fn take_profile_counters(&mut self) -> TextLayoutProfileCounters {
        TextLayoutProfileCounters {
            shape: mem::take(&mut self.shape_profile),
            width: mem::take(&mut self.width_profile),
            view: mem::take(&mut self.view_profile),
            layout: mem::take(&mut self.layout_profile),
            atom: self.atom_cache.take_profile_counters(),
            quality: mem::take(&mut self.quality_profile),
        }
    }

    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.atom_cache.intern_text(text)
    }

    #[cfg(test)]
    fn record_view_cache_hit<'a>(&'a mut self, key: &TextViewKey) -> Option<&'a TextLayout> {
        self.compact_view_cache_order_if_needed();
        let stamp = record_key_access(&mut self.view_cache_clock, &mut self.view_cache_order, key);
        let cached_layout = self.view_cache.get_mut(key)?;
        cached_layout.stamp = stamp;
        self.view_profile.hits = self.view_profile.hits.saturating_add(1);
        self.width_profile.hits = self.width_profile.hits.saturating_add(1);
        if is_no_width_key(key) {
            self.layout_profile.hits = self.layout_profile.hits.saturating_add(1);
        }
        let entry = self.view_cache.get(key)?;
        let quality = quality_values(&entry.layout);
        self.quality_profile.record_values(quality);
        Some(&entry.layout)
    }

    #[cfg(test)]
    /// Record view-cache recency without reallocating the cached layout.
    pub(super) fn touch_view_cache_key(&mut self, key: &TextViewKey) {
        let stamp = record_key_access(&mut self.view_cache_clock, &mut self.view_cache_order, key);
        if let Some(entry) = self.view_cache.get_mut(key) {
            entry.stamp = stamp;
        }
    }

    fn evict_shape_cache_until(&mut self, incoming: usize) {
        while self.shape_cache.len() >= SHAPE_CACHE_ENTRY_BUDGET
            || self.shape_cache_bytes.saturating_add(incoming) > SHAPE_CACHE_BYTE_BUDGET
        {
            let Some((candidate, queued_stamp)) = self.shape_cache_order.pop_front() else {
                break;
            };
            let Some(entry) = self.shape_cache.get(&candidate) else {
                continue;
            };
            if entry.stamp != queued_stamp {
                continue;
            }
            if let Some(entry) = self.shape_cache.remove(&candidate) {
                self.shape_cache_bytes = self.shape_cache_bytes.saturating_sub(entry.bytes);
                self.shape_profile.evictions = self.shape_profile.evictions.saturating_add(1);
            }
        }
    }

    fn evict_view_cache_until(&mut self, incoming: usize) {
        let byte_budget = self.view_cache_byte_budget();
        while self.view_cache.len() >= VIEW_CACHE_ENTRY_BUDGET
            || self.view_cache_bytes.saturating_add(incoming) > byte_budget
        {
            let Some((candidate, queued_stamp)) = self.view_cache_order.pop_front() else {
                break;
            };
            let Some(entry) = self.view_cache.get(&candidate) else {
                continue;
            };
            if entry.stamp != queued_stamp {
                continue;
            }
            if let Some(entry) = self.view_cache.remove(&candidate) {
                self.view_cache_bytes = self.view_cache_bytes.saturating_sub(entry.bytes);
                self.view_profile.evictions = self.view_profile.evictions.saturating_add(1);
                self.width_profile.evictions = self.width_profile.evictions.saturating_add(1);
                if is_no_width_key(&candidate) {
                    self.layout_profile.evictions = self.layout_profile.evictions.saturating_add(1);
                }
            }
        }
    }

    fn compact_shape_cache_order_if_needed(&mut self) {
        if self.shape_cache_order.len() <= SHAPE_CACHE_ENTRY_BUDGET.saturating_mul(2) {
            return;
        }
        let mut compacted = VecDeque::with_capacity(self.shape_cache.len());
        for (key, stamp) in mem::take(&mut self.shape_cache_order) {
            if self
                .shape_cache
                .get(&key)
                .is_some_and(|entry| entry.stamp == stamp)
            {
                compacted.push_back((key, stamp));
            }
        }
        self.shape_cache_order = compacted;
    }

    fn view_cache_byte_budget(&self) -> usize {
        #[cfg(test)]
        if let Some(budget) = self.view_cache_byte_budget_override {
            return budget;
        }
        VIEW_CACHE_BYTE_BUDGET
    }

    fn compact_view_cache_order_if_needed(&mut self) {
        if self.view_cache_order.len() <= VIEW_CACHE_ENTRY_BUDGET.saturating_mul(2) {
            return;
        }
        let mut compacted = VecDeque::with_capacity(self.view_cache.len());
        for (key, stamp) in mem::take(&mut self.view_cache_order) {
            if self
                .view_cache
                .get(&key)
                .is_some_and(|entry| entry.stamp == stamp)
            {
                compacted.push_back((key, stamp));
            }
        }
        self.view_cache_order = compacted;
    }
}

fn record_key_access<K: Clone + Eq>(
    clock: &mut u64,
    order: &mut VecDeque<(K, u64)>,
    key: &K,
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
        self.record_values(quality_values(layout));
    }

    fn record_values(&mut self, values: (u64, u64, u64, u64)) {
        self.unsupported_shaping_runs = self.unsupported_shaping_runs.saturating_add(values.0);
        self.unsupported_shaping_scalars =
            self.unsupported_shaping_scalars.saturating_add(values.1);
        self.fallback_glyphs = self.fallback_glyphs.saturating_add(values.2);
        self.missing_glyphs = self.missing_glyphs.saturating_add(values.3);
    }
}

fn quality_values(layout: &TextLayout) -> (u64, u64, u64, u64) {
    (
        layout.unsupported_shaping_runs,
        layout.unsupported_shaping_scalars,
        layout.fallback_glyphs,
        layout.missing_glyphs,
    )
}

fn is_no_width_key(key: &TextViewKey) -> bool {
    key.width_bits == u32::MAX && key.align == TextAlign::Left && key.wrap == TextWrap::None
}

#[cfg(test)]
#[path = "cache/tests.rs"]
mod tests;
