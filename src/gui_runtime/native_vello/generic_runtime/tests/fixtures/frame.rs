use super::{super::*, demo::*};

pub(in super::super) struct AnimatingBridge;

pub(in super::super) struct PaintOnlyFrameBridge {
    pub(in super::super) pending_frame: bool,
}

impl Default for PaintOnlyFrameBridge {
    fn default() -> Self {
        Self {
            pending_frame: true,
        }
    }
}

impl RuntimeBridge<DemoMessage> for AnimatingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn needs_animation(&mut self) -> bool {
        true
    }
}

impl RuntimeBridge<DemoMessage> for PaintOnlyFrameBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::request_paint_only()
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        if std::mem::take(&mut self.pending_frame) {
            vec![DemoMessage::Increment]
        } else {
            Vec::new()
        }
    }
}
