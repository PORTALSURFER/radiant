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
) -> Rect {
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
pub fn top_text_line(bounds: Rect, font_size: f32, insets: TextLineInsets) -> Rect {
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
mod tests;
