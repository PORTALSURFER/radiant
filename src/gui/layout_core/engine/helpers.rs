//! Shared layout-engine helpers.

mod axis;
mod boxes;
mod geometry;
mod linear;

pub(super) use axis::LayoutAxis;
pub(super) use boxes::{fit_aspect_box, select_switch_child};
pub(super) use geometry::{content_rect, place_child_rect};
pub(super) use linear::{
    LinearLayoutState, align_main_offsets, allocate_fill_sizes, apply_linear_overflow_policy,
    linear_sizing_summary, resolved_main_sizes_into,
};
