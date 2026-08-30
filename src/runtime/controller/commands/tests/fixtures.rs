use super::super::*;
use crate::layout::{Constraints, ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams};
#[allow(deprecated)]
use crate::runtime::{
    PlatformCompletion, PlatformRequest, PlatformResponse, PlatformServiceFallback,
    RuntimeHostCapabilities, RuntimeInputHost, RuntimePlatformHost, RuntimeQueueHost, SurfaceChild,
    SurfaceNode, WidgetMessageMapper,
};
use crate::widgets::{InteractiveRowWidget, WidgetSizing};
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

#[derive(Default)]
pub(super) struct DeferredFocusBridge {
    pub(super) show_focus_target: bool,
    pub(super) project_count: usize,
}

#[derive(Default)]
pub(super) struct OrderedFocusBridge {
    pub(super) command: Option<Command<usize>>,
    pub(super) project_count: usize,
}

#[derive(Default)]
pub(super) struct DeferredPlatformFallbackBridge {
    pub(super) show_fallback_target: bool,
    pub(super) project_count: usize,
}

#[derive(Default)]
pub(super) struct DeferredScrollBridge {
    pub(super) project_count: usize,
}

#[derive(Default)]
pub(super) struct DeferredScrollFocusBridge {
    pub(super) show_focus_target: bool,
    pub(super) project_count: usize,
    pub(super) scroll_updates: usize,
    pub(super) last_scroll_update: Option<crate::runtime::ScrollUpdate>,
}

impl RuntimeBridge<usize> for PlatformCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_platform()
    }
}

#[allow(deprecated)]
impl RuntimePlatformHost<usize> for PlatformCommandBridge {
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
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_queues()
    }
}

impl RuntimeQueueHost<usize> for QueuedCommandBridge {
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<usize>>) {
        commands.append(&mut self.commands);
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
        crate::runtime::test_arc_surface(UiSurface::new(node))
    }

    fn update(&mut self, message: usize) -> Command<usize> {
        match message {
            1 => {
                self.show_focus_target = true;
                Command::focus(42)
            }
            2 => {
                self.show_focus_target = true;
                Command::batch([Command::request_paint_only(), Command::focus(42)])
            }
            _ => Command::none(),
        }
    }
}

impl RuntimeBridge<usize> for OrderedFocusBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn update(&mut self, _message: usize) -> Command<usize> {
        self.command.take().unwrap_or_default()
    }
}

impl RuntimeBridge<usize> for DeferredPlatformFallbackBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        self.project_count += 1;
        let mut children = vec![fixed_child(
            22.0,
            SurfaceNode::widget(
                InteractiveRowWidget::new(42, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
                WidgetMessageMapper::none(),
            ),
        )];
        if self.show_fallback_target {
            children.push(fixed_child(
                22.0,
                SurfaceNode::widget(
                    InteractiveRowWidget::new(43, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
                    WidgetMessageMapper::none(),
                ),
            ));
        }
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::column(20, 0.0, children)))
    }

    fn update(&mut self, message: usize) -> Command<usize> {
        if message == 1 {
            self.show_fallback_target = true;
        }
        Command::none()
    }
}

impl RuntimeBridge<usize> for DeferredScrollBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        self.project_count += 1;
        crate::runtime::test_arc_surface(UiSurface::new(scroll_test_surface(
            SurfaceNode::container(42, ContainerPolicy::default(), Vec::new()),
        )))
    }
}

impl RuntimeBridge<usize> for DeferredScrollFocusBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        self.project_count += 1;
        let target = if self.show_focus_target {
            SurfaceNode::widget(
                InteractiveRowWidget::new(42, WidgetSizing::fixed(Vector2::new(120.0, 22.0))),
                WidgetMessageMapper::none(),
            )
        } else {
            SurfaceNode::container(42, ContainerPolicy::default(), Vec::new())
        };
        crate::runtime::test_arc_surface(UiSurface::new(scroll_test_surface(target)))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<usize> for DeferredScrollFocusBridge {
    fn scroll_updated(&mut self, update: crate::runtime::ScrollUpdate) -> Option<Command<usize>> {
        self.scroll_updates += 1;
        self.last_scroll_update = Some(update);
        self.show_focus_target = true;
        Some(Command::focus(42))
    }
}

fn scroll_test_surface<Message>(middle: SurfaceNode<Message>) -> SurfaceNode<Message> {
    SurfaceNode::scroll_area(
        10,
        SurfaceNode::column(
            11,
            0.0,
            vec![
                fixed_child(
                    80.0,
                    SurfaceNode::widget(
                        InteractiveRowWidget::new(
                            30,
                            WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                ),
                fixed_child(22.0, middle),
                fixed_child(
                    80.0,
                    SurfaceNode::widget(
                        InteractiveRowWidget::new(
                            31,
                            WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                ),
            ],
        ),
    )
}

fn fixed_child<Message>(height: f32, child: SurfaceNode<Message>) -> SurfaceChild<Message> {
    SurfaceChild::new(
        SlotParams {
            size_main: SizeModeMain::Fixed(height),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::unconstrained(),
            margin: Default::default(),
            align_cross_override: None,
            allow_fixed_compress: false,
        },
        child,
    )
}
