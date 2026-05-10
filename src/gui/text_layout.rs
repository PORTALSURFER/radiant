//! Generic retained helpers for simple text-line placement.

mod cache;

use crate::gui::types::{Point, Rect};
use cache::TextLineKey;

pub use cache::TextLineLayoutCache;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TextLineMode {
    Center,
    Top,
}

/// Insets applied before resolving a single text-line placement rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLineInsets {
    /// Left inset applied before line placement.
    pub left: f32,
    /// Right inset applied before line placement.
    pub right: f32,
    /// Top inset applied before line placement.
    pub top: f32,
    /// Bottom inset applied before line placement.
    pub bottom: f32,
}

impl TextLineInsets {
    /// Build equal horizontal and vertical insets.
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    /// Build horizontal-only insets.
    pub fn horizontal(horizontal: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: 0.0,
            bottom: 0.0,
        }
    }
}

/// Resolve a vertically centered text-line rect.
pub fn centered_text_line(
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    family_id: u64,
) -> Rect {
    let _ = family_id;
    compute_text_line(
        bounds,
        font_size,
        insets,
        min_top_inset,
        TextLineMode::Center,
    )
}

/// Resolve a vertically centered text-line rect through an owned micro-layout cache.
pub fn centered_text_line_with_cache(
    cache: &mut TextLineLayoutCache,
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    family_id: u64,
) -> Rect {
    text_line(
        Some(cache),
        bounds,
        font_size,
        insets,
        min_top_inset,
        family_id,
        TextLineMode::Center,
    )
}

/// Resolve a top-aligned text-line rect.
pub fn top_text_line(bounds: Rect, font_size: f32, insets: TextLineInsets, family_id: u64) -> Rect {
    let _ = family_id;
    compute_text_line(bounds, font_size, insets, 0.0, TextLineMode::Top)
}

/// Resolve a top-aligned text-line rect through an owned micro-layout cache.
pub fn top_text_line_with_cache(
    cache: &mut TextLineLayoutCache,
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    family_id: u64,
) -> Rect {
    text_line(
        Some(cache),
        bounds,
        font_size,
        insets,
        0.0,
        family_id,
        TextLineMode::Top,
    )
}

/// Snap a text-line rectangle so its bottom edge lands on a full pixel.
///
/// This keeps repeated text rows from alternating between adjacent raster rows
/// when compact density tokens produce fractional line positions.
pub fn snap_text_baseline_to_pixel(line: Rect) -> Rect {
    let height = line.height().max(0.0);
    if height <= 0.0 {
        return line;
    }
    let baseline = (line.min.y + height).round();
    let min_y = baseline - height;
    Rect::from_min_max(
        Point::new(line.min.x, min_y),
        Point::new(line.max.x, baseline),
    )
}

fn text_line(
    cache: Option<&mut TextLineLayoutCache>,
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    family_id: u64,
    mode: TextLineMode,
) -> Rect {
    let empty = bounds.empty_at_min();
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let compute = || compute_text_line(bounds, font_size, insets, min_top_inset, mode);
    let Some(cache) = cache else {
        return compute();
    };
    let key = TextLineKey::new(mode, family_id, bounds, insets, font_size, min_top_inset);
    cache.cached_text_line(key, compute)
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
        return bounds.empty_at_min();
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
    line.clamp_to(inner)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_line_reuses_cached_geometry_for_identical_inputs() {
        let mut cache = TextLineLayoutCache::new();
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
        let first = centered_text_line_with_cache(
            &mut cache,
            bounds,
            12.0,
            TextLineInsets::symmetric(8.0, 3.0),
            0.0,
            1,
        );
        let second = centered_text_line_with_cache(
            &mut cache,
            bounds,
            12.0,
            TextLineInsets::symmetric(8.0, 3.0),
            0.0,
            1,
        );
        assert_eq!(first, second);
        assert_eq!(cache.misses_for_test(), 1);
    }

    #[test]
    fn centered_line_invalidates_when_font_bounds_or_insets_change() {
        let mut cache = TextLineLayoutCache::new();
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
        let base = centered_text_line_with_cache(
            &mut cache,
            bounds,
            12.0,
            TextLineInsets::symmetric(8.0, 3.0),
            0.0,
            2,
        );
        let taller = centered_text_line_with_cache(
            &mut cache,
            bounds,
            14.0,
            TextLineInsets::symmetric(8.0, 3.0),
            0.0,
            2,
        );
        let wider = centered_text_line_with_cache(
            &mut cache,
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(230.0, 60.0)),
            12.0,
            TextLineInsets::symmetric(8.0, 3.0),
            0.0,
            2,
        );
        let inset = centered_text_line_with_cache(
            &mut cache,
            bounds,
            12.0,
            TextLineInsets::symmetric(10.0, 3.0),
            0.0,
            2,
        );
        assert_ne!(base, taller);
        assert_ne!(base, wider);
        assert_ne!(base, inset);
        assert_eq!(cache.misses_for_test(), 4);
    }

    #[test]
    fn top_line_uses_top_edge_after_horizontal_inset() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
        let line = top_text_line(bounds, 11.0, TextLineInsets::horizontal(5.0), 3);
        assert_eq!(line.min, Point::new(15.0, 20.0));
        assert_eq!(line.max, Point::new(205.0, 31.0));
    }

    #[test]
    fn explicit_cache_eviction_keeps_capacity_bounded() {
        let mut cache = TextLineLayoutCache::with_capacity(2);
        let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 40.0));

        for family_id in 1..=3 {
            let _ = top_text_line_with_cache(
                &mut cache,
                bounds,
                12.0,
                TextLineInsets::horizontal(4.0),
                family_id,
            );
        }

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.misses_for_test(), 3);
    }

    #[test]
    fn snap_text_baseline_to_pixel_keeps_height_and_rounds_bottom_edge() {
        let line = Rect::from_min_max(Point::new(10.0, 20.25), Point::new(110.0, 34.75));

        assert_eq!(
            snap_text_baseline_to_pixel(line),
            Rect::from_min_max(Point::new(10.0, 20.5), Point::new(110.0, 35.0))
        );
    }
}
