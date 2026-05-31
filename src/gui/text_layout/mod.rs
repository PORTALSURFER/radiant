//! Generic retained helpers for simple text-line placement.

mod cache;
mod insets;
mod placement;
mod width;

#[cfg(test)]
use crate::gui::types::Point;
use crate::gui::types::Rect;
use cache::TextLineKey;
use placement::compute_text_line;

pub use cache::TextLineLayoutCache;
pub use insets::TextLineInsets;
pub use placement::snap_text_baseline_to_pixel;
pub use width::{
    TextWidthEstimate, estimated_text_width, estimated_text_width_for_char_count,
    estimated_text_width_for_char_count_in_range, estimated_text_width_in_range,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TextLineMode {
    Center,
    Top,
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

#[cfg(test)]
mod tests;
