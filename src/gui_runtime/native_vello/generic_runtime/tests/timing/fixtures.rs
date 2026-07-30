use super::shared::*;
use crate::runtime::{
    NativeFrameDiagnostics, PaintPrimitive, RuntimeAnimationActivity, RuntimeAnimationHost,
    RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities, RuntimeQueueHost,
    RuntimeTransientOverlayHost, TransientOverlayContext,
};
use crate::widgets::TextWidget;
use std::cell::Cell;

#[derive(Default)]
pub(super) struct CountingProjectBridge {
    pub(super) project_count: usize,
}

impl RuntimeBridge<DemoMessage> for CountingProjectBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        self.project_count += 1;
        demo_surface(&DemoState::default())
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for CountingProjectBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

#[derive(Default)]
pub(super) struct CountingAnimationActivityBridge {
    pub(super) animation_activity_polls: usize,
}

impl RuntimeBridge<DemoMessage> for CountingAnimationActivityBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_animation()
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }
}

impl RuntimeAnimationHost for CountingAnimationActivityBridge {
    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.animation_activity_polls += 1;
        RuntimeAnimationActivity::idle()
    }
}

#[derive(Default)]
pub(super) struct NoTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

#[derive(Default)]
pub(super) struct ExactTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

#[derive(Default)]
pub(super) struct LargeExactBridge;

impl RuntimeBridge<DemoMessage> for LargeExactBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        let rows = (0..3_000_u64)
            .map(|index| {
                SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                    10_000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                )))
            })
            .collect();
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy {
                kind: crate::layout::ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            rows,
        )))
    }
}

impl RuntimeBridge<DemoMessage> for ExactTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            TextWidget::new(
                101,
                "Stable",
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            ),
            WidgetMessageMapper::none(),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_transient_overlays()
    }
}

impl RuntimeTransientOverlayHost for ExactTransientOverlayBridge {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

impl RuntimeBridge<DemoMessage> for NoTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for NoTransientOverlayBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

impl RuntimeTransientOverlayHost for NoTransientOverlayBridge {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

#[derive(Default)]
pub(super) struct OptInTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

impl RuntimeBridge<DemoMessage> for OptInTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_transient_overlays()
    }
}

impl RuntimeTransientOverlayHost for OptInTransientOverlayBridge {
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

pub(super) struct NoFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for NoFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }
}

#[derive(Default)]
pub(super) struct CountingFrameDiagnosticsBridge {
    pub(super) observer_checks: Cell<usize>,
}

impl RuntimeBridge<DemoMessage> for CountingFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        self.observer_checks
            .set(self.observer_checks.get().saturating_add(1));
        RuntimeHostCapabilities::new()
    }
}

pub(super) struct OptInFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for OptInFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new().with_frame_diagnostics()
    }
}

impl RuntimeFrameDiagnosticsHost for OptInFrameDiagnosticsBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}

#[derive(Default)]
pub(super) struct TestFrameMessageBridge {
    queued: bool,
}

impl RuntimeBridge<DemoMessage> for TestFrameMessageBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, DemoMessage> {
        RuntimeHostCapabilities::new()
            .with_animation()
            .with_queues()
            .with_frame_diagnostics()
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::request_repaint()
    }
}

impl RuntimeAnimationHost for TestFrameMessageBridge {
    fn needs_animation(&mut self) -> bool {
        true
    }

    fn queue_animation_frame(&mut self) -> bool {
        self.queued = true;
        true
    }
}

impl RuntimeQueueHost<DemoMessage> for TestFrameMessageBridge {
    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        if std::mem::take(&mut self.queued) {
            vec![DemoMessage::Increment]
        } else {
            Vec::new()
        }
    }
}

impl RuntimeFrameDiagnosticsHost for TestFrameMessageBridge {
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}
}
