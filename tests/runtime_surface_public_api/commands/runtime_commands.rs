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

    let mut context = radiant::prelude::UpdateContext::default();
    context
        .business()
        .background("increment")
        .run(|_| DemoMessage::Increment, |message| message);
    let performed = runtime.execute_command(context.into_command());
    assert!(performed.repaint_requested);
    let drained = drain_until_messages(&mut runtime, 1);
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(runtime.bridge().count, 2);

    let exit = runtime.execute_command(Command::exit());
    assert!(exit.exit_requested);
    assert!(runtime.take_exit_requested());
}

#[test]
fn surface_runtime_records_slow_update_handler_diagnostics() {
    struct SlowUpdateBridge;

    impl RuntimeBridge<DemoMessage> for SlowUpdateBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
            Arc::new(UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                1,
                "Diagnostics",
                radiant::widgets::WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            ))))
        }

        fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
            std::thread::sleep(Duration::from_millis(60));
            Command::none()
        }
    }

    let mut runtime = SurfaceRuntime::new(SlowUpdateBridge, Vector2::new(160.0, 80.0));

    runtime.dispatch_message(DemoMessage::Increment);

    let diagnostics = runtime.runtime_diagnostics();
    assert_eq!(diagnostics.ui.update_handlers, 1);
    assert_eq!(diagnostics.ui.slow_update_handlers, 1);
    assert!(diagnostics.ui.longest_update_handler >= Duration::from_millis(50));
}
