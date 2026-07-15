use super::super::{RenderFrameProfile, RetainedSurfaceEncodeStats, gpu_surface};
use crate::gui_runtime::native_vello::TextLayoutProfileCounters;
use crate::gui_runtime::native_vello::generic_runtime::FrameWork;
use std::time::Duration;

pub(super) struct NativeFrameDiagnosticsParts {
    pub(super) stats: RetainedSurfaceEncodeStats,
    pub(super) text_stats: TextLayoutProfileCounters,
    pub(super) retained_policy: crate::runtime::RetainedSurfaceCachePolicy,
    pub(super) retained_entries: usize,
    pub(super) gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    pub(super) profile: RenderFrameProfile,
    pub(super) render_to_texture_elapsed: Duration,
    pub(super) since_last_present: Duration,
    pub(super) frame_work: FrameWork,
    pub(super) surface_refresh: crate::runtime::SurfaceRefreshDiagnostics,
    pub(super) surface_refresh_total: Duration,
}

pub(super) fn native_frame_diagnostics(
    parts: NativeFrameDiagnosticsParts,
) -> crate::runtime::NativeFrameDiagnostics {
    let surface_invalidation =
        surface_invalidation_name(parts.frame_work, parts.surface_refresh.invalidation);
    crate::runtime::NativeFrameDiagnostics {
        presentation: crate::runtime::NativeFramePresentationDiagnostics {
            frame_work_kind: parts.frame_work.kind(),
            frame_work_reason: parts.frame_work.reason().name(),
            surface_invalidation,
            paint_only: parts.frame_work.is_paint_only(),
            scene_rebuild: parts.frame_work.needs_scene_rebuild(),
        },
        scene: crate::runtime::NativeSceneDiagnostics {
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
            },
            signal: crate::runtime::NativeGpuSurfaceSignalDiagnostics {
                summary_builds: parts.gpu_surface_stats.signal.summary_builds,
                summary_cache_hits: parts.gpu_surface_stats.signal.summary_cache_hits,
                body_renders: parts.gpu_surface_stats.signal.body_renders,
                body_cache_hits: parts.gpu_surface_stats.signal.body_cache_hits,
            },
            composite: crate::runtime::NativeGpuSurfaceCompositeDiagnostics {
                binding_rebuilds: parts.gpu_surface_stats.composite.binding_rebuilds,
                binding_cache_hits: parts.gpu_surface_stats.composite.binding_cache_hits,
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
            text_stats: Default::default(),
            retained_policy: RetainedSurfaceCachePolicy::default(),
            retained_entries: 0,
            gpu_surface_stats: Default::default(),
            profile: RenderFrameProfile::default(),
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
            },
            surface_refresh_total: Duration::from_micros(23),
        });

        assert_eq!(diagnostics.presentation.surface_invalidation, "layout");
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
