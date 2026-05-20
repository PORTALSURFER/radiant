//! Compact fixed-row geometry helpers.

mod fitting;
mod rects;
mod widths;

pub use fitting::{
    fixed_width_item_extent_for_available_width, visible_suffix_widths, visible_suffix_widths_into,
};
pub use rects::{
    fixed_width_row_rects_end, fixed_width_row_rects_end_into, fixed_width_row_rects_start,
    fixed_width_row_rects_start_into, stacked_row_rects, stacked_row_rects_into,
};
pub use widths::{fixed_width_group_width, grouped_fixed_width_row_width};

#[cfg(test)]
#[path = "row_helpers/tests.rs"]
mod tests;
