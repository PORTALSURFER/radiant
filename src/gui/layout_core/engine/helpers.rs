//! Shared layout-engine helpers.

mod boxes;
mod geometry;
mod linear;

pub(super) use boxes::{fit_aspect_box, select_switch_child};
pub(super) use geometry::{content_rect, place_child_rect};
pub(super) use linear::{
    LinearLayoutState, align_main_offsets, allocate_fill_sizes, compress_if_needed,
    main_margin_total, scale_sizes_to_fit,
};
