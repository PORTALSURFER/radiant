use super::super::*;
use crate::runtime::controller::commands::batching::{
    DEFAULT_RUNTIME_COMMANDS_PER_DRAIN, DEFAULT_RUNTIME_MESSAGES_PER_DRAIN,
    take_runtime_command_batch_into, take_runtime_message_batch_into,
};

#[test]
fn runtime_command_batch_preserves_order_and_keeps_remainder() {
    let mut commands = (0..70).map(Command::message).collect::<Vec<_>>();
    let mut batch = Vec::with_capacity(8);
    let mut pending = Vec::new();

    take_runtime_command_batch_into(
        &mut commands,
        &mut batch,
        &mut pending,
        DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
    );

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
    assert!(pending.is_empty());
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
    let mut pending = Vec::new();
    let capacity = batch.capacity();

    take_runtime_command_batch_into(
        &mut commands,
        &mut batch,
        &mut pending,
        DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
    );

    assert!(commands.is_empty());
    assert_eq!(batch.capacity(), capacity);
    assert!(matches!(batch.pop(), Some(Command::Message(1))));
    assert!(matches!(batch.pop(), Some(Command::Message(2))));
    assert!(matches!(batch.pop(), Some(Command::Message(3))));
    assert!(batch.pop().is_none());
    assert!(pending.is_empty());
}

#[test]
fn runtime_batched_command_remainders_reuse_command_storage() {
    let mut commands = Vec::with_capacity(128);
    commands.push(Command::batch((0..70).map(Command::message)));
    let retained_capacity = commands.capacity();
    let mut batch = Vec::with_capacity(64);
    let mut pending = Vec::with_capacity(128);
    let pending_capacity = pending.capacity();

    take_runtime_command_batch_into(
        &mut commands,
        &mut batch,
        &mut pending,
        DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
    );

    assert_eq!(commands.capacity(), retained_capacity);
    assert_eq!(pending.capacity(), pending_capacity);
    assert!(pending.is_empty());
    assert_eq!(commands.len(), 6);
}

#[test]
fn runtime_batched_command_drains_reuse_pending_scratch_storage() {
    let mut commands = vec![Command::batch((0..70).map(Command::message))];
    let mut batch = Vec::with_capacity(64);
    let mut pending = Vec::with_capacity(128);
    let pending_capacity = pending.capacity();

    take_runtime_command_batch_into(
        &mut commands,
        &mut batch,
        &mut pending,
        DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
    );
    batch.clear();
    commands.clear();
    commands.push(Command::batch((0..2).map(Command::message)));
    take_runtime_command_batch_into(
        &mut commands,
        &mut batch,
        &mut pending,
        DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
    );

    assert_eq!(pending.capacity(), pending_capacity);
    assert!(pending.is_empty());
    assert_eq!(commands.len(), 0);
    assert_eq!(batch.len(), 2);
}

#[test]
fn runtime_command_batch_defers_flattening_batches_beyond_drain_budget() {
    let mut commands = (0..64).map(Command::message).collect::<Vec<_>>();
    commands.push(Command::batch([Command::message(64), Command::message(65)]));
    let mut batch = Vec::with_capacity(8);
    let mut pending = Vec::new();

    take_runtime_command_batch_into(
        &mut commands,
        &mut batch,
        &mut pending,
        DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
    );

    let mut drained = Vec::new();
    while let Some(command) = batch.pop() {
        let Command::Message(message) = command else {
            panic!("test command should be a message");
        };
        drained.push(message);
    }
    assert_eq!(drained, (0..64).collect::<Vec<_>>());
    assert_eq!(commands.len(), 1);
    assert!(matches!(commands.first(), Some(Command::Batch(_))));
    assert!(pending.is_empty());
}

#[test]
fn runtime_message_batch_preserves_order_and_keeps_remainder() {
    let mut messages = (0..70).collect::<Vec<_>>();
    let mut batch = Vec::with_capacity(8);

    take_runtime_message_batch_into(
        &mut messages,
        &mut batch,
        DEFAULT_RUNTIME_MESSAGES_PER_DRAIN,
    );

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

    take_runtime_message_batch_into(
        &mut messages,
        &mut batch,
        DEFAULT_RUNTIME_MESSAGES_PER_DRAIN,
    );

    assert!(messages.is_empty());
    assert_eq!(batch.capacity(), capacity);
    assert_eq!(batch.pop(), Some(1));
    assert_eq!(batch.pop(), Some(2));
    assert_eq!(batch.pop(), Some(3));
    assert_eq!(batch.pop(), None);
}
