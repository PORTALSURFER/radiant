use super::{Point, Rect};

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

/// Return the suffix of `widths` that fits in `available_width`.
///
/// This preserves the rightmost items for compact action clusters and returns
/// widths in their original order.
pub fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    let mut visible = Vec::new();
    visible_suffix_widths_into(widths, available_width, gap, &mut visible);
    visible
}

/// Write the suffix of `widths` that fits in `available_width` into `visible`.
///
/// The output is cleared before use and retains its allocation for callers that
/// recalculate compact control clusters during repeated layout or paint passes.
pub fn visible_suffix_widths_into(
    widths: &[f32],
    available_width: f32,
    gap: f32,
    visible: &mut Vec<f32>,
) {
    visible.clear();
    if available_width <= 0.0 || widths.is_empty() {
        return;
    }
    let mut used = 0.0;
    let mut first_visible = widths.len();
    for (index, width) in widths.iter().rev().enumerate() {
        let candidate = used + width + if index > 0 { gap } else { 0.0 };
        if candidate >= available_width {
            break;
        }
        first_visible -= 1;
        used = candidate;
    }
    let suffix = &widths[first_visible..];
    if suffix.len() > visible.capacity() {
        visible.reserve(suffix.len());
    }
    visible.extend_from_slice(suffix);
}

/// Return the width of one fixed-width control group.
pub fn fixed_width_group_width(item_width: f32, item_count: usize, gap: f32) -> f32 {
    if item_width <= 0.0 || item_count == 0 {
        return 0.0;
    }
    (item_width * item_count as f32) + (gap.max(0.0) * item_count.saturating_sub(1) as f32)
}

/// Return the total width of fixed-width control groups separated by `group_gap`.
pub fn grouped_fixed_width_row_width(
    item_width: f32,
    group_counts: &[usize],
    gap: f32,
    group_gap: f32,
) -> f32 {
    if item_width <= 0.0 || group_counts.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    let mut visible_groups = 0usize;
    for count in group_counts.iter().copied().filter(|count| *count > 0) {
        total += fixed_width_group_width(item_width, count, gap);
        visible_groups += 1;
    }
    if visible_groups > 1 {
        total += group_gap.max(0.0) * visible_groups.saturating_sub(1) as f32;
    }
    total
}

/// Resolve a fixed item width that fits `item_count` items after reserved gaps.
pub fn fixed_width_item_extent_for_available_width(
    available_width: f32,
    item_count: usize,
    reserved_gap_width: f32,
    min_item_width: f32,
    max_item_width: f32,
) -> f32 {
    if available_width <= 0.0 || item_count == 0 {
        return 0.0;
    }
    let raw_width = (available_width - reserved_gap_width.max(0.0)) / item_count as f32;
    if raw_width <= 0.0 {
        0.0
    } else {
        raw_width
            .floor()
            .clamp(min_item_width.max(0.0), max_item_width.max(0.0))
    }
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

#[cfg(test)]
mod tests {
    use super::{
        fixed_width_group_width, fixed_width_item_extent_for_available_width,
        fixed_width_row_rects_end, fixed_width_row_rects_end_into, fixed_width_row_rects_start,
        fixed_width_row_rects_start_into, grouped_fixed_width_row_width, visible_suffix_widths,
        visible_suffix_widths_into,
    };
    use crate::gui::types::{Point, Rect};

    #[test]
    fn fixed_width_row_rects_start_places_items_from_left_edge() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
        let rects = fixed_width_row_rects_start(bounds, 4.0, &[20.0, 30.0], 1, 10);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min.x, 10.0);
        assert_eq!(rects[0].max.x, 30.0);
        assert_eq!(rects[1].min.x, 34.0);
        assert_eq!(rects[1].max.x, 64.0);
    }

    #[test]
    fn fixed_width_row_rects_end_places_items_against_right_edge() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
        let rects = fixed_width_row_rects_end(bounds, 4.0, &[20.0, 30.0], 1, 2, 10);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min.x, 56.0);
        assert_eq!(rects[0].max.x, 76.0);
        assert_eq!(rects[1].min.x, 80.0);
        assert_eq!(rects[1].max.x, 110.0);
    }

    #[test]
    fn fixed_width_row_rects_end_overflow_starts_at_left_edge() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(60.0, 40.0));
        let rects = fixed_width_row_rects_end(bounds, 4.0, &[40.0, 40.0], 1, 2, 10);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min.x, 10.0);
        assert_eq!(rects[0].max.x, 50.0);
        assert_eq!(rects[1].min.x, 54.0);
        assert_eq!(rects[1].max.x, 60.0);
    }

    #[test]
    fn visible_suffix_widths_preserves_rightmost_items_that_fit() {
        assert_eq!(
            visible_suffix_widths(&[20.0, 30.0, 40.0], 80.0, 4.0),
            [30.0, 40.0]
        );
        assert!(visible_suffix_widths(&[20.0], 20.0, 4.0).is_empty());
        assert_eq!(visible_suffix_widths(&[20.0], 20.1, 4.0), [20.0]);
    }

    #[test]
    fn fixed_width_row_rects_presizes_output() {
        let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 20.0));
        let rects = fixed_width_row_rects_start(bounds, 2.0, &[10.0, 20.0, 30.0], 1, 10);

        assert_eq!(rects.len(), 3);
        assert!(rects.capacity() >= 3);
    }

    #[test]
    fn fixed_width_row_rects_into_reuses_output_storage() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
        let mut rects = Vec::with_capacity(8);
        rects.push(Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
        ));
        let capacity = rects.capacity();

        fixed_width_row_rects_start_into(bounds, 4.0, &[20.0, 30.0], 1, 10, &mut rects);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects.capacity(), capacity);
        assert_eq!(rects[0].min.x, 10.0);
        assert_eq!(rects[1].max.x, 64.0);

        fixed_width_row_rects_end_into(bounds, 4.0, &[20.0, 30.0], 1, 2, 10, &mut rects);

        assert_eq!(rects.capacity(), capacity);
        assert_eq!(rects[0].min.x, 56.0);
        assert_eq!(rects[1].max.x, 110.0);
    }

    #[test]
    fn visible_suffix_widths_into_reuses_output_storage() {
        let mut visible = Vec::with_capacity(8);
        visible.push(99.0);
        let capacity = visible.capacity();

        visible_suffix_widths_into(&[20.0, 30.0, 40.0], 80.0, 4.0, &mut visible);

        assert_eq!(visible, [30.0, 40.0]);
        assert_eq!(visible.capacity(), capacity);

        visible_suffix_widths_into(&[20.0, 30.0, 40.0], 0.0, 4.0, &mut visible);

        assert!(visible.is_empty());
        assert_eq!(visible.capacity(), capacity);
    }

    #[test]
    fn grouped_fixed_width_row_width_counts_visible_groups_and_gaps() {
        assert_eq!(fixed_width_group_width(10.0, 3, 2.0), 34.0);
        assert_eq!(
            grouped_fixed_width_row_width(10.0, &[3, 0, 2], 2.0, 6.0),
            62.0
        );
        assert_eq!(grouped_fixed_width_row_width(0.0, &[3], 2.0, 6.0), 0.0);
        assert_eq!(grouped_fixed_width_row_width(10.0, &[], 2.0, 6.0), 0.0);
    }

    #[test]
    fn fixed_width_item_extent_for_available_width_fits_items_after_reserved_gaps() {
        assert_eq!(
            fixed_width_item_extent_for_available_width(100.0, 4, 12.0, 6.0, 20.0),
            20.0
        );
        assert_eq!(
            fixed_width_item_extent_for_available_width(40.0, 4, 12.0, 6.0, 20.0),
            7.0
        );
        assert_eq!(
            fixed_width_item_extent_for_available_width(20.0, 4, 12.0, 6.0, 20.0),
            6.0
        );
        assert_eq!(
            fixed_width_item_extent_for_available_width(10.0, 4, 12.0, 6.0, 20.0),
            0.0
        );
    }
}
