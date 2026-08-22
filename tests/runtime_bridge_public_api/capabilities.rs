use super::*;
use radiant::runtime::{
    FrameGpuTimingOutcome, FrameGpuTimingSample, FrameGpuTimingUnavailableReason,
    NativeFrameDiagnostics, PaintPrimitive, RuntimeFrameDiagnosticsHost, RuntimeFrameGpuTimingHost,
    RuntimeHostCapabilities, RuntimeTransientOverlayHost, SurfacePaintPlan,
    TransientOverlayContext,
};
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct MinimalHost {
    updates: usize,
}

impl RuntimeBridge<()> for MinimalHost {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::arc_surface(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
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
        crate::arc_surface(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
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

#[derive(Default)]
struct GpuTimingHost {
    samples: Vec<FrameGpuTimingSample>,
}

impl RuntimeBridge<()> for GpuTimingHost {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::arc_surface(UiSurface::new(SurfaceNode::column(1, 0.0, Vec::new())))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new()
    }
}

impl RuntimeFrameGpuTimingHost for GpuTimingHost {
    fn observe_frame_gpu_timing(&mut self, sample: FrameGpuTimingSample) {
        self.samples.push(sample);
    }
}

#[test]
fn gpu_timing_capability_vetoes_without_opt_in_and_delivers_when_enabled() {
    let sample = FrameGpuTimingSample::new(
        7,
        11,
        FrameGpuTimingOutcome::unavailable(FrameGpuTimingUnavailableReason::Unsupported),
    );
    let mut host = GpuTimingHost::default();
    let disabled = RuntimeHostCapabilities::<GpuTimingHost, ()>::new();

    assert!(!disabled.has_frame_gpu_timing());
    assert!(!disabled.observe_frame_gpu_timing(&mut host, sample));
    assert!(host.samples.is_empty());

    let enabled = RuntimeHostCapabilities::<GpuTimingHost, ()>::new().with_frame_gpu_timing();
    assert!(enabled.has_frame_gpu_timing());
    assert!(enabled.observe_frame_gpu_timing(&mut host, sample));
    assert_eq!(host.samples, vec![sample]);
}

#[test]
fn stateful_application_registers_gpu_timing_callback() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let observed_by_callback = Rc::clone(&observed);
    let mut bridge = radiant::prelude::app(())
        .view(|_| radiant::prelude::text("GPU timing"))
        .on_frame_gpu_timing(move |_, sample| {
            observed_by_callback.borrow_mut().push(sample);
        })
        .into_bridge();
    let capabilities = bridge.host_capabilities();
    let sample = FrameGpuTimingSample::new(
        3,
        19,
        FrameGpuTimingOutcome::available(Duration::from_nanos(37)),
    );

    assert!(capabilities.has_frame_gpu_timing());
    assert!(!capabilities.has_frame_profile());
    assert!(capabilities.observe_frame_gpu_timing(&mut bridge, sample));
    assert_eq!(*observed.borrow(), vec![sample]);
}
