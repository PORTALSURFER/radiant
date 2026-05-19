use super::super::*;

pub(super) fn native_frame_diagnostics(
    stats: RetainedSurfaceEncodeStats,
    retained_policy: crate::runtime::RetainedSurfaceCachePolicy,
    retained_entries: usize,
    gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    profile: RenderFrameProfile,
    render_to_texture_elapsed: Duration,
    since_last_present: Duration,
) -> crate::runtime::NativeFrameDiagnostics {
    crate::runtime::NativeFrameDiagnostics {
        scene: crate::runtime::NativeSceneDiagnostics {
            paint_plan_primitives: stats.paint_plan_primitives,
            clip_layer_count: stats.clip_layer_count,
            text_primitive_count: stats.text_primitive_count,
            text_input_count: stats.text_input_count,
            image_count: stats.image_count,
            svg_document_count: stats.svg_document_count,
            gpu_surface_count: stats.gpu_surface_count,
            custom_surface_count: stats.custom_surface_count,
            custom_surface_fallback_count: stats.custom_surface_fallback_count,
            text_run_count: stats.text_run_count,
        },
        retained_surfaces: crate::runtime::NativeRetainedSurfaceDiagnostics {
            cache_capacity: retained_policy.max_frames,
            cache_entries: retained_entries,
            bridge_calls: stats.bridge_calls,
            cache_hits: stats.cache_hits,
            miss_count: stats.retained_surface_miss_count,
            retained_frame_primitive_count: stats.retained_frame_primitive_count,
            retained_frame_text_run_count: stats.retained_frame_text_run_count,
        },
        gpu_surfaces: crate::runtime::NativeGpuSurfaceDiagnostics {
            atlas_texture_uploads: gpu_surface_stats.atlas_texture_uploads,
            atlas_texture_cache_hits: gpu_surface_stats.atlas_texture_cache_hits,
            signal_summary_builds: gpu_surface_stats.signal_summary_builds,
            signal_summary_cache_hits: gpu_surface_stats.signal_summary_cache_hits,
            signal_body_renders: gpu_surface_stats.signal_body_renders,
            signal_body_cache_hits: gpu_surface_stats.signal_body_cache_hits,
            composite_binding_rebuilds: gpu_surface_stats.composite_binding_rebuilds,
            composite_binding_cache_hits: gpu_surface_stats.composite_binding_cache_hits,
        },
        timings: crate::runtime::NativeFrameTimingDiagnostics {
            coalesced_wheel_route: profile.coalesced_wheel_route,
            refresh_surface: profile.refresh_surface,
            paint_plan: profile.paint_plan,
            render_to_texture: render_to_texture_elapsed,
            full_screen_blit: profile.full_screen_blit,
            composited_base_refresh: profile.composited_base_refresh,
            composited_base_cache_hit: profile.composited_base_cache_hit,
            transient_overlay_paint: profile.transient_overlay_paint,
            transient_overlay_primitives: profile.transient_overlay_primitives,
            submit_present: profile.submit_present,
            since_last_present,
        },
    }
}
