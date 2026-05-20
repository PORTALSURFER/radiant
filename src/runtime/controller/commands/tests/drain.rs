use super::{super::*, fixtures::QueuedCommandBridge};

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
