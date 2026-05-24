use super::*;
use radiant::runtime::{
    NativeCompositedBaseTiming, NativeFrameDiagnostics, NativeFrameTimingDiagnostics,
    NativeFrameWorkTimings, NativeGpuSurfaceCustomShaderDiagnostics,
    NativeGpuSurfaceCustomShaderFailureDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeGpuSurfaceSignalDiagnostics, NativeGpuSurfaceUnsupportedCustomShaderDiagnostics,
    NativeGpuTimingStatus, NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics,
    NativeSceneSurfaceDiagnostics, NativeSceneTraversalDiagnostics, NativeTextCacheCounters,
    NativeTextCacheDiagnostics, NativeTextDiagnostics, NativeTextQualityDiagnostics,
    NativeTextQualityStatus, NativeTransientOverlayTiming, RuntimeBridge,
};
use std::time::Duration;

#[test]
fn runtime_bridge_can_observe_structured_frame_diagnostics() {
    let mut bridge = DiagnosticBridge::default();
    let diagnostics = NativeFrameDiagnostics {
        scene: NativeSceneDiagnostics {
            traversal: NativeSceneTraversalDiagnostics {
                paint_plan_primitives: 12,
                ..NativeSceneTraversalDiagnostics::default()
            },
            surfaces: NativeSceneSurfaceDiagnostics {
                custom_surface_count: 2,
                ..NativeSceneSurfaceDiagnostics::default()
            },
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
            signal: NativeGpuSurfaceSignalDiagnostics {
                summary_cache_hits: 4,
                ..NativeGpuSurfaceSignalDiagnostics::default()
            },
            custom_shader: NativeGpuSurfaceCustomShaderDiagnostics {
                surfaces_rendered: 2,
                pipeline_rebuilds: 1,
                binding_rebuilds: 1,
                binding_cache_hits: 3,
                failures: NativeGpuSurfaceCustomShaderFailureDiagnostics {
                    surfaces_failed: 1,
                    shader_module_failures: 1,
                    pipeline_failures: 1,
                    binding_failures: 1,
                },
                unsupported: NativeGpuSurfaceUnsupportedCustomShaderDiagnostics {
                    surfaces: 1,
                    vertices: 6,
                    source_bytes: 64,
                    uniform_bytes: 16,
                    storage_bytes: 128,
                },
            },
            ..NativeGpuSurfaceDiagnostics::default()
        },
        text: NativeTextDiagnostics {
            cache: NativeTextCacheDiagnostics {
                layout: NativeTextCacheCounters {
                    hits: 6,
                    ..NativeTextCacheCounters::default()
                },
                atom: NativeTextCacheCounters {
                    misses: 2,
                    ..NativeTextCacheCounters::default()
                },
            },
            quality: NativeTextQualityDiagnostics {
                unsupported_shaping_runs: 1,
                unsupported_shaping_scalars: 4,
                fallback_glyphs: 3,
                missing_glyphs: 1,
            },
        },
        timings: NativeFrameTimingDiagnostics {
            gpu_timing_status: NativeGpuTimingStatus::CpuEnvelopeOnly,
            frame_work: NativeFrameWorkTimings {
                refresh_surface: Duration::from_micros(7),
                paint_plan: Duration::from_micros(11),
                render_to_texture: Duration::from_micros(13),
                full_screen_blit: Duration::from_micros(17),
                ..NativeFrameWorkTimings::default()
            },
            composited_base: NativeCompositedBaseTiming::default(),
            transient_overlay: NativeTransientOverlayTiming {
                primitives: 5,
                ..NativeTransientOverlayTiming::default()
            },
            submit_present: Duration::from_micros(19),
            since_last_present: Duration::from_micros(1000),
        },
    };

    bridge.observe_frame_diagnostics(diagnostics);

    assert_eq!(bridge.last, Some(diagnostics));
    assert!(diagnostics.text.has_shaping_limits());
    assert!(diagnostics.text.has_font_coverage_gaps());
    assert!(diagnostics.text.has_text_quality_warnings());
    assert_eq!(
        diagnostics.text.quality_status(),
        NativeTextQualityStatus::ShapingAndFontCoverageLimited
    );
    assert_eq!(
        NativeTextDiagnostics {
            quality: NativeTextQualityDiagnostics {
                unsupported_shaping_runs: 1,
                ..NativeTextQualityDiagnostics::default()
            },
            ..NativeTextDiagnostics::default()
        }
        .quality_status(),
        NativeTextQualityStatus::ShapingLimited
    );
    assert_eq!(
        NativeTextDiagnostics {
            quality: NativeTextQualityDiagnostics {
                missing_glyphs: 1,
                ..NativeTextQualityDiagnostics::default()
            },
            ..NativeTextDiagnostics::default()
        }
        .quality_status(),
        NativeTextQualityStatus::FontCoverageLimited
    );
    assert!(!NativeTextDiagnostics::default().has_text_quality_warnings());
    assert_eq!(
        NativeTextDiagnostics::default().quality_status(),
        NativeTextQualityStatus::Clean
    );
    assert_eq!(
        diagnostics.timings.gpu_timing_status,
        NativeGpuTimingStatus::CpuEnvelopeOnly
    );
    assert_eq!(
        diagnostics.timings.cpu_envelope_total(),
        Duration::from_micros(67)
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
