use super::fixtures::{RuntimeCommandBridge, drain_until_messages};
use super::*;

#[test]
fn custom_host_moves_only_opaque_wake_for_non_send_ui_message() {
    use std::rc::Rc;

    #[derive(Clone)]
    struct UiOnlyMessage(Rc<str>);

    #[derive(Default)]
    struct UiOnlyBridge {
        count: usize,
        wakes: Arc<Mutex<Vec<radiant::runtime::RuntimeTimerWake>>>,
    }

    impl RuntimeBridge<UiOnlyMessage> for UiOnlyBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<UiOnlyMessage>> {
            Arc::new(UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                1,
                "UI-only",
                radiant::widgets::WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            ))))
        }

        fn update(&mut self, message: UiOnlyMessage) -> Command<UiOnlyMessage> {
            if message.0.as_ref() == "timer" {
                self.count += 1;
            }
            Command::none()
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, UiOnlyMessage> {
            RuntimeHostCapabilities::new().with_tasks().with_queues()
        }
    }

    impl RuntimeTaskHost<UiOnlyMessage> for UiOnlyBridge {
        fn schedule_timer(
            &mut self,
            delay: Duration,
            wake: radiant::runtime::RuntimeTimerWake,
        ) -> bool {
            let wakes = Arc::clone(&self.wakes);
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                wakes.lock().expect("timer wakes poisoned").push(wake);
            });
            true
        }
    }

    impl RuntimeQueueHost<UiOnlyMessage> for UiOnlyBridge {
        fn take_runtime_timer_wakes(&mut self) -> Vec<radiant::runtime::RuntimeTimerWake> {
            std::mem::take(&mut *self.wakes.lock().expect("timer wakes poisoned"))
        }
    }

    let mut runtime = SurfaceRuntime::new(UiOnlyBridge::default(), Vector2::new(160.0, 80.0));
    runtime.execute_command(Command::after(
        Duration::from_millis(1),
        UiOnlyMessage(Rc::from("timer")),
    ));
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut dispatched = 0;
    while dispatched < 1 && Instant::now() < deadline {
        dispatched += runtime.drain_runtime_messages().messages_dispatched;
        if dispatched < 1 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(dispatched, 1);
    assert_eq!(runtime.bridge().count, 1);
}

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

    let mut context = radiant::prelude::UiUpdateContext::default();
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
fn retained_timer_wakes_keep_runtime_work_alive_until_next_budgeted_drain() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
    let command = Command::batch(
        (0..70).map(|_| Command::after(Duration::from_millis(1), DemoMessage::Increment)),
    );

    runtime.execute_command(command);
    let deadline = Instant::now() + Duration::from_secs(1);
    while runtime.bridge().pending_timer_wake_count() < 70 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(runtime.bridge().pending_timer_wake_count(), 70);

    let first = runtime.drain_runtime_messages();
    assert_eq!(first.messages_dispatched, 64);
    assert!(first.runtime_work_remaining);
    assert!(first.repaint_requested);
    assert!(runtime.bridge().command_drain_count() >= 1);

    let second = runtime.drain_runtime_messages();
    assert_eq!(second.messages_dispatched, 6);
    assert!(!second.runtime_work_remaining);
    assert_eq!(runtime.bridge().count, 70);
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
