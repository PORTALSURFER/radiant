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
    fixed_width_row_rects(bounds, gap, widths, false)
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
    fixed_width_row_rects(bounds, gap, widths, true)
}

/// Return the suffix of `widths` that fits in `available_width`.
///
/// This preserves the rightmost items for compact action clusters and returns
/// widths in their original order.
pub fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    if available_width <= 0.0 || widths.is_empty() {
        return Vec::new();
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
    widths[first_visible..].to_vec()
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

fn fixed_width_row_rects(bounds: Rect, gap: f32, widths: &[f32], align_end: bool) -> Vec<Rect> {
    if widths.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Vec::new();
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

    let mut rects = Vec::with_capacity(widths.len());
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
    rects
}

#[cfg(test)]
mod tests {
    use super::{
        fixed_width_group_width, fixed_width_item_extent_for_available_width,
        fixed_width_row_rects_end, fixed_width_row_rects_start, grouped_fixed_width_row_width,
        visible_suffix_widths,
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
