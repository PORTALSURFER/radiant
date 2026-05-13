use super::*;
use crate::layout::ContainerPolicy;
use crate::runtime::SurfaceNode;
use std::sync::Arc;

#[derive(Default)]
struct QueuedCommandBridge {
    commands: Vec<Command<usize>>,
    dispatched: Vec<usize>,
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

#[test]
fn runtime_command_batch_preserves_order_and_keeps_remainder() {
    let mut commands = (0..70).map(Command::message).collect::<Vec<_>>();
    let mut batch = Vec::with_capacity(8);

    take_runtime_command_batch_into(&mut commands, &mut batch);

    let mut drained = Vec::new();
    while let Some(command) = batch.pop() {
        let Command::Message(message) = command else {
            panic!("test command should be a message");
        };
        drained.push(message);
    }
    assert_eq!(drained, (0..64).collect::<Vec<_>>());
    assert_eq!(commands.len(), 6);
    assert!(commands.iter().enumerate().all(
        |(offset, command)| matches!(command, Command::Message(message) if *message == offset + 64)
    ));
    assert!(batch.capacity() >= 64);
}

#[test]
fn runtime_command_batch_reuses_output_storage_for_small_drains() {
    let mut commands = vec![
        Command::message(1),
        Command::message(2),
        Command::message(3),
    ];
    let mut batch = Vec::with_capacity(64);
    let capacity = batch.capacity();

    take_runtime_command_batch_into(&mut commands, &mut batch);

    assert!(commands.is_empty());
    assert_eq!(batch.capacity(), capacity);
    assert!(matches!(batch.pop(), Some(Command::Message(1))));
    assert!(matches!(batch.pop(), Some(Command::Message(2))));
    assert!(matches!(batch.pop(), Some(Command::Message(3))));
    assert!(batch.pop().is_none());
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
fn runtime_batched_command_remainders_reuse_command_storage() {
    let mut commands = Vec::with_capacity(128);
    commands.push(Command::batch((0..70).map(Command::message)));
    let retained_capacity = commands.capacity();
    let mut batch = Vec::with_capacity(64);

    take_runtime_command_batch_into(&mut commands, &mut batch);

    assert_eq!(commands.capacity(), retained_capacity);
    assert_eq!(commands.len(), 6);
}

#[test]
fn runtime_message_batch_preserves_order_and_keeps_remainder() {
    let mut messages = (0..70).collect::<Vec<_>>();
    let mut batch = Vec::with_capacity(8);

    take_runtime_message_batch_into(&mut messages, &mut batch);

    let mut drained = Vec::new();
    while let Some(message) = batch.pop() {
        drained.push(message);
    }
    assert_eq!(drained, (0..64).collect::<Vec<_>>());
    assert_eq!(messages, (64..70).collect::<Vec<_>>());
    assert!(batch.capacity() >= 64);
}

#[test]
fn runtime_message_batch_reuses_output_storage_for_small_drains() {
    let mut messages = vec![1, 2, 3];
    let mut batch = Vec::with_capacity(64);
    let capacity = batch.capacity();

    take_runtime_message_batch_into(&mut messages, &mut batch);

    assert!(messages.is_empty());
    assert_eq!(batch.capacity(), capacity);
    assert_eq!(batch.pop(), Some(1));
    assert_eq!(batch.pop(), Some(2));
    assert_eq!(batch.pop(), Some(3));
    assert_eq!(batch.pop(), None);
}
