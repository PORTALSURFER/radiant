use super::shared::*;
use crate::runtime::{PaintPrimitive, RuntimeAnimationActivity, TransientOverlayContext};

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
}

#[derive(Default)]
pub(super) struct CountingAnimationActivityBridge {
    pub(super) animation_activity_polls: usize,
}

impl RuntimeBridge<DemoMessage> for CountingAnimationActivityBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.animation_activity_polls += 1;
        RuntimeAnimationActivity::idle()
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }
}

#[derive(Default)]
pub(super) struct NoTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

impl RuntimeBridge<DemoMessage> for NoTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn has_transient_overlay_painter(&self) -> bool {
        false
    }

    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

#[derive(Default)]
pub(super) struct DefaultTransientOverlayBridge {
    pub(super) paint_calls: usize,
}

impl RuntimeBridge<DemoMessage> for DefaultTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

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

    fn has_frame_diagnostics_observer(&self) -> bool {
        false
    }
}

pub(super) struct DefaultFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for DefaultFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }
}

#[derive(Default)]
pub(super) struct TestFrameMessageBridge {
    queued: bool,
}

impl RuntimeBridge<DemoMessage> for TestFrameMessageBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn needs_animation(&mut self) -> bool {
        true
    }

    fn queue_animation_frame(&mut self) -> bool {
        self.queued = true;
        true
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        if std::mem::take(&mut self.queued) {
            vec![DemoMessage::Increment]
        } else {
            Vec::new()
        }
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::request_repaint()
    }
}
