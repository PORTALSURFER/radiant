//! Owned cache for text-line micro-layout.

use crate::gui::text_layout::{TextLineInsets, TextLineMode};
use crate::gui::types::Rect;

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

/// Bounded, renderer-owned cache for deterministic single-line text placement.
///
/// Keep one cache per renderer, adapter, or layout owner that wants retained
/// text-line geometry. This avoids a process-global lock while still letting
/// hot paths reuse repeated label placement calculations.
#[derive(Debug)]
pub struct TextLineLayoutCache {
    entries: Vec<TextLineEntry>,
    limit: usize,
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

impl TextLineLayoutCache {
    /// Create a bounded text-line geometry cache with Radiant's default capacity.
    pub fn new() -> Self {
        Self::with_capacity(CACHE_LIMIT)
    }

    /// Create a bounded text-line geometry cache with a custom capacity.
    pub fn with_capacity(limit: usize) -> Self {
        let limit = limit.clamp(1, CACHE_LIMIT);
        Self {
            entries: Vec::with_capacity(limit),
            limit,
            #[cfg(test)]
            misses: 0,
        }
    }

    /// Return the number of cached text-line placements currently retained.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether this cache currently holds no retained placements.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drop all retained text-line placements.
    pub fn clear(&mut self) {
        self.entries.clear();
        #[cfg(test)]
        {
            self.misses = 0;
        }
    }

    pub(super) fn cached_text_line(
        &mut self,
        key: TextLineKey,
        compute: impl FnOnce() -> Rect,
    ) -> Rect {
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            let entry = self.entries.remove(index);
            self.entries.push(entry);
            return entry.rect;
        }

        let rect = compute();
        #[cfg(test)]
        {
            self.misses += 1;
        }
        if self.entries.len() == self.limit {
            self.entries.remove(0);
        }
        self.entries.push(TextLineEntry { key, rect });
        rect
    }

    #[cfg(test)]
    pub(super) fn misses_for_test(&self) -> usize {
        self.misses
    }
}

impl Default for TextLineLayoutCache {
    fn default() -> Self {
        Self::new()
    }
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
