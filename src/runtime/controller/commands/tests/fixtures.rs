use super::super::*;
use crate::layout::ContainerPolicy;
use crate::runtime::{
    BusinessMessageSink, PlatformCompletion, PlatformRequest, PlatformResponse,
    PlatformServiceFallback, SurfaceNode, TaskPriority, WidgetMessageMapper,
};
use crate::widgets::{InteractiveRowWidget, WidgetSizing};
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

#[derive(Default)]
pub(super) struct DeferredFocusBridge {
    pub(super) show_focus_target: bool,
    pub(super) project_count: usize,
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

impl RuntimeBridge<usize> for DeferredFocusBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        self.project_count += 1;
        let node = if self.show_focus_target {
            SurfaceNode::widget(
                InteractiveRowWidget::new(42, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
                WidgetMessageMapper::none(),
            )
        } else {
            SurfaceNode::container(1, ContainerPolicy::default(), Vec::new())
        };
        Arc::new(UiSurface::new(node))
    }

    fn update(&mut self, message: usize) -> Command<usize> {
        if message == 1 {
            self.show_focus_target = true;
            Command::focus(42)
        } else {
            Command::none()
        }
    }
}
