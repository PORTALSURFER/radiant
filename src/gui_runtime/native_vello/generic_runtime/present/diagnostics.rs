use super::super::{RenderFrameProfile, RetainedSurfaceEncodeStats, gpu_surface};
use crate::gui_runtime::native_vello::TextLayoutProfileCounters;
use crate::gui_runtime::native_vello::generic_runtime::FrameWork;
use std::time::Duration;

pub(super) struct NativeFrameDiagnosticsParts {
    pub(super) stats: RetainedSurfaceEncodeStats,
    pub(super) scene_encode_count: u64,
    pub(super) scene_reuse_count: u64,
    pub(super) scene_assembly_count: u64,
    pub(super) scene_assembly_veto_count: u64,
    pub(super) scene_mixed_assembly_count: u64,
    pub(super) scene_assembly_fresh_count: u64,
    pub(super) scene_assembly_reused_count: u64,
    pub(super) scene_assembly_append_count: u64,
    pub(super) scene_build_outcome: &'static str,
    pub(super) text_stats: TextLayoutProfileCounters,
    pub(super) retained_policy: crate::runtime::RetainedSurfaceCachePolicy,
    pub(super) retained_entries: usize,
    pub(super) gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    pub(super) profile: RenderFrameProfile,
    pub(super) input_to_present_latency_us: Option<u64>,
    pub(super) render_to_texture_elapsed: Duration,
    pub(super) since_last_present: Duration,
    pub(super) frame_work: FrameWork,
    pub(super) surface_refresh: crate::runtime::SurfaceRefreshDiagnostics,
    pub(super) surface_refresh_total: Duration,
    pub(super) surface_recovery: crate::runtime::NativeSurfaceRecoveryDiagnostics,
}

pub(super) fn native_frame_diagnostics(
    parts: NativeFrameDiagnosticsParts,
) -> crate::runtime::NativeFrameDiagnostics {
    let surface_invalidation =
        surface_invalidation_name(parts.frame_work, parts.surface_refresh.invalidation);
    crate::runtime::NativeFrameDiagnostics {
        window_identity: parts.profile.window_identity,
        frame_sequence: parts.profile.frame_sequence,
        input_to_present_latency_us: parts.input_to_present_latency_us,
        cpu_fairness: crate::runtime::NativeCpuFrameFairnessDiagnostics::default(),
        cpu_observation: crate::runtime::NativeCpuFrameObservationDiagnostics::default(),
        presentation: crate::runtime::NativeFramePresentationDiagnostics {
            frame_work_kind: parts.frame_work.kind(),
            frame_work_reason: parts.frame_work.reason().name(),
            surface_invalidation,
            paint_only: parts.frame_work.is_paint_only(),
            scene_rebuild: parts.frame_work.needs_scene_rebuild(),
        },
        surface_recovery: parts.surface_recovery,
        scene: crate::runtime::NativeSceneDiagnostics {
            scene_encode_count: parts.scene_encode_count,
            scene_reuse_count: parts.scene_reuse_count,
            scene_assembly_count: parts.scene_assembly_count,
            scene_assembly_veto_count: parts.scene_assembly_veto_count,
            scene_mixed_assembly_count: parts.scene_mixed_assembly_count,
            scene_assembly_fresh_count: parts.scene_assembly_fresh_count,
            scene_assembly_reused_count: parts.scene_assembly_reused_count,
            scene_assembly_append_count: parts.scene_assembly_append_count,
            scene_build_outcome: parts.scene_build_outcome,
            traversal: crate::runtime::NativeSceneTraversalDiagnostics {
                paint_plan_primitives: parts.stats.paint_plan_primitives,
                clip_layer_count: parts.stats.clip_layer_count,
            },
            text: crate::runtime::NativeSceneTextDiagnostics {
                text_primitive_count: parts.stats.text_primitive_count,
                text_input_count: parts.stats.text_input_count,
                text_run_count: parts.stats.text_run_count,
            },
            media: crate::runtime::NativeSceneMediaDiagnostics {
                image_count: parts.stats.image_count,
                svg_document_count: parts.stats.svg_document_count,
            },
            surfaces: crate::runtime::NativeSceneSurfaceDiagnostics {
                gpu_surface_count: parts.stats.gpu_surface_count,
                custom_surface_count: parts.stats.custom_surface_count,
                custom_surface_fallback_count: parts.stats.custom_surface_fallback_count,
            },
        },
        text: crate::runtime::NativeTextDiagnostics {
            cache: crate::runtime::NativeTextCacheDiagnostics {
                layout: crate::runtime::NativeTextCacheCounters {
                    hits: parts.text_stats.layout.hits,
                    misses: parts.text_stats.layout.misses,
                    evictions: parts.text_stats.layout.evictions,
                },
                atom: crate::runtime::NativeTextCacheCounters {
                    hits: parts.text_stats.atom.hits,
                    misses: parts.text_stats.atom.misses,
                    evictions: parts.text_stats.atom.evictions,
                },
            },
            quality: crate::runtime::NativeTextQualityDiagnostics {
                unsupported_shaping_runs: parts.text_stats.quality.unsupported_shaping_runs,
                unsupported_shaping_scalars: parts.text_stats.quality.unsupported_shaping_scalars,
                fallback_glyphs: parts.text_stats.quality.fallback_glyphs,
                missing_glyphs: parts.text_stats.quality.missing_glyphs,
            },
        },
        retained_surfaces: crate::runtime::NativeRetainedSurfaceDiagnostics {
            cache_capacity: parts.retained_policy.max_frames,
            cache_entries: parts.retained_entries,
            bridge_calls: parts.stats.bridge_calls,
            cache_hits: parts.stats.cache_hits,
            miss_count: parts.stats.retained_surface_miss_count,
            retained_frame_primitive_count: parts.stats.retained_frame_primitive_count,
            retained_frame_text_run_count: parts.stats.retained_frame_text_run_count,
        },
        gpu_surfaces: crate::runtime::NativeGpuSurfaceDiagnostics {
            atlas: crate::runtime::NativeGpuSurfaceAtlasDiagnostics {
                texture_uploads: parts.gpu_surface_stats.atlas.texture_uploads,
                texture_cache_hits: parts.gpu_surface_stats.atlas.texture_cache_hits,
                texture_revision_mismatches: parts
                    .gpu_surface_stats
                    .atlas
                    .texture_revision_mismatches,
                texture_content_mismatches: parts
                    .gpu_surface_stats
                    .atlas
                    .texture_content_mismatches,
            },
            signal: crate::runtime::NativeGpuSurfaceSignalDiagnostics {
                summary_builds: parts.gpu_surface_stats.signal.summary_builds,
                summary_cache_hits: parts.gpu_surface_stats.signal.summary_cache_hits,
                summary_revision_mismatches: parts
                    .gpu_surface_stats
                    .signal
                    .summary_revision_mismatches,
                summary_content_mismatches: parts
                    .gpu_surface_stats
                    .signal
                    .summary_content_mismatches,
                body_renders: parts.gpu_surface_stats.signal.body_renders,
                body_cache_hits: parts.gpu_surface_stats.signal.body_cache_hits,
                body_revision_mismatches: parts.gpu_surface_stats.signal.body_revision_mismatches,
                body_content_mismatches: parts.gpu_surface_stats.signal.body_content_mismatches,
            },
            composite: crate::runtime::NativeGpuSurfaceCompositeDiagnostics {
                binding_rebuilds: parts.gpu_surface_stats.composite.binding_rebuilds,
                binding_cache_hits: parts.gpu_surface_stats.composite.binding_cache_hits,
                binding_revision_mismatches: parts
                    .gpu_surface_stats
                    .composite
                    .binding_revision_mismatches,
                binding_content_mismatches: parts
                    .gpu_surface_stats
                    .composite
                    .binding_content_mismatches,
            },
            custom_shader: crate::runtime::NativeGpuSurfaceCustomShaderDiagnostics {
                surfaces_rendered: parts.gpu_surface_stats.custom_shader.surfaces_rendered,
                pipeline_rebuilds: parts.gpu_surface_stats.custom_shader.pipeline_rebuilds,
                binding_rebuilds: parts.gpu_surface_stats.custom_shader.binding_rebuilds,
                binding_cache_hits: parts.gpu_surface_stats.custom_shader.binding_cache_hits,
                failures: crate::runtime::NativeGpuSurfaceCustomShaderFailureDiagnostics {
                    surfaces_failed: parts
                        .gpu_surface_stats
                        .custom_shader
                        .failures
                        .surfaces_failed,
                    shader_module_failures: parts
                        .gpu_surface_stats
                        .custom_shader
                        .failures
                        .shader_module_failures,
                    pipeline_failures: parts
                        .gpu_surface_stats
                        .custom_shader
                        .failures
                        .pipeline_failures,
                    binding_failures: parts
                        .gpu_surface_stats
                        .custom_shader
                        .failures
                        .binding_failures,
                },
                unsupported: crate::runtime::NativeGpuSurfaceUnsupportedCustomShaderDiagnostics {
                    surfaces: parts.gpu_surface_stats.custom_shader.unsupported.surfaces,
                    vertices: parts.gpu_surface_stats.custom_shader.unsupported.vertices,
                    source_bytes: parts
                        .gpu_surface_stats
                        .custom_shader
                        .unsupported
                        .source_bytes,
                    uniform_bytes: parts
                        .gpu_surface_stats
                        .custom_shader
                        .unsupported
                        .uniform_bytes,
                    storage_bytes: parts
                        .gpu_surface_stats
                        .custom_shader
                        .unsupported
                        .storage_bytes,
                },
            },
        },
        timings: crate::runtime::NativeFrameTimingDiagnostics {
            gpu_timing_status: crate::runtime::NativeGpuTimingStatus::CpuEnvelopeOnly,
            frame_work: crate::runtime::NativeFrameWorkTimings {
                coalesced_wheel_route: parts.profile.coalesced_wheel_route,
                refresh_surface: parts.surface_refresh_total,
                application_projection: parts.surface_refresh.timings.application_projection,
                runtime_projection: parts.surface_refresh.timings.runtime_projection,
                widget_state_sync: parts.surface_refresh.timings.widget_state_sync,
                layout: parts.surface_refresh.timings.layout,
                paint_plan: parts.profile.paint_plan,
                render_to_texture: parts.render_to_texture_elapsed,
                full_screen_blit: parts.profile.full_screen_blit,
            },
            composited_base: crate::runtime::NativeCompositedBaseTiming {
                refresh: parts.profile.composited_base_refresh,
                cache_hit: parts.profile.composited_base_cache_hit,
            },
            transient_overlay: crate::runtime::NativeTransientOverlayTiming {
                paint: parts.profile.transient_overlay_paint,
                primitives: parts.profile.transient_overlay_primitives,
            },
            submit_present: parts.profile.submit_present,
            since_last_present: parts.since_last_present,
        },
    }
}

fn surface_invalidation_name(
    frame_work: FrameWork,
    invalidation: crate::runtime::SurfaceInvalidation,
) -> &'static str {
    if invalidation == crate::runtime::SurfaceInvalidation::None && frame_work.is_paint_only() {
        return crate::runtime::SurfaceInvalidation::PaintOnly.name();
    }
    invalidation.name()
}

#[cfg(test)]
mod tests {
    use super::{NativeFrameDiagnosticsParts, native_frame_diagnostics, surface_invalidation_name};
    use crate::gui_runtime::native_vello::generic_runtime::{
        FrameWork, FrameWorkReason, RenderFrameProfile, SceneRebuildMode,
    };
    use crate::runtime::{
        RetainedSurfaceCachePolicy, SurfaceInvalidation, SurfaceRefreshDiagnostics,
        SurfaceRefreshTimings,
    };
    use std::time::Duration;

    #[test]
    fn paint_only_frame_reports_paint_only_without_a_runtime_refresh() {
        assert_eq!(
            surface_invalidation_name(
                FrameWork::PaintOnly {
                    reason: FrameWorkReason::TimedPaintOnlyAnimation,
                },
                SurfaceInvalidation::None,
            ),
            "paint_only"
        );
        assert_eq!(
            surface_invalidation_name(
                FrameWork::RebuildScene {
                    reason: FrameWorkReason::RoutedInput,
                    mode: SceneRebuildMode::Immediate,
                },
                SurfaceInvalidation::None,
            ),
            "none"
        );
        assert_eq!(
            surface_invalidation_name(FrameWork::None, SurfaceInvalidation::Surface),
            "surface"
        );
    }

    #[test]
    fn native_frame_diagnostics_use_the_accumulated_refresh_total() {
        let diagnostics = native_frame_diagnostics(NativeFrameDiagnosticsParts {
            stats: Default::default(),
            scene_encode_count: 7,
            scene_reuse_count: 11,
            scene_assembly_count: 13,
            scene_assembly_veto_count: 17,
            scene_mixed_assembly_count: 19,
            scene_assembly_fresh_count: 23,
            scene_assembly_reused_count: 29,
            scene_assembly_append_count: 31,
            scene_build_outcome: "retained_assembly_veto_fallback",
            text_stats: Default::default(),
            retained_policy: RetainedSurfaceCachePolicy::default(),
            retained_entries: 0,
            gpu_surface_stats: Default::default(),
            profile: RenderFrameProfile {
                window_identity: Some(
                    crate::runtime::NativeWindowDiagnosticIdentity::from_runtime_value(7),
                ),
                frame_sequence: Some(41),
                ..RenderFrameProfile::default()
            },
            input_to_present_latency_us: Some(29_000),
            render_to_texture_elapsed: Duration::ZERO,
            since_last_present: Duration::ZERO,
            frame_work: FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::Immediate,
            },
            surface_refresh: SurfaceRefreshDiagnostics {
                invalidation: SurfaceInvalidation::Layout,
                timings: SurfaceRefreshTimings {
                    application_projection: Duration::from_micros(2),
                    runtime_projection: Duration::from_micros(3),
                    widget_state_sync: Duration::from_micros(5),
                    layout: Duration::from_micros(7),
                },
                identity: Default::default(),
            },
            surface_refresh_total: Duration::from_micros(23),
            surface_recovery: crate::runtime::NativeSurfaceRecoveryDiagnostics {
                lost: 37,
                outdated: 41,
                timeouts: 43,
                others: 47,
                completed_reconfigures: 53,
                zero_size_deferrals: 59,
                retry_requests: 61,
                timeout_retry_requests: 67,
                other_retry_requests: 71,
            },
        });

        assert_eq!(diagnostics.presentation.surface_invalidation, "layout");
        assert_eq!(
            diagnostics.window_identity.map(|identity| identity.get()),
            Some(7)
        );
        assert_eq!(diagnostics.frame_sequence, Some(41));
        assert_eq!(diagnostics.input_to_present_latency_us, Some(29_000));
        assert_eq!(diagnostics.surface_recovery.lost, 37);
        assert_eq!(diagnostics.surface_recovery.outdated, 41);
        assert_eq!(diagnostics.surface_recovery.timeouts, 43);
        assert_eq!(diagnostics.surface_recovery.others, 47);
        assert_eq!(diagnostics.surface_recovery.completed_reconfigures, 53);
        assert_eq!(diagnostics.surface_recovery.zero_size_deferrals, 59);
        assert_eq!(diagnostics.surface_recovery.retry_requests, 61);
        assert_eq!(diagnostics.surface_recovery.timeout_retry_requests, 67);
        assert_eq!(diagnostics.surface_recovery.other_retry_requests, 71);
        assert_eq!(diagnostics.scene.scene_encode_count, 7);
        assert_eq!(diagnostics.scene.scene_reuse_count, 11);
        assert_eq!(diagnostics.scene.scene_assembly_count, 13);
        assert_eq!(diagnostics.scene.scene_assembly_veto_count, 17);
        assert_eq!(diagnostics.scene.scene_mixed_assembly_count, 19);
        assert_eq!(diagnostics.scene.scene_assembly_fresh_count, 23);
        assert_eq!(diagnostics.scene.scene_assembly_reused_count, 29);
        assert_eq!(diagnostics.scene.scene_assembly_append_count, 31);
        assert_eq!(
            diagnostics.scene.scene_build_outcome,
            "retained_assembly_veto_fallback"
        );
        assert_eq!(
            diagnostics.timings.frame_work.refresh_surface,
            Duration::from_micros(23)
        );
        assert_eq!(
            diagnostics.timings.cpu_envelope_total(),
            Duration::from_micros(23)
        );
    }
}
