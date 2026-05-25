use super::fixtures::{RuntimeCommandBridge, drain_until_messages};
use super::*;

#[test]
fn surface_runtime_executes_focus_exit_and_deferred_commands() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let focus = runtime.execute_command(Command::focus(11));
    assert!(!focus.exit_requested);
    assert_eq!(runtime.focused_widget(), Some(11));

    let deferred = runtime.execute_command(Command::after(
        Duration::from_millis(1),
        DemoMessage::Increment,
    ));
    assert!(deferred.repaint_requested);
    let drained = drain_until_messages(&mut runtime, 1);
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 1);

    let performed = runtime.execute_command(Command::perform(
        "increment",
        || DemoMessage::Increment,
        |message| message,
    ));
    assert!(performed.repaint_requested);
    let drained = drain_until_messages(&mut runtime, 1);
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 2);

    let exit = runtime.execute_command(Command::exit());
    assert!(exit.exit_requested);
    assert!(runtime.take_exit_requested());
}
