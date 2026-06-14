use super::super::*;
use crate::layout::ContainerPolicy;
use crate::runtime::{
    BusinessMessageSink, PlatformCompletion, PlatformRequest, PlatformResponse,
    PlatformServiceFallback, SurfaceNode, TaskPriority,
};
use std::sync::{Arc, Mutex};

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

#[derive(Default)]
pub(super) struct StreamingCommandBridge {
    pub(super) dispatched: Arc<Mutex<Vec<usize>>>,
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

impl RuntimeBridge<usize> for StreamingCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(message);
    }

    fn spawn_streaming_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce(BusinessMessageSink<usize>) + Send + 'static>,
    ) -> bool {
        let dispatched = Arc::clone(&self.dispatched);
        let sink = BusinessMessageSink::new(move |message| {
            dispatched
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(message);
            true
        });
        work(sink);
        true
    }
}
