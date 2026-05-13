//! Native render profiling diagnostics for the generic Vello runtime.

use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RenderFrameProfile {
    pub(super) coalesced_wheel_route: Duration,
    pub(super) refresh_surface: Duration,
    pub(super) paint_plan: Duration,
    pub(super) full_screen_blit: Duration,
    pub(super) composited_base_refresh: Duration,
    pub(super) composited_base_cache_hit: bool,
    pub(super) transient_overlay_paint: Duration,
    pub(super) transient_overlay_primitives: usize,
    pub(super) submit_present: Duration,
}

pub(super) fn maybe_log_render_profile(
    reason: &'static str,
    stats: RetainedSurfaceEncodeStats,
    render_to_texture_elapsed: Duration,
    frame: RenderFrameProfile,
    gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    since_last_present: Duration,
) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        paint_plan_primitives = stats.paint_plan_primitives,
        scene_clip_layers = stats.clip_layer_count,
        scene_text_primitives = stats.text_primitive_count,
        scene_text_inputs = stats.text_input_count,
        scene_text_runs = stats.text_run_count,
        scene_images = stats.image_count,
        scene_gpu_surfaces = stats.gpu_surface_count,
        scene_custom_surfaces = stats.custom_surface_count,
        retained_bridge_calls = stats.bridge_calls,
        retained_cache_hits = stats.cache_hits,
        retained_frame_primitives = stats.retained_frame_primitive_count,
        retained_frame_text_runs = stats.retained_frame_text_run_count,
        gpu_surface_atlas_texture_uploads = gpu_surface_stats.atlas_texture_uploads,
        gpu_signal_summary_builds = gpu_surface_stats.signal_summary_builds,
        gpu_signal_summary_cache_hits = gpu_surface_stats.signal_summary_cache_hits,
        refresh_surface_us = frame.refresh_surface.as_micros(),
        paint_plan_us = frame.paint_plan.as_micros(),
        render_to_texture_us = render_to_texture_elapsed.as_micros(),
        full_screen_blit_encode_us = frame.full_screen_blit.as_micros(),
        coalesced_wheel_route_us = frame.coalesced_wheel_route.as_micros(),
        gpu_signal_body_renders = gpu_surface_stats.signal_body_renders,
        gpu_signal_body_cache_hits = gpu_surface_stats.signal_body_cache_hits,
        gpu_signal_body_encode_us = gpu_surface_stats.signal_body_encode_elapsed.as_micros(),
        gpu_surface_composite_binding_rebuilds = gpu_surface_stats.composite_binding_rebuilds,
        gpu_surface_composite_binding_cache_hits = gpu_surface_stats.composite_binding_cache_hits,
        gpu_surface_composite_encode_us = gpu_surface_stats.composite_encode_elapsed.as_micros(),
        composited_base_refresh_us = frame.composited_base_refresh.as_micros(),
        composited_base_cache_hit = frame.composited_base_cache_hit,
        transient_overlay_paint_us = frame.transient_overlay_paint.as_micros(),
        transient_overlay_primitives = frame.transient_overlay_primitives,
        submit_present_us = frame.submit_present.as_micros(),
        since_last_present_us = since_last_present.as_micros(),
        "radiant native render profile"
    );
}
