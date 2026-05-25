use super::{
    StackedRowRectsParts, fixed_width_group_width, fixed_width_item_extent_for_available_width,
    fixed_width_row_rects_end, fixed_width_row_rects_end_into, fixed_width_row_rects_start,
    fixed_width_row_rects_start_into, grouped_fixed_width_row_width, stacked_row_rects,
    stacked_row_rects_from_parts, stacked_row_rects_into, stacked_row_rects_into_from_parts,
    visible_suffix_widths, visible_suffix_widths_into,
};
use crate::gui::types::{Point, Rect};

#[path = "tests/fitting.rs"]
mod fitting;
#[path = "tests/fixed_rects.rs"]
mod fixed_rects;
#[path = "tests/stacked_rows.rs"]
mod stacked_rows;
#[path = "tests/widths.rs"]
mod widths;
