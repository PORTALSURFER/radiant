use super::super::*;

pub(super) struct NativeFrameDiagnosticsParts {
    pub(super) stats: RetainedSurfaceEncodeStats,
    pub(super) text_stats: TextLayoutProfileCounters,
    pub(super) retained_policy: crate::runtime::RetainedSurfaceCachePolicy,
    pub(super) retained_entries: usize,
    pub(super) gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    pub(super) profile: RenderFrameProfile,
    pub(super) render_to_texture_elapsed: Duration,
    pub(super) since_last_present: Duration,
}

pub(super) fn native_frame_diagnostics(
    parts: NativeFrameDiagnosticsParts,
) -> crate::runtime::NativeFrameDiagnostics {
    crate::runtime::NativeFrameDiagnostics {
        scene: crate::runtime::NativeSceneDiagnostics {
            paint_plan_primitives: parts.stats.paint_plan_primitives,
            clip_layer_count: parts.stats.clip_layer_count,
            text_primitive_count: parts.stats.text_primitive_count,
            text_input_count: parts.stats.text_input_count,
            image_count: parts.stats.image_count,
            svg_document_count: parts.stats.svg_document_count,
            gpu_surface_count: parts.stats.gpu_surface_count,
            custom_surface_count: parts.stats.custom_surface_count,
            custom_surface_fallback_count: parts.stats.custom_surface_fallback_count,
            text_run_count: parts.stats.text_run_count,
        },
        text: crate::runtime::NativeTextDiagnostics {
            layout_cache_hits: parts.text_stats.layout_hits,
            layout_cache_misses: parts.text_stats.layout_misses,
            layout_cache_evictions: parts.text_stats.layout_evictions,
            atom_cache_hits: parts.text_stats.atom_hits,
            atom_cache_misses: parts.text_stats.atom_misses,
            atom_cache_evictions: parts.text_stats.atom_evictions,
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
            atlas_texture_uploads: parts.gpu_surface_stats.atlas_texture_uploads,
            atlas_texture_cache_hits: parts.gpu_surface_stats.atlas_texture_cache_hits,
            signal_summary_builds: parts.gpu_surface_stats.signal_summary_builds,
            signal_summary_cache_hits: parts.gpu_surface_stats.signal_summary_cache_hits,
            signal_body_renders: parts.gpu_surface_stats.signal_body_renders,
            signal_body_cache_hits: parts.gpu_surface_stats.signal_body_cache_hits,
            composite_binding_rebuilds: parts.gpu_surface_stats.composite_binding_rebuilds,
            composite_binding_cache_hits: parts.gpu_surface_stats.composite_binding_cache_hits,
            unsupported_custom_shader_surfaces: parts
                .gpu_surface_stats
                .unsupported_custom_shader_surfaces,
        },
        timings: crate::runtime::NativeFrameTimingDiagnostics {
            coalesced_wheel_route: parts.profile.coalesced_wheel_route,
            refresh_surface: parts.profile.refresh_surface,
            paint_plan: parts.profile.paint_plan,
            render_to_texture: parts.render_to_texture_elapsed,
            full_screen_blit: parts.profile.full_screen_blit,
            composited_base_refresh: parts.profile.composited_base_refresh,
            composited_base_cache_hit: parts.profile.composited_base_cache_hit,
            transient_overlay_paint: parts.profile.transient_overlay_paint,
            transient_overlay_primitives: parts.profile.transient_overlay_primitives,
            submit_present: parts.profile.submit_present,
            since_last_present: parts.since_last_present,
        },
    }
}
