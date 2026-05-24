use crate::runtime::{Command, RuntimeBridge, UiSurface};
use std::sync::Arc;

pub(super) struct AuxiliarySurfaceBridge<Message> {
    pub(super) surface: Arc<UiSurface<Message>>,
    outbox: Vec<Message>,
}

impl<Message> AuxiliarySurfaceBridge<Message> {
    pub(super) fn new(surface: Arc<UiSurface<Message>>) -> Self {
        Self {
            surface,
            outbox: Vec::new(),
        }
    }

    pub(super) fn take_messages(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.outbox)
    }
}

impl<Message> RuntimeBridge<Message> for AuxiliarySurfaceBridge<Message> {
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::clone(&self.surface)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.outbox.push(message);
        Command::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layout::NodeId, runtime::SurfaceNode};

    fn empty_surface<Message>() -> Arc<UiSurface<Message>> {
        Arc::new(UiSurface::new(SurfaceNode::column(
            NodeId::from(1_u64),
            0.0,
            Vec::new(),
        )))
    }

    #[test]
    fn auxiliary_bridge_queues_surface_messages_until_drained() {
        let mut bridge = AuxiliarySurfaceBridge::new(empty_surface());

        let _ = bridge.update("open");
        let _ = bridge.update("close");

        assert_eq!(bridge.take_messages(), vec!["open", "close"]);
        assert!(bridge.take_messages().is_empty());
    }
}
