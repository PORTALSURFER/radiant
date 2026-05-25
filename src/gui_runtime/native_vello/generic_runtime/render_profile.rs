//! Native render profiling diagnostics for the generic Vello runtime.

use super::{
    RetainedSurfaceEncodeStats, gpu_surface::GpuSurfaceRenderStats, render_profile_enabled,
};
use crate::gui_runtime::native_vello::TextLayoutProfileCounters;
use std::time::Duration;
use tracing::info;

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
    text_stats: TextLayoutProfileCounters,
    render_to_texture_elapsed: Duration,
    frame: RenderFrameProfile,
    gpu_surface_stats: GpuSurfaceRenderStats,
    since_last_present: Duration,
) {
    if !render_profile_enabled() {
        return;
    }
    let cpu_envelope_total = tracked_cpu_envelope_total(frame, render_to_texture_elapsed);
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
        scene_custom_surface_fallbacks = stats.custom_surface_fallback_count,
        text_layout_cache_hits = text_stats.layout.hits,
        text_layout_cache_misses = text_stats.layout.misses,
        text_layout_cache_evictions = text_stats.layout.evictions,
        text_atom_cache_hits = text_stats.atom.hits,
        text_atom_cache_misses = text_stats.atom.misses,
        text_atom_cache_evictions = text_stats.atom.evictions,
        text_unsupported_shaping_runs = text_stats.quality.unsupported_shaping_runs,
        text_unsupported_shaping_scalars = text_stats.quality.unsupported_shaping_scalars,
        text_fallback_glyphs = text_stats.quality.fallback_glyphs,
        text_missing_glyphs = text_stats.quality.missing_glyphs,
        text_quality_status = text_quality_status(text_stats),
        retained_bridge_calls = stats.bridge_calls,
        retained_cache_hits = stats.cache_hits,
        retained_surface_misses = stats.retained_surface_miss_count,
        retained_frame_primitives = stats.retained_frame_primitive_count,
        retained_frame_text_runs = stats.retained_frame_text_run_count,
        gpu_surface_atlas_texture_uploads = gpu_surface_stats.atlas.texture_uploads,
        gpu_surface_atlas_texture_cache_hits = gpu_surface_stats.atlas.texture_cache_hits,
        gpu_signal_summary_builds = gpu_surface_stats.signal.summary_builds,
        gpu_signal_summary_cache_hits = gpu_surface_stats.signal.summary_cache_hits,
        refresh_surface_us = frame.refresh_surface.as_micros(),
        paint_plan_us = frame.paint_plan.as_micros(),
        render_to_texture_us = render_to_texture_elapsed.as_micros(),
        full_screen_blit_encode_us = frame.full_screen_blit.as_micros(),
        coalesced_wheel_route_us = frame.coalesced_wheel_route.as_micros(),
        gpu_signal_body_renders = gpu_surface_stats.signal.body_renders,
        gpu_signal_body_cache_hits = gpu_surface_stats.signal.body_cache_hits,
        gpu_signal_body_encode_us = gpu_surface_stats.signal.body_encode_elapsed.as_micros(),
        gpu_surface_composite_binding_rebuilds = gpu_surface_stats.composite.binding_rebuilds,
        gpu_surface_composite_binding_cache_hits = gpu_surface_stats.composite.binding_cache_hits,
        gpu_surface_custom_shader_surfaces_rendered =
            gpu_surface_stats.custom_shader.surfaces_rendered,
        gpu_surface_custom_shader_pipeline_rebuilds =
            gpu_surface_stats.custom_shader.pipeline_rebuilds,
        gpu_surface_custom_shader_binding_rebuilds =
            gpu_surface_stats.custom_shader.binding_rebuilds,
        gpu_surface_custom_shader_binding_cache_hits =
            gpu_surface_stats.custom_shader.binding_cache_hits,
        gpu_surface_custom_shader_surfaces_failed =
            gpu_surface_stats.custom_shader.failures.surfaces_failed,
        gpu_surface_custom_shader_shader_module_failures = gpu_surface_stats
            .custom_shader
            .failures
            .shader_module_failures,
        gpu_surface_custom_shader_pipeline_failures =
            gpu_surface_stats.custom_shader.failures.pipeline_failures,
        gpu_surface_custom_shader_binding_failures =
            gpu_surface_stats.custom_shader.failures.binding_failures,
        gpu_surface_unsupported_custom_shader_surfaces =
            gpu_surface_stats.custom_shader.unsupported.surfaces,
        gpu_surface_unsupported_custom_shader_vertices =
            gpu_surface_stats.custom_shader.unsupported.vertices,
        gpu_surface_unsupported_custom_shader_source_bytes =
            gpu_surface_stats.custom_shader.unsupported.source_bytes,
        gpu_surface_unsupported_custom_shader_uniform_bytes =
            gpu_surface_stats.custom_shader.unsupported.uniform_bytes,
        gpu_surface_unsupported_custom_shader_storage_bytes =
            gpu_surface_stats.custom_shader.unsupported.storage_bytes,
        gpu_surface_composite_encode_us = gpu_surface_stats.composite.encode_elapsed.as_micros(),
        frame_cpu_envelope_total_us = cpu_envelope_total.as_micros(),
        gpu_timing_status = "cpu_envelope_only",
        composited_base_refresh_us = frame.composited_base_refresh.as_micros(),
        composited_base_cache_hit = frame.composited_base_cache_hit,
        transient_overlay_paint_us = frame.transient_overlay_paint.as_micros(),
        transient_overlay_primitives = frame.transient_overlay_primitives,
        submit_present_us = frame.submit_present.as_micros(),
        since_last_present_us = since_last_present.as_micros(),
        "radiant native render profile"
    );
}

fn tracked_cpu_envelope_total(
    frame: RenderFrameProfile,
    render_to_texture_elapsed: Duration,
) -> Duration {
    frame.coalesced_wheel_route
        + frame.refresh_surface
        + frame.paint_plan
        + render_to_texture_elapsed
        + frame.full_screen_blit
        + frame.composited_base_refresh
        + frame.transient_overlay_paint
        + frame.submit_present
}

fn text_quality_status(text_stats: TextLayoutProfileCounters) -> &'static str {
    match (
        text_stats.quality.unsupported_shaping_runs > 0
            || text_stats.quality.unsupported_shaping_scalars > 0,
        text_stats.quality.fallback_glyphs > 0 || text_stats.quality.missing_glyphs > 0,
    ) {
        (false, false) => "clean",
        (true, false) => "shaping_limited",
        (false, true) => "font_coverage_limited",
        (true, true) => "shaping_and_font_coverage_limited",
    }
}
