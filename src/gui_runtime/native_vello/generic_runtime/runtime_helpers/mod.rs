//! Focused helper modules for the generic native runner.

mod gpu_surface_regions;
mod input;
mod profile;
mod rect_occlusion;

pub(in crate::gui_runtime::native_vello) use gpu_surface_regions::{
    GpuSurfaceInteractionRegion, GpuSurfaceInteractionScratch,
    collect_gpu_surface_interaction_regions_with_scratch,
};
pub(super) use input::scroll_delta_to_logical;
pub(super) use profile::{maybe_log_route_profile, render_profile_enabled};
pub(super) use rect_occlusion::{
    intersect_rect, visible_rects_after_occlusion, visible_rects_after_occlusion_into,
};
