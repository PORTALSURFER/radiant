use super::*;
use radiant::runtime::SurfaceChild;
use radiant::widgets::TextInputWidget;

#[test]
fn app_startup_commands_use_full_runtime_dispatch() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::row(
                1,
                8.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                        10,
                        format!("Startup ({})", state.count),
                        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
                    ))),
                    SurfaceChild::fill(SurfaceNode::static_widget(TextInputWidget::new(
                        11,
                        state.name.clone(),
                        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
                    ))),
                ],
            ))
        })
        .on_startup(|state, context| {
            state.name = String::from("ready");
            context.focus(11);
            context.request_repaint();
            context.after(Duration::from_millis(1), DemoMessage::Increment);
            context.spawn(
                "startup-increment",
                || DemoMessage::Increment,
                |message| message,
            );
        })
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(300.0, 48.0));
    let repaint_called = Arc::new(AtomicBool::new(false));
    runtime
        .bridge_mut()
        .install_repaint_signal(Arc::new(CountingRepaintSignal {
            called: Arc::clone(&repaint_called),
        }));

    let startup = runtime.drain_runtime_messages();
    assert!(startup.repaint_requested);
    assert_eq!(runtime.focused_widget(), Some(11));
    assert!(repaint_called.load(Ordering::Acquire));

    let drained = drain_until_messages(&mut runtime, 2);
    assert_eq!(drained.messages_dispatched, 2);
    assert_eq!(text_value(runtime.surface(), 10), "Startup (2)");
}

#[test]
fn app_startup_runs_once_when_repaint_signal_is_reinstalled() {
    let mut bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Startup runs: {}", state.count),
                WidgetSizing::fixed(Vector2::new(160.0, 20.0)).with_baseline(14.0),
            )))
        })
        .on_startup(|state, _context| {
            state.count += 1;
        })
        .update_with(|_state, _message: DemoMessage, _context| {})
        .into_bridge();

    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::new(AtomicBool::new(false)),
    }));
    bridge.install_repaint_signal(Arc::new(CountingRepaintSignal {
        called: Arc::new(AtomicBool::new(false)),
    }));

    let surface = bridge.project_surface();

    assert_eq!(text_value(&surface, 10), "Startup runs: 1");
}

#[test]
fn app_runtime_effects_stop_after_runtime_exit() {
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Effects ({})", state.count),
                WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
            )))
        })
        .subscriptions(|_| {
            radiant::prelude::Subscription::interval("fast", Duration::from_millis(1), || {
                DemoMessage::Increment
            })
        })
        .update_with(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    runtime
        .bridge_mut()
        .install_repaint_signal(Arc::new(CountingRepaintSignal {
            called: Arc::new(AtomicBool::new(false)),
        }));

    let active = drain_until_messages(&mut runtime, 1);
    assert!(active.messages_dispatched > 0);

    let _ = runtime.bridge_mut().on_runtime_exit();
    let delayed = runtime.execute_command(Command::after(
        Duration::from_millis(5),
        DemoMessage::Increment,
    ));
    assert!(!delayed.repaint_requested);
    let stopped = runtime.drain_runtime_messages();
    assert_eq!(stopped.messages_dispatched, 0);
}
