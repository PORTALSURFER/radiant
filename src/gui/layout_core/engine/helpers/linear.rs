//! Shared linear sizing, overflow, and alignment helpers.

mod alignment;
mod overflow;
mod sizing;

pub(in crate::gui::layout_core::engine) use alignment::align_main_offsets;
pub(in crate::gui::layout_core::engine) use overflow::apply_linear_overflow_policy;
pub(in crate::gui::layout_core::engine) use sizing::{
    LinearLayoutState, allocate_fill_sizes, linear_sizing_summary, resolved_main_size,
    resolved_main_sizes_into, resolved_main_total,
};
