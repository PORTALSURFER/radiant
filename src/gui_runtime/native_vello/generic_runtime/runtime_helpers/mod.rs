//! Focused helper modules for the generic native runner.

mod gpu_surface_regions;
mod input;
mod profile;

pub(super) use gpu_surface_regions::{
    GpuSurfaceInteractionRegion, collect_gpu_surface_interaction_regions,
};
pub(super) use input::scroll_delta_to_logical;
pub(super) use profile::{maybe_log_route_profile, render_profile_enabled};
