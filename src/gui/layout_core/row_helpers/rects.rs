use crate::gui::types::{Point, Rect};

/// Compute fixed-width row item rects aligned to the start edge of `bounds`.
///
/// This helper is intended for compact toolbars and control strips that need
/// deterministic slot geometry without owning a host-specific layout adapter.
/// The ID parameters are retained for API consistency with declarative layout
/// callers; this helper returns geometry only.
pub fn fixed_width_row_rects_start(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    _row_id: u64,
    _first_item_id: u64,
) -> Vec<Rect> {
    let mut rects = Vec::new();
    fixed_width_row_rects_into(bounds, gap, widths, false, &mut rects);
    rects
}

/// Compute start-aligned fixed-width row item rects into caller-owned storage.
///
/// This is the allocation-reusing counterpart to [`fixed_width_row_rects_start`]
/// for renderers, toolbars, and dense control strips that rebuild fixed row
/// geometry repeatedly.
pub fn fixed_width_row_rects_start_into(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    _row_id: u64,
    _first_item_id: u64,
    rects: &mut Vec<Rect>,
) {
    fixed_width_row_rects_into(bounds, gap, widths, false, rects);
}

/// Compute fixed-width row item rects aligned to the end edge of `bounds`.
///
/// The ID parameters are retained for API consistency with declarative layout
/// callers; this helper returns geometry only.
pub fn fixed_width_row_rects_end(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    _row_id: u64,
    _spacer_id: u64,
    _first_item_id: u64,
) -> Vec<Rect> {
    let mut rects = Vec::new();
    fixed_width_row_rects_into(bounds, gap, widths, true, &mut rects);
    rects
}

/// Compute end-aligned fixed-width row item rects into caller-owned storage.
///
/// This preserves the public geometry contract of [`fixed_width_row_rects_end`]
/// while allowing hot paths to reuse an existing output buffer.
pub fn fixed_width_row_rects_end_into(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    _row_id: u64,
    _spacer_id: u64,
    _first_item_id: u64,
    rects: &mut Vec<Rect>,
) {
    fixed_width_row_rects_into(bounds, gap, widths, true, rects);
}

fn fixed_width_row_rects_into(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    align_end: bool,
    rects: &mut Vec<Rect>,
) {
    rects.clear();
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return;
    }

    let gap = gap.max(0.0);
    let total_width = widths
        .iter()
        .copied()
        .map(|width| width.max(0.0))
        .sum::<f32>()
        + gap * widths.len().saturating_sub(1) as f32;
    let mut x = if align_end && total_width < bounds.width() {
        bounds.max.x - total_width
    } else {
        bounds.min.x
    };

    if widths.len() > rects.capacity() {
        rects.reserve(widths.len());
    }
    for (index, width) in widths.iter().copied().enumerate() {
        if index > 0 {
            x += gap;
        }
        let width = width.max(0.0);
        let rect = Rect {
            min: Point::new(x, bounds.min.y),
            max: Point::new(x + width, bounds.max.y),
        };
        x += width;
        rects.push(rect.clamp_to(bounds));
    }
}
