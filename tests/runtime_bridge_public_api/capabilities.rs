use super::*;
use radiant::runtime::{
    NativeFrameDiagnostics, PaintPrimitive, RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities,
    RuntimeTransientOverlayHost, SurfacePaintPlan, TransientOverlayContext,
};
use std::time::Duration;

#[derive(Default)]
struct MinimalHost {
    updates: usize,
}

impl RuntimeBridge<()> for MinimalHost {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn update(&mut self, _message: ()) -> radiant::runtime::Command<()> {
        self.updates += 1;
        radiant::runtime::Command::none()
    }
}

#[test]
fn minimal_bridge_has_no_diagnostics_or_transient_overlay_capability() {
    let mut runtime = SurfaceRuntime::new(MinimalHost::default(), Vector2::new(120.0, 40.0));

    assert!(!runtime.host_capabilities().has_frame_diagnostics());
    assert!(!runtime.host_capabilities().has_transient_overlay());
    runtime.dispatch_message(());

    assert_eq!(runtime.bridge().updates, 1);
}

#[derive(Default)]
struct AdvancedHost {
    overlay_calls: usize,
    diagnostics_calls: usize,
}

impl RuntimeBridge<()> for AdvancedHost {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        Arc::new(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new()
            .with_transient_overlays()
            .with_frame_diagnostics()
    }
}

impl RuntimeTransientOverlayHost for AdvancedHost {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.overlay_calls += 1;
    }
}

impl RuntimeFrameDiagnosticsHost for AdvancedHost {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {
        self.diagnostics_calls += 1;
    }
}

#[test]
fn advanced_bridge_runs_only_explicitly_registered_callbacks() {
    let mut bridge = AdvancedHost::default();
    let capabilities = bridge.host_capabilities();
    let plan = SurfacePaintPlan::empty(&radiant::theme::ThemeTokens::default());
    let mut primitives = Vec::new();

    assert!(capabilities.has_transient_overlay());
    assert!(capabilities.has_frame_diagnostics());
    assert!(capabilities.paint_transient_overlay(
        &mut bridge,
        TransientOverlayContext::new(&plan, Vector2::new(120.0, 40.0), Duration::ZERO),
        &mut primitives,
    ));
    assert!(
        capabilities.observe_frame_diagnostics(&mut bridge, NativeFrameDiagnostics::default(),)
    );

    assert_eq!(bridge.overlay_calls, 1);
    assert_eq!(bridge.diagnostics_calls, 1);
}
