use super::{super::*, fixtures::QueuedCommandBridge};
use crate::layout::ContainerPolicy;
use crate::runtime::{
    DragPreview, DragRequest, RuntimeHostCapabilities, RuntimeQueueDelivery, RuntimeQueueHost,
    RuntimeQueueItem, RuntimeTimerWake, SurfaceNode, UiSurface,
};
use std::sync::Arc;

#[derive(Default)]
struct ExitTimerBridge {
    wakes: Vec<RuntimeTimerWake>,
    mapped: usize,
    reduced: usize,
}

impl RuntimeBridge<usize> for ExitTimerBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, _message: usize) {
        self.reduced += 1;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_queues()
    }
}

impl RuntimeQueueHost<usize> for ExitTimerBridge {
    fn take_runtime_timer_wakes(&mut self) -> Vec<RuntimeTimerWake> {
        std::mem::take(&mut self.wakes)
    }

    fn map_runtime_timer_wake(&mut self, _wake: RuntimeTimerWake) -> Option<usize> {
        self.mapped += 1;
        Some(7)
    }
}

#[derive(Default)]
struct OrderedIngressBridge {
    items: Vec<RuntimeQueueItem<usize>>,
    mapped: Vec<&'static str>,
    reduced: Vec<usize>,
}

impl RuntimeBridge<usize> for OrderedIngressBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
        Arc::new(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, message: usize) {
        self.reduced.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
        RuntimeHostCapabilities::new().with_queues()
    }
}

impl RuntimeQueueHost<usize> for OrderedIngressBridge {
    fn drain_runtime_queue_item_batch_into(
        &mut self,
        items: &mut Vec<RuntimeQueueItem<usize>>,
        _max_items: usize,
    ) -> bool {
        items.append(&mut self.items);
        false
    }

    fn map_runtime_timer_wake(&mut self, _wake: RuntimeTimerWake) -> Option<usize> {
        self.mapped.push("timer");
        Some(2)
    }

    fn map_runtime_queue_delivery(&mut self, delivery: RuntimeQueueDelivery) -> Option<usize> {
        self.mapped.push("delivery");
        delivery.downcast::<usize>().ok()
    }
}

#[test]
fn ordered_ingress_maps_and_reduces_earlier_delivery_before_later_timer() {
    let bridge = OrderedIngressBridge {
        items: vec![
            RuntimeQueueItem::Delivery(RuntimeQueueDelivery::new(1_usize)),
            RuntimeQueueItem::Timer(RuntimeTimerWake::application(1, 0, 1)),
        ],
        mapped: Vec::new(),
        reduced: Vec::new(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));

    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 2);
    assert_eq!(runtime.bridge().mapped, vec!["delivery", "timer"]);
    assert_eq!(runtime.bridge().reduced, vec![1, 2]);
}

#[test]
fn ordered_ingress_maps_and_reduces_earlier_timer_before_later_delivery() {
    let bridge = OrderedIngressBridge {
        items: vec![
            RuntimeQueueItem::Timer(RuntimeTimerWake::application(1, 0, 1)),
            RuntimeQueueItem::Delivery(RuntimeQueueDelivery::new(1_usize)),
        ],
        mapped: Vec::new(),
        reduced: Vec::new(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));

    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 2);
    assert_eq!(runtime.bridge().mapped, vec!["timer", "delivery"]);
    assert_eq!(runtime.bridge().reduced, vec![2, 1]);
}

#[test]
fn exit_fences_application_timer_wakes_before_lifecycle_cleanup() {
    let mut runtime = SurfaceRuntime::new(ExitTimerBridge::default(), Vector2::new(100.0, 100.0));
    runtime
        .bridge_mut()
        .wakes
        .push(RuntimeTimerWake::application(1, 0, 1));

    let outcome = runtime.execute_command(Command::exit());
    assert!(outcome.exit_requested);

    // A late host wake can arrive before the host lifecycle callback clears
    // its application timer registry; the controller must consume it fenced.
    runtime
        .bridge_mut()
        .wakes
        .push(RuntimeTimerWake::application(2, 0, 1));
    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 0);
    assert_eq!(runtime.bridge().mapped, 0);
    assert_eq!(runtime.bridge().reduced, 0);
}

#[test]
fn runtime_command_drains_are_bounded_and_request_followup_wakeup() {
    let bridge = QueuedCommandBridge {
        commands: (0..70).map(Command::message).collect(),
        dispatched: Vec::new(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));

    let first = runtime.drain_runtime_messages();

    assert_eq!(first.messages_dispatched, 64);
    assert!(first.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..64).collect::<Vec<_>>());

    let second = runtime.drain_runtime_messages();

    assert_eq!(second.messages_dispatched, 6);
    assert!(!second.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..70).collect::<Vec<_>>());
}

#[test]
fn runtime_batched_command_drains_are_bounded_and_request_followup_wakeup() {
    let bridge = QueuedCommandBridge {
        commands: vec![Command::batch((0..70).map(Command::message))],
        dispatched: Vec::new(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));

    let first = runtime.drain_runtime_messages();

    assert_eq!(first.messages_dispatched, 64);
    assert!(first.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..64).collect::<Vec<_>>());

    let second = runtime.drain_runtime_messages();

    assert_eq!(second.messages_dispatched, 6);
    assert!(!second.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..70).collect::<Vec<_>>());
}

#[test]
fn runtime_batched_command_remainders_preserve_following_command_order() {
    let bridge = QueuedCommandBridge {
        commands: vec![
            Command::batch((0..70).map(Command::message)),
            Command::message(70),
        ],
        dispatched: Vec::new(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));

    let first = runtime.drain_runtime_messages();

    assert_eq!(first.messages_dispatched, 64);
    assert!(first.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..64).collect::<Vec<_>>());

    let second = runtime.drain_runtime_messages();

    assert_eq!(second.messages_dispatched, 7);
    assert!(!second.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..71).collect::<Vec<_>>());
}

#[test]
fn runtime_message_drains_are_smaller_during_active_drag() {
    let bridge = QueuedCommandBridge {
        commands: (0..70).map(Command::message).collect(),
        dispatched: Vec::new(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(100.0, 100.0));
    runtime.execute_command(Command::begin_drag(DragRequest::new(
        DragPreview::sized("dragging", Vector2::new(120.0, 24.0)),
        Point::new(20.0, 20.0),
    )));

    let first = runtime.drain_runtime_messages();

    assert_eq!(first.messages_dispatched, 8);
    assert!(first.runtime_work_remaining);
    assert_eq!(runtime.bridge().dispatched, (0..8).collect::<Vec<_>>());
}
