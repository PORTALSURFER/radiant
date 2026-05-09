//! Retained cache for text-line micro-layout.

use crate::gui::text_layout::{TextLineInsets, TextLineMode};
use crate::gui::types::Rect;
use std::sync::{Mutex, OnceLock};

const CACHE_LIMIT: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TextLineKey {
    mode: TextLineMode,
    family_id: u64,
    bounds: RectKey,
    insets: InsetsKey,
    font_size: u32,
    min_top_inset: u32,
}

impl TextLineKey {
    pub(super) fn new(
        mode: TextLineMode,
        family_id: u64,
        bounds: Rect,
        insets: TextLineInsets,
        font_size: f32,
        min_top_inset: f32,
    ) -> Self {
        Self {
            mode,
            family_id,
            bounds: RectKey::from(bounds),
            insets: InsetsKey::from(insets),
            font_size: font_size.to_bits(),
            min_top_inset: min_top_inset.to_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TextLineEntry {
    key: TextLineKey,
    rect: Rect,
}

#[derive(Debug, Default)]
struct TextLineCache {
    entries: Vec<TextLineEntry>,
    #[cfg(test)]
    misses: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RectKey {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InsetsKey {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

pub(super) fn cached_text_line(key: TextLineKey, compute: impl FnOnce() -> Rect) -> Rect {
    let mut cache = cache()
        .lock()
        .expect("text-line micro-layout cache poisoned");
    if let Some(index) = cache.entries.iter().position(|entry| entry.key == key) {
        let entry = cache.entries.remove(index);
        cache.entries.push(entry);
        return entry.rect;
    }

    let rect = compute();
    #[cfg(test)]
    {
        cache.misses += 1;
    }
    if cache.entries.len() == CACHE_LIMIT {
        cache.entries.remove(0);
    }
    cache.entries.push(TextLineEntry { key, rect });
    rect
}

fn cache() -> &'static Mutex<TextLineCache> {
    static CACHE: OnceLock<Mutex<TextLineCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(TextLineCache::default()))
}

impl From<Rect> for RectKey {
    fn from(rect: Rect) -> Self {
        Self {
            min_x: rect.min.x.to_bits(),
            min_y: rect.min.y.to_bits(),
            max_x: rect.max.x.to_bits(),
            max_y: rect.max.y.to_bits(),
        }
    }
}

impl From<TextLineInsets> for InsetsKey {
    fn from(insets: TextLineInsets) -> Self {
        Self {
            left: insets.left.to_bits(),
            right: insets.right.to_bits(),
            top: insets.top.to_bits(),
            bottom: insets.bottom.to_bits(),
        }
    }
}

#[cfg(test)]
pub(super) fn reset_for_test() {
    let mut cache = cache()
        .lock()
        .expect("text-line micro-layout cache poisoned");
    cache.entries.clear();
    cache.misses = 0;
}

#[cfg(test)]
pub(super) fn misses_for_test() -> usize {
    cache()
        .lock()
        .expect("text-line micro-layout cache poisoned")
        .misses
}
