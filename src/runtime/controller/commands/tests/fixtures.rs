use super::super::*;
use crate::layout::ContainerPolicy;
use crate::runtime::{
    PlatformCompletion, PlatformRequest, PlatformResponse, PlatformServiceFallback, SurfaceNode,
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct QueuedCommandBridge {
    pub(super) commands: Vec<Command<usize>>,
    pub(super) dispatched: Vec<usize>,
}

#[derive(Default)]
pub(super) struct PlatformCommandBridge {
    pub(super) dispatched: Vec<usize>,
    pub(super) requests: Vec<PlatformRequest>,
}

impl RuntimeBridge<usize> for PlatformCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn request_platform_service(
        &mut self,
        request: PlatformRequest,
        on_completed: PlatformCompletion<usize>,
    ) -> Result<(), PlatformServiceFallback<usize>> {
        self.requests.push(request);
        let message = on_completed(Ok(PlatformResponse::Canceled));
        self.reduce_message(message);
        Ok(())
    }
}

impl RuntimeBridge<usize> for QueuedCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<usize>>) {
        commands.append(&mut self.commands);
    }
}
