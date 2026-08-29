//! Native render profiling diagnostics for the generic Vello runtime.

use super::runner_state::{
    NativeWindowAtlasResidencySnapshots, NativeWindowCompositedBaseResidencySnapshots,
    NativeWindowCustomShaderResidencySnapshots, NativeWindowSignalResidencySnapshots,
    NativeWindowTargetResidencySnapshots,
};
use super::{
    GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCompositedBaseResidencySnapshot,
    GpuSurfaceCustomShaderResidencySnapshot, GpuSurfaceSignalResidencySnapshot,
    GpuSurfaceTargetResidencySnapshot, NativeAdapterAtlasResidencyProfile,
    NativeAdapterCustomShaderResidencyProfile, NativeAdapterRenderCanvasUploadProfile,
    NativeAdapterSignalResidencyProfile, RetainedSurfaceEncodeStats,
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
    pub(super) signal_residency: NativeWindowSignalResidencySnapshots,
    pub(super) custom_shader_residency: NativeWindowCustomShaderResidencySnapshots,
    pub(super) composited_base_residency: NativeWindowCompositedBaseResidencySnapshots,
    pub(super) target_residency: NativeWindowTargetResidencySnapshots,
    pub(super) application_atlas_residency: NativeAdapterAtlasResidencyProfile,
    pub(super) application_signal_residency: NativeAdapterSignalResidencyProfile,
    pub(super) application_custom_shader_residency: NativeAdapterCustomShaderResidencyProfile,
    pub(super) application_render_canvas_uploads: NativeAdapterRenderCanvasUploadProfile,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeRenderProfileSignalResidency {
    generation_known: Option<bool>,
    generation_serial: Option<u64>,
    signal_buffer_resident_count: Option<usize>,
    signal_buffer_logical_bytes: Option<u64>,
    signal_body_texture_resident_count: Option<usize>,
    signal_body_texture_logical_rgba_bytes: Option<u64>,
}

fn project_signal_residency(
    snapshot: Option<GpuSurfaceSignalResidencySnapshot>,
) -> NativeRenderProfileSignalResidency {
    NativeRenderProfileSignalResidency {
        generation_known: snapshot.map(GpuSurfaceSignalResidencySnapshot::generation_known),
        generation_serial: snapshot.and_then(GpuSurfaceSignalResidencySnapshot::generation_serial),
        signal_buffer_resident_count: snapshot
            .map(|snapshot| snapshot.signal_buffer_resident_count),
        signal_buffer_logical_bytes: snapshot
            .and_then(|snapshot| snapshot.signal_buffer_logical_bytes),
        signal_body_texture_resident_count: snapshot
            .map(|snapshot| snapshot.signal_body_texture_resident_count),
        signal_body_texture_logical_rgba_bytes: snapshot
            .and_then(|snapshot| snapshot.signal_body_texture_logical_rgba_bytes),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeRenderProfileCustomShaderResidency {
    generation_known: Option<bool>,
    generation_serial: Option<u64>,
    pipeline_resident_count: Option<usize>,
    binding_resident_count: Option<usize>,
    surface_uniform_logical_bytes: Option<u64>,
    app_uniform_logical_bytes: Option<u64>,
    storage_logical_bytes: Option<u64>,
    presentation_uniform_logical_bytes: Option<u64>,
}

fn project_custom_shader_residency(
    snapshot: Option<GpuSurfaceCustomShaderResidencySnapshot>,
) -> NativeRenderProfileCustomShaderResidency {
    NativeRenderProfileCustomShaderResidency {
        generation_known: snapshot.map(GpuSurfaceCustomShaderResidencySnapshot::generation_known),
        generation_serial: snapshot
            .and_then(GpuSurfaceCustomShaderResidencySnapshot::generation_serial),
        pipeline_resident_count: snapshot.map(|snapshot| snapshot.pipeline_resident_count),
        binding_resident_count: snapshot.map(|snapshot| snapshot.binding_resident_count),
        surface_uniform_logical_bytes: snapshot
            .and_then(|snapshot| snapshot.surface_uniform_logical_bytes),
        app_uniform_logical_bytes: snapshot.and_then(|snapshot| snapshot.app_uniform_logical_bytes),
        storage_logical_bytes: snapshot.and_then(|snapshot| snapshot.storage_logical_bytes),
        presentation_uniform_logical_bytes: snapshot
            .and_then(|snapshot| snapshot.presentation_uniform_logical_bytes),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeRenderProfileCompositedBaseResidency {
    generation_known: Option<bool>,
    generation_serial: Option<u64>,
    active_object_count: Option<usize>,
    retired_object_count: Option<usize>,
    active_requested_backing_bytes: Option<u64>,
    retired_requested_backing_bytes: Option<u64>,
}

fn project_composited_base_residency(
    snapshot: Option<GpuSurfaceCompositedBaseResidencySnapshot>,
) -> NativeRenderProfileCompositedBaseResidency {
    NativeRenderProfileCompositedBaseResidency {
        generation_known: snapshot.map(GpuSurfaceCompositedBaseResidencySnapshot::generation_known),
        generation_serial: snapshot
            .and_then(GpuSurfaceCompositedBaseResidencySnapshot::generation_serial),
        active_object_count: snapshot.map(|snapshot| snapshot.active_object_count),
        retired_object_count: snapshot.map(|snapshot| snapshot.retired_object_count),
        active_requested_backing_bytes: snapshot
            .and_then(|snapshot| snapshot.active_requested_backing_bytes),
        retired_requested_backing_bytes: snapshot
            .and_then(|snapshot| snapshot.retired_requested_backing_bytes),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NativeRenderProfileTargetResidency {
    generation_known: Option<bool>,
    generation_serial: Option<u64>,
    // Keep the existing profile field names as the active-target aliases.
    resident_count: Option<usize>,
    requested_rgba8_bytes: Option<u64>,
    predecessor_object_count: Option<usize>,
    predecessor_requested_rgba8_bytes: Option<u64>,
}

fn project_target_residency(
    snapshot: Option<GpuSurfaceTargetResidencySnapshot>,
) -> NativeRenderProfileTargetResidency {
    NativeRenderProfileTargetResidency {
        generation_known: snapshot.map(GpuSurfaceTargetResidencySnapshot::generation_known),
        generation_serial: snapshot.and_then(GpuSurfaceTargetResidencySnapshot::generation_serial),
        resident_count: snapshot.map(|snapshot| snapshot.active_object_count),
        requested_rgba8_bytes: snapshot.and_then(|snapshot| snapshot.active_requested_rgba8_bytes),
        predecessor_object_count: snapshot.map(|snapshot| snapshot.predecessor_object_count),
        predecessor_requested_rgba8_bytes: snapshot
            .and_then(|snapshot| snapshot.predecessor_requested_rgba8_bytes),
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
        signal_residency,
        custom_shader_residency,
        composited_base_residency,
        target_residency,
        application_atlas_residency,
        application_signal_residency,
        application_custom_shader_residency,
        application_render_canvas_uploads,
    } = gpu_surface;
    let active_atlas = project_atlas_residency(atlas_residency.active);
    let quarantine_0_atlas = project_atlas_residency(atlas_residency.quarantine_0);
    let quarantine_1_atlas = project_atlas_residency(atlas_residency.quarantine_1);
    let active_signal = project_signal_residency(signal_residency.active);
    let quarantine_0_signal = project_signal_residency(signal_residency.quarantine_0);
    let quarantine_1_signal = project_signal_residency(signal_residency.quarantine_1);
    let active_custom_shader = project_custom_shader_residency(custom_shader_residency.active);
    let quarantine_0_custom_shader =
        project_custom_shader_residency(custom_shader_residency.quarantine_0);
    let quarantine_1_custom_shader =
        project_custom_shader_residency(custom_shader_residency.quarantine_1);
    let active_composited_base =
        project_composited_base_residency(composited_base_residency.active);
    let quarantine_0_composited_base =
        project_composited_base_residency(composited_base_residency.quarantine_0);
    let quarantine_1_composited_base =
        project_composited_base_residency(composited_base_residency.quarantine_1);
    let active_target = project_target_residency(target_residency.active);
    let quarantine_0_target = project_target_residency(target_residency.quarantine_0);
    let quarantine_1_target = project_target_residency(target_residency.quarantine_1);
    let render_canvas_uploads = gpu_surface_stats.render_canvas_uploads;
    let render_canvas_upload_plan = gpu_surface_stats.render_canvas_upload_plan;
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
        gpu_surface_atlas_application_adapter_generation_known = application_atlas_residency
            .adapter_generation
            .map(|generation| generation.is_known()),
        gpu_surface_atlas_application_adapter_generation_serial = application_atlas_residency
            .adapter_generation
            .and_then(|generation| generation.known_serial()),
        gpu_surface_atlas_application_active_resident_count =
            application_atlas_residency.active_resident_count,
        gpu_surface_atlas_application_active_logical_rgba_bytes =
            application_atlas_residency.active_logical_rgba_texel_bytes,
        gpu_surface_atlas_application_quarantined_resident_count =
            application_atlas_residency.quarantined_resident_count,
        gpu_surface_atlas_application_quarantined_logical_rgba_bytes =
            application_atlas_residency.quarantined_logical_rgba_texel_bytes,
        gpu_surface_render_canvas_upload_application_adapter_generation_known =
            application_render_canvas_uploads
                .adapter_generation
                .map(|generation| generation.is_known()),
        gpu_surface_render_canvas_upload_application_adapter_generation_serial =
            application_render_canvas_uploads
                .adapter_generation
                .and_then(|generation| generation.known_serial()),
        gpu_surface_render_canvas_upload_observed_candidate_plan_count =
            application_render_canvas_uploads.observed_candidate_plan_count,
        gpu_surface_render_canvas_upload_observed_candidate_plan_window_count =
            application_render_canvas_uploads.observed_candidate_plan_window_count,
        gpu_surface_render_canvas_upload_observed_candidate_no_work_count =
            application_render_canvas_uploads.observed_candidate_no_work_count,
        gpu_surface_render_canvas_upload_observed_candidate_exact_count =
            application_render_canvas_uploads.observed_candidate_exact_count,
        gpu_surface_render_canvas_upload_observed_candidate_invalid_count =
            application_render_canvas_uploads.observed_candidate_invalid_count,
        gpu_surface_render_canvas_upload_observed_candidate_unsupported_count =
            application_render_canvas_uploads.observed_candidate_unsupported_count,
        gpu_surface_render_canvas_upload_observed_candidate_incomplete_count =
            application_render_canvas_uploads.observed_candidate_incomplete_count,
        gpu_surface_render_canvas_upload_observed_candidate_overflow_count =
            application_render_canvas_uploads.observed_candidate_overflow_count,
        gpu_surface_render_canvas_upload_observed_candidate_exact_immutable_payload_operations =
            application_render_canvas_uploads
                .observed_candidate_exact_immutable_payload_operations,
        gpu_surface_render_canvas_upload_observed_candidate_exact_immutable_payload_bytes =
            application_render_canvas_uploads
                .observed_candidate_exact_immutable_payload_logical_bytes,
        gpu_surface_render_canvas_upload_observed_candidate_exact_volatile_payload_operations =
            application_render_canvas_uploads.observed_candidate_exact_volatile_payload_operations,
        gpu_surface_render_canvas_upload_observed_candidate_exact_volatile_payload_bytes =
            application_render_canvas_uploads
                .observed_candidate_exact_volatile_payload_logical_bytes,
        gpu_surface_render_canvas_upload_observed_candidate_exact_renderer_parameter_operations =
            application_render_canvas_uploads
                .observed_candidate_exact_renderer_parameter_operations,
        gpu_surface_render_canvas_upload_observed_candidate_exact_renderer_parameter_bytes =
            application_render_canvas_uploads
                .observed_candidate_exact_renderer_parameter_logical_bytes,
        gpu_surface_render_canvas_upload_application_immutable_payload_operations =
            application_render_canvas_uploads.immutable_payload_operations,
        gpu_surface_render_canvas_upload_application_immutable_payload_bytes =
            application_render_canvas_uploads.immutable_payload_logical_bytes,
        gpu_surface_render_canvas_upload_application_volatile_payload_operations =
            application_render_canvas_uploads.volatile_payload_operations,
        gpu_surface_render_canvas_upload_application_volatile_payload_bytes =
            application_render_canvas_uploads.volatile_payload_logical_bytes,
        gpu_surface_render_canvas_upload_application_renderer_parameter_operations =
            application_render_canvas_uploads.renderer_parameter_operations,
        gpu_surface_render_canvas_upload_application_renderer_parameter_bytes =
            application_render_canvas_uploads.renderer_parameter_logical_bytes,
        gpu_surface_render_canvas_upload_immutable_payload_operations =
            render_canvas_uploads.immutable_payload.operations,
        gpu_surface_render_canvas_upload_immutable_payload_bytes =
            render_canvas_uploads.immutable_payload.logical_bytes,
        gpu_surface_render_canvas_upload_volatile_payload_operations =
            render_canvas_uploads.volatile_payload.operations,
        gpu_surface_render_canvas_upload_volatile_payload_bytes =
            render_canvas_uploads.volatile_payload.logical_bytes,
        gpu_surface_render_canvas_upload_renderer_parameter_operations =
            render_canvas_uploads.renderer_parameter.operations,
        gpu_surface_render_canvas_upload_renderer_parameter_bytes =
            render_canvas_uploads.renderer_parameter.logical_bytes,
        gpu_surface_render_canvas_upload_candidate_plan = ?render_canvas_upload_plan,
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
    info!(
        reason,
        window_identity = frame
            .window_identity
            .map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
        gpu_surface_target_texture_active_generation_known = active_target.generation_known,
        gpu_surface_target_texture_active_generation_serial = active_target.generation_serial,
        gpu_surface_target_texture_active_resident_count = active_target.resident_count,
        gpu_surface_target_texture_active_requested_rgba8_bytes =
            active_target.requested_rgba8_bytes,
        gpu_surface_target_texture_active_predecessor_object_count =
            active_target.predecessor_object_count,
        gpu_surface_target_texture_active_predecessor_requested_rgba8_bytes =
            active_target.predecessor_requested_rgba8_bytes,
        gpu_surface_target_texture_q0_generation_known = quarantine_0_target.generation_known,
        gpu_surface_target_texture_q0_generation_serial = quarantine_0_target.generation_serial,
        gpu_surface_target_texture_q0_resident_count = quarantine_0_target.resident_count,
        gpu_surface_target_texture_q0_requested_rgba8_bytes =
            quarantine_0_target.requested_rgba8_bytes,
        gpu_surface_target_texture_q0_predecessor_object_count =
            quarantine_0_target.predecessor_object_count,
        gpu_surface_target_texture_q0_predecessor_requested_rgba8_bytes =
            quarantine_0_target.predecessor_requested_rgba8_bytes,
        gpu_surface_target_texture_q1_generation_known = quarantine_1_target.generation_known,
        gpu_surface_target_texture_q1_generation_serial = quarantine_1_target.generation_serial,
        gpu_surface_target_texture_q1_resident_count = quarantine_1_target.resident_count,
        gpu_surface_target_texture_q1_requested_rgba8_bytes =
            quarantine_1_target.requested_rgba8_bytes,
        gpu_surface_target_texture_q1_predecessor_object_count =
            quarantine_1_target.predecessor_object_count,
        gpu_surface_target_texture_q1_predecessor_requested_rgba8_bytes =
            quarantine_1_target.predecessor_requested_rgba8_bytes,
        "radiant native render profile target texture residency"
    );
    info!(
        reason,
        window_identity = frame
            .window_identity
            .map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
        gpu_surface_composited_base_active_generation_known =
            active_composited_base.generation_known,
        gpu_surface_composited_base_active_generation_serial =
            active_composited_base.generation_serial,
        gpu_surface_composited_base_active_object_count =
            active_composited_base.active_object_count,
        gpu_surface_composited_base_active_retired_object_count =
            active_composited_base.retired_object_count,
        gpu_surface_composited_base_active_requested_backing_bytes =
            active_composited_base.active_requested_backing_bytes,
        gpu_surface_composited_base_active_retired_requested_backing_bytes =
            active_composited_base.retired_requested_backing_bytes,
        gpu_surface_composited_base_q0_generation_known =
            quarantine_0_composited_base.generation_known,
        gpu_surface_composited_base_q0_generation_serial =
            quarantine_0_composited_base.generation_serial,
        gpu_surface_composited_base_q0_active_object_count =
            quarantine_0_composited_base.active_object_count,
        gpu_surface_composited_base_q0_retired_object_count =
            quarantine_0_composited_base.retired_object_count,
        gpu_surface_composited_base_q0_active_requested_backing_bytes =
            quarantine_0_composited_base.active_requested_backing_bytes,
        gpu_surface_composited_base_q0_retired_requested_backing_bytes =
            quarantine_0_composited_base.retired_requested_backing_bytes,
        gpu_surface_composited_base_q1_generation_known =
            quarantine_1_composited_base.generation_known,
        gpu_surface_composited_base_q1_generation_serial =
            quarantine_1_composited_base.generation_serial,
        gpu_surface_composited_base_q1_active_object_count =
            quarantine_1_composited_base.active_object_count,
        gpu_surface_composited_base_q1_retired_object_count =
            quarantine_1_composited_base.retired_object_count,
        gpu_surface_composited_base_q1_active_requested_backing_bytes =
            quarantine_1_composited_base.active_requested_backing_bytes,
        gpu_surface_composited_base_q1_retired_requested_backing_bytes =
            quarantine_1_composited_base.retired_requested_backing_bytes,
        "radiant native render profile composited base residency"
    );
    info!(
        reason,
        window_identity = frame
            .window_identity
            .map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
        gpu_surface_custom_shader_active_generation_known = active_custom_shader.generation_known,
        gpu_surface_custom_shader_active_generation_serial = active_custom_shader.generation_serial,
        gpu_surface_custom_shader_active_pipeline_resident_count =
            active_custom_shader.pipeline_resident_count,
        gpu_surface_custom_shader_active_binding_resident_count =
            active_custom_shader.binding_resident_count,
        gpu_surface_custom_shader_active_surface_uniform_logical_bytes =
            active_custom_shader.surface_uniform_logical_bytes,
        gpu_surface_custom_shader_active_app_uniform_logical_bytes =
            active_custom_shader.app_uniform_logical_bytes,
        gpu_surface_custom_shader_active_storage_logical_bytes =
            active_custom_shader.storage_logical_bytes,
        gpu_surface_custom_shader_active_presentation_uniform_logical_bytes =
            active_custom_shader.presentation_uniform_logical_bytes,
        gpu_surface_custom_shader_q0_generation_known = quarantine_0_custom_shader.generation_known,
        gpu_surface_custom_shader_q0_generation_serial =
            quarantine_0_custom_shader.generation_serial,
        gpu_surface_custom_shader_q0_pipeline_resident_count =
            quarantine_0_custom_shader.pipeline_resident_count,
        gpu_surface_custom_shader_q0_binding_resident_count =
            quarantine_0_custom_shader.binding_resident_count,
        gpu_surface_custom_shader_q0_surface_uniform_logical_bytes =
            quarantine_0_custom_shader.surface_uniform_logical_bytes,
        gpu_surface_custom_shader_q0_app_uniform_logical_bytes =
            quarantine_0_custom_shader.app_uniform_logical_bytes,
        gpu_surface_custom_shader_q0_storage_logical_bytes =
            quarantine_0_custom_shader.storage_logical_bytes,
        gpu_surface_custom_shader_q0_presentation_uniform_logical_bytes =
            quarantine_0_custom_shader.presentation_uniform_logical_bytes,
        gpu_surface_custom_shader_q1_generation_known = quarantine_1_custom_shader.generation_known,
        gpu_surface_custom_shader_q1_generation_serial =
            quarantine_1_custom_shader.generation_serial,
        gpu_surface_custom_shader_q1_pipeline_resident_count =
            quarantine_1_custom_shader.pipeline_resident_count,
        gpu_surface_custom_shader_q1_binding_resident_count =
            quarantine_1_custom_shader.binding_resident_count,
        gpu_surface_custom_shader_q1_surface_uniform_logical_bytes =
            quarantine_1_custom_shader.surface_uniform_logical_bytes,
        gpu_surface_custom_shader_q1_app_uniform_logical_bytes =
            quarantine_1_custom_shader.app_uniform_logical_bytes,
        gpu_surface_custom_shader_q1_storage_logical_bytes =
            quarantine_1_custom_shader.storage_logical_bytes,
        gpu_surface_custom_shader_q1_presentation_uniform_logical_bytes =
            quarantine_1_custom_shader.presentation_uniform_logical_bytes,
        gpu_surface_custom_shader_application_adapter_generation_known =
            application_custom_shader_residency
                .adapter_generation
                .map(|generation| generation.is_known()),
        gpu_surface_custom_shader_application_adapter_generation_serial =
            application_custom_shader_residency
                .adapter_generation
                .and_then(|generation| generation.known_serial()),
        gpu_surface_custom_shader_application_active_pipeline_resident_count =
            application_custom_shader_residency.active_pipeline_resident_count,
        gpu_surface_custom_shader_application_active_binding_resident_count =
            application_custom_shader_residency.active_binding_resident_count,
        gpu_surface_custom_shader_application_active_surface_uniform_logical_bytes =
            application_custom_shader_residency.active_surface_uniform_logical_bytes,
        gpu_surface_custom_shader_application_active_app_uniform_logical_bytes =
            application_custom_shader_residency.active_app_uniform_logical_bytes,
        gpu_surface_custom_shader_application_active_storage_logical_bytes =
            application_custom_shader_residency.active_storage_logical_bytes,
        gpu_surface_custom_shader_application_active_presentation_uniform_logical_bytes =
            application_custom_shader_residency.active_presentation_uniform_logical_bytes,
        gpu_surface_custom_shader_application_quarantined_pipeline_resident_count =
            application_custom_shader_residency.quarantined_pipeline_resident_count,
        gpu_surface_custom_shader_application_quarantined_binding_resident_count =
            application_custom_shader_residency.quarantined_binding_resident_count,
        gpu_surface_custom_shader_application_quarantined_surface_uniform_logical_bytes =
            application_custom_shader_residency.quarantined_surface_uniform_logical_bytes,
        gpu_surface_custom_shader_application_quarantined_app_uniform_logical_bytes =
            application_custom_shader_residency.quarantined_app_uniform_logical_bytes,
        gpu_surface_custom_shader_application_quarantined_storage_logical_bytes =
            application_custom_shader_residency.quarantined_storage_logical_bytes,
        gpu_surface_custom_shader_application_quarantined_presentation_uniform_logical_bytes =
            application_custom_shader_residency.quarantined_presentation_uniform_logical_bytes,
        "radiant native render profile custom shader residency"
    );
    info!(
        reason,
        window_identity = frame
            .window_identity
            .map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
        gpu_surface_signal_active_generation_known = active_signal.generation_known,
        gpu_surface_signal_active_generation_serial = active_signal.generation_serial,
        gpu_surface_signal_active_buffer_resident_count =
            active_signal.signal_buffer_resident_count,
        gpu_surface_signal_active_buffer_logical_bytes = active_signal.signal_buffer_logical_bytes,
        gpu_surface_signal_active_body_texture_resident_count =
            active_signal.signal_body_texture_resident_count,
        gpu_surface_signal_active_body_texture_logical_rgba_bytes =
            active_signal.signal_body_texture_logical_rgba_bytes,
        gpu_surface_signal_q0_generation_known = quarantine_0_signal.generation_known,
        gpu_surface_signal_q0_generation_serial = quarantine_0_signal.generation_serial,
        gpu_surface_signal_q0_buffer_resident_count =
            quarantine_0_signal.signal_buffer_resident_count,
        gpu_surface_signal_q0_buffer_logical_bytes =
            quarantine_0_signal.signal_buffer_logical_bytes,
        gpu_surface_signal_q0_body_texture_resident_count =
            quarantine_0_signal.signal_body_texture_resident_count,
        gpu_surface_signal_q0_body_texture_logical_rgba_bytes =
            quarantine_0_signal.signal_body_texture_logical_rgba_bytes,
        gpu_surface_signal_q1_generation_known = quarantine_1_signal.generation_known,
        gpu_surface_signal_q1_generation_serial = quarantine_1_signal.generation_serial,
        gpu_surface_signal_q1_buffer_resident_count =
            quarantine_1_signal.signal_buffer_resident_count,
        gpu_surface_signal_q1_buffer_logical_bytes =
            quarantine_1_signal.signal_buffer_logical_bytes,
        gpu_surface_signal_q1_body_texture_resident_count =
            quarantine_1_signal.signal_body_texture_resident_count,
        gpu_surface_signal_q1_body_texture_logical_rgba_bytes =
            quarantine_1_signal.signal_body_texture_logical_rgba_bytes,
        "radiant native render profile signal residency"
    );
    info!(
        reason,
        window_identity = frame
            .window_identity
            .map(NativeWindowDiagnosticIdentity::get),
        frame_sequence = frame.frame_sequence,
        gpu_surface_signal_application_adapter_generation_known = application_signal_residency
            .adapter_generation
            .map(|generation| generation.is_known()),
        gpu_surface_signal_application_adapter_generation_serial = application_signal_residency
            .adapter_generation
            .and_then(|generation| generation.known_serial()),
        gpu_surface_signal_application_active_buffer_resident_count =
            application_signal_residency.active_signal_buffer_resident_count,
        gpu_surface_signal_application_active_buffer_logical_bytes =
            application_signal_residency.active_signal_buffer_logical_bytes,
        gpu_surface_signal_application_active_body_texture_resident_count =
            application_signal_residency.active_signal_body_texture_resident_count,
        gpu_surface_signal_application_active_body_texture_logical_rgba_bytes =
            application_signal_residency.active_signal_body_texture_logical_rgba_bytes,
        gpu_surface_signal_application_quarantined_buffer_resident_count =
            application_signal_residency.quarantined_signal_buffer_resident_count,
        gpu_surface_signal_application_quarantined_buffer_logical_bytes =
            application_signal_residency.quarantined_signal_buffer_logical_bytes,
        gpu_surface_signal_application_quarantined_body_texture_resident_count =
            application_signal_residency.quarantined_signal_body_texture_resident_count,
        gpu_surface_signal_application_quarantined_body_texture_logical_rgba_bytes =
            application_signal_residency.quarantined_signal_body_texture_logical_rgba_bytes,
        "radiant native render profile application signal residency"
    );
}

pub(super) fn maybe_log_slow_render_profile(
    reason: &'static str,
    stats: RetainedSurfaceEncodeStats,
    render_to_texture_elapsed: Duration,
    frame: RenderFrameProfile,
    gpu_surface_stats: GpuSurfaceRenderStats,
    application_render_canvas_uploads: NativeAdapterRenderCanvasUploadProfile,
    since_last_present: Duration,
) {
    if !slow_render_profile_enabled() {
        return;
    }
    let cpu_envelope_total = tracked_cpu_envelope_total(frame, render_to_texture_elapsed);
    let render_canvas_uploads = gpu_surface_stats.render_canvas_uploads;
    let render_canvas_upload_plan = gpu_surface_stats.render_canvas_upload_plan;
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
        gpu_surface_render_canvas_upload_application_adapter_generation_known =
            application_render_canvas_uploads
                .adapter_generation
                .map(|generation| generation.is_known()),
        gpu_surface_render_canvas_upload_application_adapter_generation_serial =
            application_render_canvas_uploads
                .adapter_generation
                .and_then(|generation| generation.known_serial()),
        gpu_surface_render_canvas_upload_observed_candidate_plan_count =
            application_render_canvas_uploads.observed_candidate_plan_count,
        gpu_surface_render_canvas_upload_observed_candidate_plan_window_count =
            application_render_canvas_uploads.observed_candidate_plan_window_count,
        gpu_surface_render_canvas_upload_observed_candidate_no_work_count =
            application_render_canvas_uploads.observed_candidate_no_work_count,
        gpu_surface_render_canvas_upload_observed_candidate_exact_count =
            application_render_canvas_uploads.observed_candidate_exact_count,
        gpu_surface_render_canvas_upload_observed_candidate_invalid_count =
            application_render_canvas_uploads.observed_candidate_invalid_count,
        gpu_surface_render_canvas_upload_observed_candidate_unsupported_count =
            application_render_canvas_uploads.observed_candidate_unsupported_count,
        gpu_surface_render_canvas_upload_observed_candidate_incomplete_count =
            application_render_canvas_uploads.observed_candidate_incomplete_count,
        gpu_surface_render_canvas_upload_observed_candidate_overflow_count =
            application_render_canvas_uploads.observed_candidate_overflow_count,
        gpu_surface_render_canvas_upload_observed_candidate_exact_immutable_payload_operations =
            application_render_canvas_uploads
                .observed_candidate_exact_immutable_payload_operations,
        gpu_surface_render_canvas_upload_observed_candidate_exact_immutable_payload_bytes =
            application_render_canvas_uploads
                .observed_candidate_exact_immutable_payload_logical_bytes,
        gpu_surface_render_canvas_upload_observed_candidate_exact_volatile_payload_operations =
            application_render_canvas_uploads.observed_candidate_exact_volatile_payload_operations,
        gpu_surface_render_canvas_upload_observed_candidate_exact_volatile_payload_bytes =
            application_render_canvas_uploads
                .observed_candidate_exact_volatile_payload_logical_bytes,
        gpu_surface_render_canvas_upload_observed_candidate_exact_renderer_parameter_operations =
            application_render_canvas_uploads
                .observed_candidate_exact_renderer_parameter_operations,
        gpu_surface_render_canvas_upload_observed_candidate_exact_renderer_parameter_bytes =
            application_render_canvas_uploads
                .observed_candidate_exact_renderer_parameter_logical_bytes,
        gpu_surface_render_canvas_upload_application_immutable_payload_operations =
            application_render_canvas_uploads.immutable_payload_operations,
        gpu_surface_render_canvas_upload_application_immutable_payload_bytes =
            application_render_canvas_uploads.immutable_payload_logical_bytes,
        gpu_surface_render_canvas_upload_application_volatile_payload_operations =
            application_render_canvas_uploads.volatile_payload_operations,
        gpu_surface_render_canvas_upload_application_volatile_payload_bytes =
            application_render_canvas_uploads.volatile_payload_logical_bytes,
        gpu_surface_render_canvas_upload_application_renderer_parameter_operations =
            application_render_canvas_uploads.renderer_parameter_operations,
        gpu_surface_render_canvas_upload_application_renderer_parameter_bytes =
            application_render_canvas_uploads.renderer_parameter_logical_bytes,
        gpu_surface_render_canvas_upload_immutable_payload_operations = render_canvas_uploads
            .immutable_payload
            .operations,
        gpu_surface_render_canvas_upload_immutable_payload_bytes = render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        gpu_surface_render_canvas_upload_volatile_payload_operations = render_canvas_uploads
            .volatile_payload
            .operations,
        gpu_surface_render_canvas_upload_volatile_payload_bytes = render_canvas_uploads
            .volatile_payload
            .logical_bytes,
        gpu_surface_render_canvas_upload_renderer_parameter_operations = render_canvas_uploads
            .renderer_parameter
            .operations,
        gpu_surface_render_canvas_upload_renderer_parameter_bytes = render_canvas_uploads
            .renderer_parameter
            .logical_bytes,
        gpu_surface_render_canvas_upload_candidate_plan = ?render_canvas_upload_plan,
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
        GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCompositedBaseResidencySnapshot,
        GpuSurfaceCustomShaderResidencySnapshot, GpuSurfaceSignalResidencySnapshot,
        GpuSurfaceTargetResidencySnapshot,
        adapter::NativeAdapterGeneration,
        runner_state::{
            NativeWindowAtlasResidencySnapshots, NativeWindowCompositedBaseResidencySnapshots,
            NativeWindowCustomShaderResidencySnapshots, NativeWindowSignalResidencySnapshots,
            NativeWindowTargetResidencySnapshots,
        },
    };
    use super::{
        NativeRenderProfileAtlasResidency, NativeRenderProfileCompositedBaseResidency,
        NativeRenderProfileCustomShaderResidency, NativeRenderProfileSignalResidency,
        NativeRenderProfileTargetResidency, project_atlas_residency,
        project_composited_base_residency, project_custom_shader_residency,
        project_signal_residency, project_target_residency,
    };

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

    #[test]
    fn composited_base_profile_projection_preserves_counts_and_requested_bytes() {
        let snapshots = NativeWindowCompositedBaseResidencySnapshots {
            active: Some(GpuSurfaceCompositedBaseResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(61),
                active_object_count: 1,
                retired_object_count: 1,
                active_requested_backing_bytes: Some(921_600),
                retired_requested_backing_bytes: Some(512),
            }),
            quarantine_0: Some(GpuSurfaceCompositedBaseResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(62),
                active_object_count: 0,
                retired_object_count: 1,
                active_requested_backing_bytes: None,
                retired_requested_backing_bytes: Some(256),
            }),
            quarantine_1: None,
        };

        assert_eq!(
            snapshots
                .active
                .map(GpuSurfaceCompositedBaseResidencySnapshot::generation),
            Some(NativeAdapterGeneration::from_test_serial(61))
        );

        assert_eq!(
            project_composited_base_residency(snapshots.active),
            NativeRenderProfileCompositedBaseResidency {
                generation_known: Some(true),
                generation_serial: Some(61),
                active_object_count: Some(1),
                retired_object_count: Some(1),
                active_requested_backing_bytes: Some(921_600),
                retired_requested_backing_bytes: Some(512),
            }
        );
        assert_eq!(
            project_composited_base_residency(snapshots.quarantine_0),
            NativeRenderProfileCompositedBaseResidency {
                generation_known: Some(true),
                generation_serial: Some(62),
                active_object_count: Some(0),
                retired_object_count: Some(1),
                active_requested_backing_bytes: None,
                retired_requested_backing_bytes: Some(256),
            }
        );
        assert_eq!(
            project_composited_base_residency(snapshots.quarantine_1),
            NativeRenderProfileCompositedBaseResidency::default()
        );
    }

    #[test]
    fn composited_base_profile_projection_keeps_unknown_and_exhausted_generation_untrusted() {
        let unknown = GpuSurfaceCompositedBaseResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            active_object_count: 1,
            retired_object_count: 0,
            active_requested_backing_bytes: Some(4),
            retired_requested_backing_bytes: None,
        };
        let mut exhausted_generation = NativeAdapterGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted_generation.advance());
        let exhausted = GpuSurfaceCompositedBaseResidencySnapshot {
            generation: exhausted_generation,
            active_object_count: 1,
            retired_object_count: 1,
            active_requested_backing_bytes: Some(8),
            retired_requested_backing_bytes: Some(8),
        };

        for snapshot in [unknown, exhausted] {
            let projection = project_composited_base_residency(Some(snapshot));
            assert_eq!(projection.generation_known, Some(false));
            assert_eq!(projection.generation_serial, None);
            assert_eq!(projection.active_object_count, Some(1));
        }
    }

    #[test]
    fn target_profile_projection_preserves_active_q0_q1_evidence() {
        let snapshots = NativeWindowTargetResidencySnapshots {
            active: Some(GpuSurfaceTargetResidencySnapshot::from_surface_config(
                NativeAdapterGeneration::from_test_serial(71),
                640,
                360,
            )),
            quarantine_0: Some(GpuSurfaceTargetResidencySnapshot::from_surface_config(
                NativeAdapterGeneration::from_test_serial(72),
                320,
                200,
            )),
            quarantine_1: Some(GpuSurfaceTargetResidencySnapshot::from_surface_config(
                NativeAdapterGeneration::from_test_serial(73),
                1,
                1,
            )),
        };

        assert_eq!(
            project_target_residency(snapshots.active),
            NativeRenderProfileTargetResidency {
                generation_known: Some(true),
                generation_serial: Some(71),
                resident_count: Some(1),
                requested_rgba8_bytes: Some(921_600),
                predecessor_object_count: Some(0),
                predecessor_requested_rgba8_bytes: None,
            }
        );
        assert_eq!(
            project_target_residency(snapshots.quarantine_0),
            NativeRenderProfileTargetResidency {
                generation_known: Some(true),
                generation_serial: Some(72),
                resident_count: Some(1),
                requested_rgba8_bytes: Some(256_000),
                predecessor_object_count: Some(0),
                predecessor_requested_rgba8_bytes: None,
            }
        );
        assert_eq!(
            project_target_residency(snapshots.quarantine_1),
            NativeRenderProfileTargetResidency {
                generation_known: Some(true),
                generation_serial: Some(73),
                resident_count: Some(1),
                requested_rgba8_bytes: Some(4),
                predecessor_object_count: Some(0),
                predecessor_requested_rgba8_bytes: None,
            }
        );
        assert_eq!(
            project_target_residency(None),
            NativeRenderProfileTargetResidency::default()
        );
    }

    #[test]
    fn target_profile_projection_suppresses_unknown_and_exhausted_generation_serials() {
        let unknown = GpuSurfaceTargetResidencySnapshot {
            generation: NativeAdapterGeneration::unknown(),
            active_object_count: 1,
            predecessor_object_count: 0,
            active_requested_rgba8_bytes: Some(4),
            predecessor_requested_rgba8_bytes: None,
        };
        let mut exhausted_generation = NativeAdapterGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted_generation.advance());
        let exhausted = GpuSurfaceTargetResidencySnapshot {
            generation: exhausted_generation,
            active_object_count: 1,
            predecessor_object_count: 0,
            active_requested_rgba8_bytes: Some(8),
            predecessor_requested_rgba8_bytes: None,
        };

        for snapshot in [unknown, exhausted] {
            let projection = project_target_residency(Some(snapshot));
            assert_eq!(projection.generation_known, Some(false));
            assert_eq!(projection.generation_serial, None);
            assert_eq!(projection.resident_count, Some(1));
            assert_eq!(
                projection.requested_rgba8_bytes,
                snapshot.active_requested_rgba8_bytes
            );
            assert_eq!(projection.predecessor_object_count, Some(0));
            assert_eq!(projection.predecessor_requested_rgba8_bytes, None);
        }
    }

    #[test]
    fn target_profile_projection_preserves_predecessor_diagnostics() {
        let projection = project_target_residency(Some(GpuSurfaceTargetResidencySnapshot {
            generation: NativeAdapterGeneration::from_test_serial(74),
            active_object_count: 1,
            predecessor_object_count: 1,
            active_requested_rgba8_bytes: Some(921_600),
            predecessor_requested_rgba8_bytes: Some(1_024),
        }));

        assert_eq!(projection.resident_count, Some(1));
        assert_eq!(projection.requested_rgba8_bytes, Some(921_600));
        assert_eq!(projection.predecessor_object_count, Some(1));
        assert_eq!(projection.predecessor_requested_rgba8_bytes, Some(1_024));
    }

    #[test]
    fn signal_profile_projection_preserves_active_q0_q1_and_unknown_bytes() {
        let snapshots = NativeWindowSignalResidencySnapshots {
            active: Some(GpuSurfaceSignalResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(21),
                signal_buffer_resident_count: 3,
                signal_buffer_logical_bytes: Some(456),
                signal_body_texture_resident_count: 2,
                signal_body_texture_logical_rgba_bytes: Some(8_192),
            }),
            quarantine_0: Some(GpuSurfaceSignalResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(22),
                signal_buffer_resident_count: 1,
                signal_buffer_logical_bytes: None,
                signal_body_texture_resident_count: 1,
                signal_body_texture_logical_rgba_bytes: Some(4),
            }),
            quarantine_1: Some(GpuSurfaceSignalResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(23),
                signal_buffer_resident_count: 0,
                signal_buffer_logical_bytes: Some(0),
                signal_body_texture_resident_count: 0,
                signal_body_texture_logical_rgba_bytes: Some(0),
            }),
        };

        assert_eq!(
            project_signal_residency(snapshots.active),
            NativeRenderProfileSignalResidency {
                generation_known: Some(true),
                generation_serial: Some(21),
                signal_buffer_resident_count: Some(3),
                signal_buffer_logical_bytes: Some(456),
                signal_body_texture_resident_count: Some(2),
                signal_body_texture_logical_rgba_bytes: Some(8_192),
            }
        );
        assert_eq!(
            project_signal_residency(snapshots.quarantine_0),
            NativeRenderProfileSignalResidency {
                generation_known: Some(true),
                generation_serial: Some(22),
                signal_buffer_resident_count: Some(1),
                signal_buffer_logical_bytes: None,
                signal_body_texture_resident_count: Some(1),
                signal_body_texture_logical_rgba_bytes: Some(4),
            }
        );
        assert_eq!(
            project_signal_residency(snapshots.quarantine_1),
            NativeRenderProfileSignalResidency {
                generation_known: Some(true),
                generation_serial: Some(23),
                signal_buffer_resident_count: Some(0),
                signal_buffer_logical_bytes: Some(0),
                signal_body_texture_resident_count: Some(0),
                signal_body_texture_logical_rgba_bytes: Some(0),
            }
        );

        let unknown = project_signal_residency(Some(GpuSurfaceSignalResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            signal_buffer_resident_count: 1,
            signal_buffer_logical_bytes: None,
            signal_body_texture_resident_count: 1,
            signal_body_texture_logical_rgba_bytes: None,
        }));
        assert_eq!(unknown.generation_known, Some(false));
        assert_eq!(unknown.generation_serial, None);
        assert_eq!(
            project_signal_residency(None),
            NativeRenderProfileSignalResidency::default()
        );
    }

    #[test]
    fn custom_shader_profile_projection_preserves_active_q0_q1_counts_and_unknown_bytes() {
        let snapshots = NativeWindowCustomShaderResidencySnapshots {
            active: Some(GpuSurfaceCustomShaderResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(31),
                pipeline_resident_count: 3,
                binding_resident_count: 2,
                surface_uniform_logical_bytes: Some(128),
                app_uniform_logical_bytes: Some(16),
                storage_logical_bytes: Some(24),
                presentation_uniform_logical_bytes: Some(32),
            }),
            quarantine_0: Some(GpuSurfaceCustomShaderResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(32),
                pipeline_resident_count: 1,
                binding_resident_count: 1,
                surface_uniform_logical_bytes: Some(64),
                app_uniform_logical_bytes: None,
                storage_logical_bytes: Some(8),
                presentation_uniform_logical_bytes: Some(0),
            }),
            quarantine_1: Some(GpuSurfaceCustomShaderResidencySnapshot {
                generation: NativeAdapterGeneration::from_test_serial(33),
                pipeline_resident_count: 0,
                binding_resident_count: 0,
                surface_uniform_logical_bytes: Some(0),
                app_uniform_logical_bytes: Some(0),
                storage_logical_bytes: Some(0),
                presentation_uniform_logical_bytes: Some(0),
            }),
        };

        assert_eq!(
            project_custom_shader_residency(snapshots.active),
            NativeRenderProfileCustomShaderResidency {
                generation_known: Some(true),
                generation_serial: Some(31),
                pipeline_resident_count: Some(3),
                binding_resident_count: Some(2),
                surface_uniform_logical_bytes: Some(128),
                app_uniform_logical_bytes: Some(16),
                storage_logical_bytes: Some(24),
                presentation_uniform_logical_bytes: Some(32),
            }
        );
        assert_eq!(
            project_custom_shader_residency(snapshots.quarantine_0),
            NativeRenderProfileCustomShaderResidency {
                generation_known: Some(true),
                generation_serial: Some(32),
                pipeline_resident_count: Some(1),
                binding_resident_count: Some(1),
                surface_uniform_logical_bytes: Some(64),
                app_uniform_logical_bytes: None,
                storage_logical_bytes: Some(8),
                presentation_uniform_logical_bytes: Some(0),
            }
        );
        assert_eq!(
            project_custom_shader_residency(snapshots.quarantine_1),
            NativeRenderProfileCustomShaderResidency {
                generation_known: Some(true),
                generation_serial: Some(33),
                pipeline_resident_count: Some(0),
                binding_resident_count: Some(0),
                surface_uniform_logical_bytes: Some(0),
                app_uniform_logical_bytes: Some(0),
                storage_logical_bytes: Some(0),
                presentation_uniform_logical_bytes: Some(0),
            }
        );
    }

    #[test]
    fn custom_shader_profile_projection_preserves_unknown_exhausted_and_absent_slots() {
        let unknown = GpuSurfaceCustomShaderResidencySnapshot {
            generation: NativeAdapterGeneration::default(),
            pipeline_resident_count: 4,
            binding_resident_count: 5,
            surface_uniform_logical_bytes: Some(64),
            app_uniform_logical_bytes: None,
            storage_logical_bytes: Some(8),
            presentation_uniform_logical_bytes: Some(0),
        };
        let mut exhausted_generation = NativeAdapterGeneration::from_test_serial(u64::MAX);
        assert!(!exhausted_generation.advance());
        let exhausted = GpuSurfaceCustomShaderResidencySnapshot {
            generation: exhausted_generation,
            pipeline_resident_count: 6,
            binding_resident_count: 7,
            surface_uniform_logical_bytes: Some(64),
            app_uniform_logical_bytes: Some(8),
            storage_logical_bytes: Some(16),
            presentation_uniform_logical_bytes: Some(0),
        };

        for snapshot in [unknown, exhausted] {
            let projection = project_custom_shader_residency(Some(snapshot));
            assert_eq!(projection.generation_known, Some(false));
            assert_eq!(projection.generation_serial, None);
            assert_eq!(
                projection.pipeline_resident_count,
                Some(snapshot.pipeline_resident_count)
            );
            assert_eq!(
                projection.binding_resident_count,
                Some(snapshot.binding_resident_count)
            );
        }
        assert_eq!(
            project_custom_shader_residency(None),
            NativeRenderProfileCustomShaderResidency::default()
        );
    }
}
