use super::*;
use radiant::runtime::{
    NativeFrameDiagnostics, NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeGpuTimingStatus, NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics,
    NativeTextDiagnostics, RuntimeBridge,
};

#[test]
fn runtime_bridge_can_observe_structured_frame_diagnostics() {
    let mut bridge = DiagnosticBridge::default();
    let diagnostics = NativeFrameDiagnostics {
        scene: NativeSceneDiagnostics {
            paint_plan_primitives: 12,
            custom_surface_count: 2,
            ..NativeSceneDiagnostics::default()
        },
        retained_surfaces: NativeRetainedSurfaceDiagnostics {
            cache_capacity: 8,
            cache_entries: 3,
            bridge_calls: 1,
            cache_hits: 2,
            ..NativeRetainedSurfaceDiagnostics::default()
        },
        gpu_surfaces: NativeGpuSurfaceDiagnostics {
            signal_summary_cache_hits: 4,
            custom_shader_surfaces_rendered: 2,
            custom_shader_pipeline_rebuilds: 1,
            custom_shader_binding_rebuilds: 1,
            custom_shader_binding_cache_hits: 3,
            custom_shader_surfaces_failed: 1,
            custom_shader_shader_module_failures: 1,
            custom_shader_pipeline_failures: 1,
            custom_shader_binding_failures: 1,
            unsupported_custom_shader_surfaces: 1,
            unsupported_custom_shader_vertices: 6,
            unsupported_custom_shader_source_bytes: 64,
            unsupported_custom_shader_uniform_bytes: 16,
            unsupported_custom_shader_storage_bytes: 128,
            ..NativeGpuSurfaceDiagnostics::default()
        },
        text: NativeTextDiagnostics {
            layout_cache_hits: 6,
            atom_cache_misses: 2,
            unsupported_shaping_runs: 1,
            unsupported_shaping_scalars: 4,
            fallback_glyphs: 3,
            missing_glyphs: 1,
            ..NativeTextDiagnostics::default()
        },
        timings: NativeFrameTimingDiagnostics {
            gpu_timing_status: NativeGpuTimingStatus::CpuEnvelopeOnly,
            transient_overlay_primitives: 5,
            ..NativeFrameTimingDiagnostics::default()
        },
    };

    bridge.observe_frame_diagnostics(diagnostics);

    assert_eq!(bridge.last, Some(diagnostics));
    assert!(diagnostics.text.has_shaping_limits());
    assert!(diagnostics.text.has_font_coverage_gaps());
    assert!(diagnostics.text.has_text_quality_warnings());
    assert!(!NativeTextDiagnostics::default().has_text_quality_warnings());
    assert_eq!(
        diagnostics.timings.gpu_timing_status,
        NativeGpuTimingStatus::CpuEnvelopeOnly
    );
}

#[derive(Default)]
struct DiagnosticBridge {
    last: Option<NativeFrameDiagnostics>,
}

impl RuntimeBridge<()> for DiagnosticBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        self.last = Some(diagnostics);
    }
}
