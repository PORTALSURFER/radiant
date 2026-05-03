use crate::gui::layout_core::{
    fixed_width_row_rects_end, fixed_width_row_rects_start,
    visible_suffix_widths as generic_visible_suffix_widths,
};
use crate::gui::types::Rect;

pub(super) fn center_square_rect(rect: Rect, side: f32) -> Rect {
    rect.centered_square(side)
}

#[cfg(test)]
pub(super) fn clamp_rect_right_edge(rect: Rect, bounds: Rect, right_edge: f32) -> Rect {
    let clamped = clamp_rect_to_bounds(rect, bounds);
    let max_x = clamped.max.x.min(right_edge.max(bounds.min.x));
    if max_x < clamped.min.x {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(
        clamped.min,
        crate::gui::types::Point::new(max_x, clamped.max.y),
    )
}

pub(super) fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    rect.clamp_to(bounds)
}

pub(super) fn empty_rect(bounds: Rect) -> Rect {
    bounds.empty_at_min()
}

pub(super) fn layout_left_aligned_fixed_widths(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    first_button_id: u64,
) -> Vec<Rect> {
    fixed_width_row_rects_start(bounds, gap, widths, row_id, first_button_id)
}

pub(super) fn layout_right_aligned_fixed_widths(
    bounds: Rect,
    gap: f32,
    widths: &[f32],
    row_id: u64,
    spacer_id: u64,
    first_button_id: u64,
) -> Vec<Rect> {
    fixed_width_row_rects_end(bounds, gap, widths, row_id, spacer_id, first_button_id)
}

pub(super) fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    generic_visible_suffix_widths(widths, available_width, gap)
}
