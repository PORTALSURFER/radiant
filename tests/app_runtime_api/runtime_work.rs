use super::*;

#[test]
fn surface_runtime_uses_bridge_drain_into_hooks_for_runtime_work() {
    let bridge = DrainIntoBridge {
        commands: vec![Command::request_repaint()],
        messages: vec![DemoMessage::Increment],
        ..DrainIntoBridge::default()
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 1);
    assert!(drained.repaint_requested);
    assert!(runtime.bridge().drained_commands_into);
    assert!(runtime.bridge().drained_messages_into);
}

#[test]
fn background_message_drains_are_budgeted_to_preserve_ui_responsiveness() {
    let bridge = DrainIntoBridge {
        messages: vec![DemoMessage::Increment; 65],
        ..DrainIntoBridge::default()
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    let first = runtime.drain_runtime_messages();

    assert_eq!(first.messages_dispatched, 64);
    assert!(first.runtime_work_remaining);
    assert!(first.repaint_requested);
    assert!(runtime.take_repaint_requested());

    let second = runtime.drain_runtime_messages();

    assert_eq!(second.messages_dispatched, 1);
    assert!(!second.runtime_work_remaining);
}
