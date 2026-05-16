use super::*;
use radiant::runtime::{
    NativeFrameDiagnostics, NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics,
    NativeRetainedSurfaceDiagnostics, NativeSceneDiagnostics, RuntimeBridge,
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
            ..NativeGpuSurfaceDiagnostics::default()
        },
        timings: NativeFrameTimingDiagnostics {
            transient_overlay_primitives: 5,
            ..NativeFrameTimingDiagnostics::default()
        },
    };

    bridge.observe_frame_diagnostics(diagnostics);

    assert_eq!(bridge.last, Some(diagnostics));
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
