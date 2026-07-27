use super::{
    super::*,
    fixtures::{PlatformCommandBridge, QueuedCommandBridge},
};
use crate::layout::ContainerPolicy;
use crate::runtime::{
    DragPreview, DragRequest, FileDialogRequest, PlatformRequest, PlatformResponse,
    PlatformResultDelivery, RuntimeBridge, RuntimeHostCapabilities, RuntimeLifecycleHost,
    RuntimePlatformResultHost, RuntimePlatformResultSink, RuntimeQueueDelivery, RuntimeQueueHost,
    RuntimeQueueItem, SurfaceNode,
};
use std::sync::Arc;

#[derive(Default)]
struct SynchronousResultBridge {
    dispatched: Vec<usize>,
}

#[derive(Default)]
struct DroppingResultBridge;

#[derive(Default)]
struct RetainingResultBridge {
    sinks: Vec<RuntimePlatformResultSink>,
}

#[derive(Default)]
struct LifecycleResultBridge {
    sinks: Vec<RuntimePlatformResultSink>,
    captures: std::rc::Weak<std::cell::RefCell<usize>>,
    observed_capture_count: Option<usize>,
}

#[derive(Default)]
struct ResultQueueBridge {
    commands: Vec<crate::runtime::Command<usize>>,
    items: Vec<RuntimeQueueItem<usize>>,
    dispatched: Vec<usize>,
    sinks: Vec<RuntimePlatformResultSink>,
}

impl RuntimeBridge<usize> for ResultQueueBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<usize>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn update(&mut self, message: usize) -> crate::runtime::Command<usize> {
        if message == 1 {
            self.items
                .push(RuntimeQueueItem::Delivery(RuntimeQueueDelivery::new(
                    crate::runtime::PlatformResultDelivery::Completed {
                        identity: crate::runtime::PlatformCompletionIdentity { id: 1, epoch: 1 },
                        result: Ok(PlatformResponse::Completed),
                    },
                )));
            crate::runtime::Command::platform_request(PlatformRequest::ReadText, |_| 2)
        } else {
            self.reduce_message(message);
            crate::runtime::Command::none()
        }
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new()
            .with_queues()
            .with_platform_results()
    }
}

impl RuntimePlatformResultHost for ResultQueueBridge {
    fn request_platform_result(
        &mut self,
        _request: PlatformRequest,
        sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        self.sinks.push(sink);
        Ok(())
    }
}

impl RuntimeQueueHost<usize> for ResultQueueBridge {
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<crate::runtime::Command<usize>>) {
        commands.append(&mut self.commands);
    }

    fn drain_runtime_queue_item_batch_into(
        &mut self,
        items: &mut Vec<RuntimeQueueItem<usize>>,
        _max_items: usize,
    ) -> bool {
        items.append(&mut self.items);
        false
    }
}

impl RuntimeBridge<usize> for DroppingResultBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<usize>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_platform_results()
    }
}

impl RuntimePlatformResultHost for DroppingResultBridge {
    fn request_platform_result(
        &mut self,
        _request: PlatformRequest,
        _sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        Ok(())
    }
}

impl RuntimeBridge<usize> for RetainingResultBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<usize>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_platform_results()
    }
}

impl RuntimePlatformResultHost for RetainingResultBridge {
    fn request_platform_result(
        &mut self,
        _request: PlatformRequest,
        sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        self.sinks.push(sink);
        Ok(())
    }
}

impl RuntimeBridge<usize> for LifecycleResultBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<usize>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new()
            .with_platform_results()
            .with_lifecycle()
    }
}

impl RuntimePlatformResultHost for LifecycleResultBridge {
    fn request_platform_result(
        &mut self,
        _request: PlatformRequest,
        sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        self.sinks.push(sink);
        Ok(())
    }
}

impl RuntimeLifecycleHost for LifecycleResultBridge {
    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.observed_capture_count = Some(self.captures.strong_count());
        None
    }
}

impl RuntimeBridge<usize> for SynchronousResultBridge {
    fn project_surface(&mut self) -> Arc<crate::runtime::UiSurface<usize>> {
        crate::runtime::test_arc_surface(crate::runtime::UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.dispatched.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_platform_results()
    }
}

impl RuntimePlatformResultHost for SynchronousResultBridge {
    fn request_platform_result(
        &mut self,
        _request: PlatformRequest,
        sink: RuntimePlatformResultSink,
    ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
        sink.send(Ok(PlatformResponse::Canceled));
        Ok(())
    }
}
use std::path::PathBuf;

#[test]
fn platform_request_dispatches_through_bridge_completion() {
    let bridge = SynchronousResultBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = PlatformRequest::PickFolder(FileDialogRequest::new().title("Choose library"));

    let outcome = runtime.execute_command(Command::platform_request(request.clone(), |result| {
        usize::from(result.is_ok())
    }));

    assert_eq!(outcome.messages_dispatched, 0);
    assert!(runtime.bridge().dispatched.is_empty());
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}

#[test]
fn synchronous_legacy_platform_host_is_deferred_as_rejection() {
    let mut runtime =
        SurfaceRuntime::new(PlatformCommandBridge::default(), Vector2::new(100.0, 100.0));
    let outcome = runtime.execute_command(Command::platform_request(
        PlatformRequest::ReadText,
        |result| usize::from(result.is_err()),
    ));
    assert_eq!(outcome.messages_dispatched, 0);
    assert!(runtime.bridge().requests.is_empty());
    assert!(runtime.bridge().dispatched.is_empty());
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
}

#[test]
fn legacy_platform_request_after_exit_releases_mapper_without_delivery() {
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let mut runtime =
        SurfaceRuntime::new(PlatformCommandBridge::default(), Vector2::new(100.0, 100.0));
    assert!(runtime.execute_command(Command::Exit).exit_requested);
    let mapper_captures = std::rc::Rc::clone(&captures);
    assert_eq!(
        runtime
            .execute_command(Command::platform_request(
                PlatformRequest::ReadText,
                move |_| {
                    *mapper_captures.borrow_mut() += 1;
                    1
                },
            ))
            .messages_dispatched,
        0
    );
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    assert!(runtime.bridge().requests.is_empty());
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
}

#[test]
fn platform_request_supports_shell_open_variants() {
    let bridge = SynchronousResultBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let path = PathBuf::from(r"C:\samples");

    runtime.execute_command(Command::platform_request(
        PlatformRequest::OpenPath(path.clone()),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 1,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::OpenUrl(String::from("https://example.invalid")),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 2,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::RevealPath(path.join("kick.wav")),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 3,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::CopyText(String::from("C:/samples/kick.wav")),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 4,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::CopyFilePaths(vec![path.join("kick.wav")]),
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 5,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::ReadText,
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 6,
            _ => 0,
        },
    ));
    runtime.execute_command(Command::platform_request(
        PlatformRequest::ReadFilePaths,
        |result| match result.expect("platform request should complete") {
            PlatformResponse::Canceled => 7,
            _ => 0,
        },
    ));

    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 7);
    assert_eq!(runtime.bridge().dispatched, vec![1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn unsupported_platform_request_reports_error_message() {
    let bridge = QueuedCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    let request = PlatformRequest::PickFolder(FileDialogRequest::new());

    let outcome = runtime.execute_command(Command::platform_request(request, |result| {
        usize::from(result.is_err())
    }));

    assert_eq!(outcome.messages_dispatched, 0);
    assert!(runtime.bridge().dispatched.is_empty());

    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}

#[test]
fn synchronous_result_host_completion_is_deferred_to_next_drain() {
    let mut runtime = SurfaceRuntime::new(
        SynchronousResultBridge::default(),
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    let outcome = runtime.execute_command(crate::runtime::Command::platform_request(
        PlatformRequest::OpenUrl(String::from("https://example.invalid")),
        |result| usize::from(result.is_ok()),
    ));
    assert_eq!(outcome.messages_dispatched, 0);
    assert!(runtime.bridge().dispatched.is_empty());

    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.bridge().dispatched, vec![1]);
}

#[test]
fn dropped_result_sinks_discard_and_release_mappers_without_dispatch() {
    let mut runtime = SurfaceRuntime::new(
        DroppingResultBridge,
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    for _ in 0..3 {
        let mapper_captures = std::rc::Rc::clone(&captures);
        let outcome = runtime.execute_command(crate::runtime::Command::platform_request(
            PlatformRequest::ReadText,
            move |_| {
                *mapper_captures.borrow_mut() += 1;
                1
            },
        ));
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(std::rc::Rc::strong_count(&captures), 2);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*captures.borrow(), 0);
        assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    }
}

#[test]
fn abandoned_result_sinks_release_bounded_capacity_and_mappers() {
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let mut runtime = SurfaceRuntime::new(
        RetainingResultBridge::default(),
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    for _ in 0..64 {
        let mapper_captures = std::rc::Rc::clone(&captures);
        runtime.execute_command(crate::runtime::Command::platform_request(
            PlatformRequest::ReadText,
            move |_| {
                *mapper_captures.borrow_mut() += 1;
                1
            },
        ));
    }
    assert_eq!(runtime.bridge().sinks.len(), 64);
    assert_eq!(std::rc::Rc::strong_count(&captures), 65);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);

    runtime.bridge_mut().sinks.clear();
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    assert_eq!(*captures.borrow(), 0);

    let mapper_captures = std::rc::Rc::clone(&captures);
    runtime.execute_command(crate::runtime::Command::platform_request(
        PlatformRequest::ReadText,
        move |_| {
            *mapper_captures.borrow_mut() += 1;
            1
        },
    ));
    assert_eq!(runtime.bridge().sinks.len(), 1);
    runtime.bridge_mut().sinks.clear();
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    assert_eq!(*captures.borrow(), 0);
}

#[test]
fn exit_fences_late_sink_drop_without_delivery() {
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let mut runtime = SurfaceRuntime::new(
        RetainingResultBridge::default(),
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    let mapper_captures = std::rc::Rc::clone(&captures);
    runtime.execute_command(crate::runtime::Command::platform_request(
        PlatformRequest::ReadText,
        move |_| {
            *mapper_captures.borrow_mut() += 1;
            1
        },
    ));
    assert_eq!(std::rc::Rc::strong_count(&captures), 2);
    assert!(
        runtime
            .execute_command(crate::runtime::Command::Exit)
            .exit_requested
    );
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    let sink = runtime.bridge_mut().sinks.pop().expect("retained sink");
    drop(sink);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(*captures.borrow(), 0);
}

#[test]
fn runtime_exit_hook_fences_late_platform_send_and_drop_without_command_exit() {
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let bridge = LifecycleResultBridge {
        captures: std::rc::Rc::downgrade(&captures),
        ..LifecycleResultBridge::default()
    };
    let mut runtime = SurfaceRuntime::new(bridge, crate::gui::types::Vector2::new(100.0, 100.0));
    for _ in 0..2 {
        let mapper_captures = std::rc::Rc::clone(&captures);
        runtime.execute_command(crate::runtime::Command::platform_request(
            PlatformRequest::ReadText,
            move |_| {
                *mapper_captures.borrow_mut() += 1;
                1
            },
        ));
    }
    assert_eq!(runtime.bridge().sinks.len(), 2);
    assert_eq!(std::rc::Rc::strong_count(&captures), 3);

    assert_eq!(runtime.host_on_runtime_exit(), None);
    assert_eq!(runtime.bridge().observed_capture_count, Some(1));
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);

    let mut sinks = std::mem::take(&mut runtime.bridge_mut().sinks).into_iter();
    sinks
        .next()
        .expect("first retained sink")
        .send(Ok(PlatformResponse::Completed));
    drop(sinks.next().expect("second retained sink"));
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(*captures.borrow(), 0);
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
}

#[test]
fn platform_request_after_exit_releases_mapper_without_delivery() {
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let mut runtime = SurfaceRuntime::new(
        DroppingResultBridge,
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    assert!(
        runtime
            .execute_command(crate::runtime::Command::Exit)
            .exit_requested
    );
    let mapper_captures = std::rc::Rc::clone(&captures);
    let outcome = runtime.execute_command(crate::runtime::Command::platform_request(
        PlatformRequest::ReadText,
        move |_| {
            *mapper_captures.borrow_mut() += 1;
            1
        },
    ));
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(*captures.borrow(), 0);
}

#[test]
fn unsupported_platform_request_after_exit_releases_mapper_without_delivery() {
    let captures = std::rc::Rc::new(std::cell::RefCell::new(0usize));
    let mut runtime = SurfaceRuntime::new(
        QueuedCommandBridge::default(),
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    assert!(
        runtime
            .execute_command(crate::runtime::Command::Exit)
            .exit_requested
    );
    let mapper_captures = std::rc::Rc::clone(&captures);
    let outcome = runtime.execute_command(crate::runtime::Command::platform_request(
        PlatformRequest::ReadText,
        move |_| {
            *mapper_captures.borrow_mut() += 1;
            1
        },
    ));
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(std::rc::Rc::strong_count(&captures), 1);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    assert_eq!(*captures.borrow(), 0);
}

#[test]
fn command_produced_platform_queue_item_waits_for_next_drain() {
    let mut bridge = ResultQueueBridge::default();
    bridge.commands.push(crate::runtime::Command::Message(1));
    let mut runtime = SurfaceRuntime::new(bridge, crate::gui::types::Vector2::new(100.0, 100.0));
    assert!(runtime.drain_runtime_messages().messages_dispatched > 0);
    assert!(runtime.bridge().dispatched.is_empty());
    assert!(runtime.drain_runtime_messages().messages_dispatched > 0);
    assert_eq!(runtime.bridge().dispatched, vec![2]);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
}

#[test]
fn frozen_platform_overflow_precedes_mapper_enqueued_arrival() {
    let mut runtime = SurfaceRuntime::new(
        QueuedCommandBridge::default(),
        crate::gui::types::Vector2::new(100.0, 100.0),
    );
    let ingress = std::sync::Arc::clone(&runtime.platform_results);
    let enqueued_identity = runtime.platform_registry.register(Box::new(|_| 9));

    let mapper_ingress = std::sync::Arc::clone(&ingress);
    let first_identity = runtime.platform_registry.register(Box::new(move |_| {
        let reservation =
            crate::runtime::controller::platform::PlatformResultIngress::reserve(&mapper_ingress)
                .expect("new mapper arrival should fit behind frozen work");
        assert!(reservation.commit(PlatformResultDelivery::Completed {
            identity: enqueued_identity,
            result: Err(String::from("new")),
        }));
        0
    }));
    let mut pending_identities = vec![first_identity];
    for message in 1..8 {
        pending_identities.push(
            runtime
                .platform_registry
                .register(Box::new(move |_| message)),
        );
    }
    let overflow_identity = runtime.platform_registry.register(Box::new(|_| 8));
    for identity in pending_identities {
        let reservation =
            crate::runtime::controller::platform::PlatformResultIngress::reserve(&ingress)
                .expect("pending platform result reservation");
        assert!(reservation.commit(PlatformResultDelivery::Completed {
            identity,
            result: Err(String::from("pending")),
        }));
    }
    assert!(
        runtime
            .platform_results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .enqueue_overflow(PlatformResultDelivery::Completed {
                identity: overflow_identity,
                result: Err(String::from("overflow")),
            })
    );

    runtime.execute_command(Command::begin_drag(DragRequest::new(
        DragPreview::sized("drag", crate::gui::types::Vector2::new(20.0, 20.0)),
        crate::gui::types::Point::new(0.0, 0.0),
    )));
    assert!(runtime.drain_runtime_messages().runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
    assert_eq!(
        runtime.bridge().dispatched,
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    );
}
