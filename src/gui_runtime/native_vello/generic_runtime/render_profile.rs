//! Native render profiling diagnostics for the generic Vello runtime.

use super::runner_state::NativeWindowAtlasResidencySnapshots;
use super::{
    GpuSurfaceAtlasResidencySnapshot, RetainedSurfaceEncodeStats,
    gpu_surface::GpuSurfaceRenderStats, render_profile_enabled,
};
use crate::gui_runtime::native_vello::TextLayoutProfileCounters;
use crate::runtime::NativeWindowDiagnosticIdentity;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const SLOW_RENDER_CPU_THRESHOLD: Duration = Duration::from_millis(12);
const SLOW_RENDER_PHASE_THRESHOLD: Duration = Duration::from_millis(6);
const SLOW_RENDER_CADENCE_THRESHOLD: Duration = Duration::from_millis(30);

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RenderFrameProfile {
    pub(super) record_timings: bool,
    pub(super) window_identity: Option<NativeWindowDiagnosticIdentity>,
    pub(super) frame_sequence: Option<u64>,
    pub(super) coalesced_wheel_route: Duration,
    pub(super) refresh_surface: Duration,
    pub(super) paint_plan: Duration,
    pub(super) deferred_scene_rebuild: Duration,
    pub(super) full_screen_blit: Duration,
    pub(super) composited_base_refresh: Duration,
    pub(super) composited_base_cache_hit: bool,
    pub(super) transient_overlay_paint: Duration,
    pub(super) transient_overlay_primitives: usize,
    pub(super) submit_present: Duration,
}

impl RenderFrameProfile {
    pub(super) fn recording(record_timings: bool) -> Self {
        Self {
            record_timings,
            ..Self::default()
        }
    }

    pub(super) fn measure<T>(&self, operation: impl FnOnce() -> T) -> (T, Duration) {
        if !self.record_timings {
            return (operation(), Duration::ZERO);
        }
        let started = Instant::now();
        let output = operation();
        (output, started.elapsed())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct NativeRenderProfileGpuSurface {
    pub(super) stats: GpuSurfaceRenderStats,
    pub(super) atlas_residency: NativeWindowAtlasResidencySnapshots,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeRenderProfileAtlasResidency {
    generation_known: Option<bool>,
    generation_serial: Option<u64>,
    resident_count: Option<usize>,
    logical_rgba_texel_bytes: Option<u64>,
}

fn project_atlas_residency(
    snapshot: Option<GpuSurfaceAtlasResidencySnapshot>,
) -> NativeRenderProfileAtlasResidency {
    NativeRenderProfileAtlasResidency {
        generation_known: snapshot.map(GpuSurfaceAtlasResidencySnapshot::generation_known),
        generation_serial: snapshot.and_then(GpuSurfaceAtlasResidencySnapshot::generation_serial),
        resident_count: snapshot.map(|snapshot| snapshot.resident_count),
        logical_rgba_texel_bytes: snapshot.and_then(|snapshot| snapshot.logical_rgba_texel_bytes),
    }
}

pub(super) fn maybe_log_render_profile(
    reason: &'static str,
    stats: RetainedSurfaceEncodeStats,
    text_stats: TextLayoutProfileCounters,
    render_to_texture_elapsed: Duration,
    frame: RenderFrameProfile,
    gpu_surface: NativeRenderProfileGpuSurface,
    since_last_present: Duration,
) {
    if !render_profile_enabled() {
        return;
    }
    let NativeRenderProfileGpuSurface {
        stats: gpu_surface_stats,
        atlas_residency,
    } = gpu_surface;
    let active_atlas = project_atlas_residency(atlas_residency.active);
    let quarantine_0_atlas = project_atlas_residency(atlas_residency.quarantine_0);
    let quarantine_1_atlas = project_atlas_residency(atlas_residency.quarantine_1);
    let cpu_envelope_total = tracked_cpu_envelope_total(frame, render_to_texture_elapsed);
    info!(
        reason,
        window_identity = frame
            .window_identity
            .map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
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
        gpu_surface_atlas_active_generation_known = active_atlas.generation_known,
        gpu_surface_atlas_active_generation_serial = active_atlas.generation_serial,
        gpu_surface_atlas_active_resident_count = active_atlas.resident_count,
        gpu_surface_atlas_active_logical_rgba_bytes = active_atlas.logical_rgba_texel_bytes,
        gpu_surface_atlas_q0_generation_known = quarantine_0_atlas.generation_known,
        gpu_surface_atlas_q0_generation_serial = quarantine_0_atlas.generation_serial,
        gpu_surface_atlas_q0_resident_count = quarantine_0_atlas.resident_count,
        gpu_surface_atlas_q0_logical_rgba_bytes = quarantine_0_atlas.logical_rgba_texel_bytes,
        gpu_surface_atlas_q1_generation_known = quarantine_1_atlas.generation_known,
        gpu_surface_atlas_q1_generation_serial = quarantine_1_atlas.generation_serial,
        gpu_surface_atlas_q1_resident_count = quarantine_1_atlas.resident_count,
        gpu_surface_atlas_q1_logical_rgba_bytes = quarantine_1_atlas.logical_rgba_texel_bytes,
        gpu_signal_summary_builds = gpu_surface_stats.signal.summary_builds,
        gpu_signal_summary_cache_hits = gpu_surface_stats.signal.summary_cache_hits,
        refresh_surface_us = frame.refresh_surface.as_micros(),
        paint_plan_us = frame.paint_plan.as_micros(),
        deferred_scene_rebuild_us = frame.deferred_scene_rebuild.as_micros(),
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
        gpu_surface_custom_shader_static_writes = gpu_surface_stats.custom_shader.static_writes,
        gpu_surface_custom_shader_static_write_bytes =
            gpu_surface_stats.custom_shader.static_write_bytes,
        gpu_surface_custom_shader_presentation_writes =
            gpu_surface_stats.custom_shader.presentation_writes,
        gpu_surface_custom_shader_presentation_write_bytes =
            gpu_surface_stats.custom_shader.presentation_write_bytes,
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

pub(super) fn maybe_log_slow_render_profile(
    reason: &'static str,
    stats: RetainedSurfaceEncodeStats,
    render_to_texture_elapsed: Duration,
    frame: RenderFrameProfile,
    gpu_surface_stats: GpuSurfaceRenderStats,
    since_last_present: Duration,
) {
    if !slow_render_profile_enabled() {
        return;
    }
    let cpu_envelope_total = tracked_cpu_envelope_total(frame, render_to_texture_elapsed);
    let slow_phase_total = [
        frame.coalesced_wheel_route,
        frame.refresh_surface,
        frame.paint_plan,
        frame.deferred_scene_rebuild,
        render_to_texture_elapsed,
        frame.full_screen_blit,
        frame.composited_base_refresh,
        frame.transient_overlay_paint,
        frame.submit_present,
    ]
    .into_iter()
    .max()
    .unwrap_or_default();
    if cpu_envelope_total < SLOW_RENDER_CPU_THRESHOLD
        && slow_phase_total < SLOW_RENDER_PHASE_THRESHOLD
        && since_last_present < SLOW_RENDER_CADENCE_THRESHOLD
    {
        return;
    }
    warn!(
        target: "radiant::debug::frame_profile",
        reason,
        window_identity = frame.window_identity.map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
        paint_plan_primitives = stats.paint_plan_primitives,
        scene_text_primitives = stats.text_primitive_count,
        scene_text_runs = stats.text_run_count,
        scene_images = stats.image_count,
        scene_gpu_surfaces = stats.gpu_surface_count,
        scene_custom_surfaces = stats.custom_surface_count,
        retained_bridge_calls = stats.bridge_calls,
        retained_cache_hits = stats.cache_hits,
        retained_surface_misses = stats.retained_surface_miss_count,
        gpu_surface_atlas_texture_uploads = gpu_surface_stats.atlas.texture_uploads,
        gpu_surface_atlas_texture_cache_hits = gpu_surface_stats.atlas.texture_cache_hits,
        gpu_signal_summary_builds = gpu_surface_stats.signal.summary_builds,
        gpu_signal_summary_cache_hits = gpu_surface_stats.signal.summary_cache_hits,
        gpu_signal_body_renders = gpu_surface_stats.signal.body_renders,
        gpu_signal_body_cache_hits = gpu_surface_stats.signal.body_cache_hits,
        gpu_signal_body_encode_us = gpu_surface_stats.signal.body_encode_elapsed.as_micros(),
        gpu_surface_composite_binding_rebuilds = gpu_surface_stats.composite.binding_rebuilds,
        gpu_surface_composite_binding_cache_hits = gpu_surface_stats.composite.binding_cache_hits,
        gpu_surface_composite_encode_us = gpu_surface_stats.composite.encode_elapsed.as_micros(),
        coalesced_wheel_route_us = frame.coalesced_wheel_route.as_micros(),
        refresh_surface_us = frame.refresh_surface.as_micros(),
        paint_plan_us = frame.paint_plan.as_micros(),
        deferred_scene_rebuild_us = frame.deferred_scene_rebuild.as_micros(),
        render_to_texture_us = render_to_texture_elapsed.as_micros(),
        full_screen_blit_encode_us = frame.full_screen_blit.as_micros(),
        composited_base_refresh_us = frame.composited_base_refresh.as_micros(),
        composited_base_cache_hit = frame.composited_base_cache_hit,
        transient_overlay_paint_us = frame.transient_overlay_paint.as_micros(),
        transient_overlay_primitives = frame.transient_overlay_primitives,
        submit_present_us = frame.submit_present.as_micros(),
        frame_cpu_envelope_total_us = cpu_envelope_total.as_micros(),
        slowest_tracked_phase_us = slow_phase_total.as_micros(),
        since_last_present_us = since_last_present.as_micros(),
        "radiant native slow frame profile"
    );
}

pub(super) const fn slow_render_profile_enabled() -> bool {
    cfg!(debug_assertions)
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

#[cfg(test)]
mod tests {
    use super::super::{
        GpuSurfaceAtlasResidencySnapshot, adapter::NativeAdapterGeneration,
        runner_state::NativeWindowAtlasResidencySnapshots,
    };
    use super::{NativeRenderProfileAtlasResidency, project_atlas_residency};

    #[test]
    fn atlas_profile_projection_preserves_generation_attribution_and_absent_slots() {
        let snapshots = NativeWindowAtlasResidencySnapshots {
            active: Some(GpuSurfaceAtlasResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(11),
                resident_count: 3,
                logical_rgba_texel_bytes: Some(12),
            }),
            quarantine_0: Some(GpuSurfaceAtlasResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(12),
                resident_count: 1,
                logical_rgba_texel_bytes: Some(4),
            }),
            quarantine_1: None,
        };

        assert_eq!(
            project_atlas_residency(snapshots.active),
            NativeRenderProfileAtlasResidency {
                generation_known: Some(true),
                generation_serial: Some(11),
                resident_count: Some(3),
                logical_rgba_texel_bytes: Some(12),
            }
        );
        assert_eq!(
            project_atlas_residency(snapshots.quarantine_0),
            NativeRenderProfileAtlasResidency {
                generation_known: Some(true),
                generation_serial: Some(12),
                resident_count: Some(1),
                logical_rgba_texel_bytes: Some(4),
            }
        );
        assert_eq!(
            project_atlas_residency(snapshots.quarantine_1),
            NativeRenderProfileAtlasResidency::default()
        );
    }

    #[test]
    fn atlas_profile_projection_keeps_unknown_and_exhausted_serials_absent() {
        let unknown = GpuSurfaceAtlasResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            resident_count: 1,
            logical_rgba_texel_bytes: Some(4),
        };
        let mut exhausted_generation = NativeAdapterGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted_generation.advance());
        let exhausted = GpuSurfaceAtlasResidencySnapshot {
            generation: exhausted_generation,
            resident_count: 2,
            logical_rgba_texel_bytes: Some(8),
        };

        for snapshot in [unknown, exhausted] {
            let projection = project_atlas_residency(Some(snapshot));
            assert_eq!(projection.generation_known, Some(false));
            assert_eq!(projection.generation_serial, None);
        }
    }
}
