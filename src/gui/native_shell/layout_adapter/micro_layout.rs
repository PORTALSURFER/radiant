//! Retained micro-layout helpers for simple native-shell text-line placement.

use crate::gui::types::{Point, Rect};
use std::sync::{Mutex, OnceLock};

const CACHE_LIMIT: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextLineMode {
    Center,
    Top,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TextLineKey {
    mode: TextLineMode,
    family_id: u64,
    bounds: RectKey,
    insets: InsetsKey,
    font_size: u32,
    min_top_inset: u32,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TextLineInsets {
    /// Left inset applied before line placement.
    pub left: f32,
    /// Right inset applied before line placement.
    pub right: f32,
    /// Top inset applied before line placement.
    pub top: f32,
    /// Bottom inset applied before line placement.
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InsetsKey {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl TextLineInsets {
    /// Build equal horizontal and vertical insets.
    pub(super) fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    /// Build horizontal-only insets.
    pub(super) fn horizontal(horizontal: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: 0.0,
            bottom: 0.0,
        }
    }
}

/// Resolve a vertically centered text-line rect through the retained micro-layout cache.
pub(super) fn centered_text_line(
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    family_id: u64,
) -> Rect {
    text_line(
        bounds,
        font_size,
        insets,
        min_top_inset,
        family_id,
        TextLineMode::Center,
    )
}

/// Resolve a top-aligned text-line rect through the retained micro-layout cache.
pub(super) fn top_text_line(
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    family_id: u64,
) -> Rect {
    text_line(bounds, font_size, insets, 0.0, family_id, TextLineMode::Top)
}

fn text_line(
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    family_id: u64,
    mode: TextLineMode,
) -> Rect {
    let empty = empty_rect(bounds);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let key = TextLineKey {
        mode,
        family_id,
        bounds: RectKey::from(bounds),
        insets: InsetsKey::from(insets),
        font_size: font_size.to_bits(),
        min_top_inset: min_top_inset.to_bits(),
    };
    cached_text_line(key, || {
        compute_text_line(bounds, font_size, insets, min_top_inset, mode)
    })
}

fn cached_text_line(key: TextLineKey, compute: impl FnOnce() -> Rect) -> Rect {
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

fn compute_text_line(
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    mode: TextLineMode,
) -> Rect {
    let inner = inset_rect(bounds, insets);
    if inner.width() <= 0.0 || inner.height() <= 0.0 {
        return empty_rect(bounds);
    }
    let line_height = font_size.max(1.0);
    let line = match mode {
        TextLineMode::Center => {
            let min_y = inner.min.y + ((inner.height() - line_height) * 0.5).max(0.0);
            Rect::from_min_max(
                Point::new(inner.min.x, min_y),
                Point::new(inner.max.x, min_y + line_height),
            )
        }
        TextLineMode::Top => Rect::from_min_max(
            inner.min,
            Point::new(inner.max.x, inner.min.y + line_height),
        ),
    };
    let line = clamp_min_top(line, inner, min_top_inset.max(0.0));
    clamp_rect_to_bounds(line, inner)
}

fn clamp_min_top(line: Rect, bounds: Rect, min_top_inset: f32) -> Rect {
    let min_y = (bounds.min.y + min_top_inset).min(bounds.max.y);
    if line.min.y >= min_y {
        return line;
    }
    Rect::from_min_max(
        Point::new(line.min.x, min_y),
        Point::new(
            line.max.x,
            (min_y + line.height().max(0.0)).min(bounds.max.y),
        ),
    )
}

fn inset_rect(rect: Rect, insets: TextLineInsets) -> Rect {
    let min_x = (rect.min.x + insets.left.max(0.0)).min(rect.max.x);
    let max_x = (rect.max.x - insets.right.max(0.0)).max(min_x);
    let min_y = (rect.min.y + insets.top.max(0.0)).min(rect.max.y);
    let max_y = (rect.max.y - insets.bottom.max(0.0)).max(min_y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return empty_rect(bounds);
    }
    Rect::from_min_max(min, max)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
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
mod tests {
    use super::*;
    use std::sync::MutexGuard;

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("text-line micro-layout test lock poisoned")
    }

    fn reset_cache() {
        let mut cache = cache()
            .lock()
            .expect("text-line micro-layout cache poisoned");
        cache.entries.clear();
        cache.misses = 0;
    }

    fn cache_misses() -> usize {
        cache()
            .lock()
            .expect("text-line micro-layout cache poisoned")
            .misses
    }

    #[test]
    fn centered_line_reuses_cached_geometry_for_identical_inputs() {
        let _guard = test_guard();
        reset_cache();
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
        let first = centered_text_line(bounds, 12.0, TextLineInsets::symmetric(8.0, 3.0), 0.0, 1);
        let second = centered_text_line(bounds, 12.0, TextLineInsets::symmetric(8.0, 3.0), 0.0, 1);
        assert_eq!(first, second);
        assert_eq!(cache_misses(), 1);
    }

    #[test]
    fn centered_line_invalidates_when_font_bounds_or_insets_change() {
        let _guard = test_guard();
        reset_cache();
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
        let base = centered_text_line(bounds, 12.0, TextLineInsets::symmetric(8.0, 3.0), 0.0, 2);
        let taller = centered_text_line(bounds, 14.0, TextLineInsets::symmetric(8.0, 3.0), 0.0, 2);
        let wider = centered_text_line(
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(230.0, 60.0)),
            12.0,
            TextLineInsets::symmetric(8.0, 3.0),
            0.0,
            2,
        );
        let inset = centered_text_line(bounds, 12.0, TextLineInsets::symmetric(10.0, 3.0), 0.0, 2);
        assert_ne!(base, taller);
        assert_ne!(base, wider);
        assert_ne!(base, inset);
        assert_eq!(cache_misses(), 4);
    }

    #[test]
    fn top_line_uses_top_edge_after_horizontal_inset() {
        let _guard = test_guard();
        reset_cache();
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
        let line = top_text_line(bounds, 11.0, TextLineInsets::horizontal(5.0), 3);
        assert_eq!(line.min, Point::new(15.0, 20.0));
        assert_eq!(line.max, Point::new(205.0, 31.0));
    }
}
