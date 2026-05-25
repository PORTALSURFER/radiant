use super::*;
use radiant::theme::DpiScale;
use std::time::{Duration, Instant};

enum CommandDemoMessage {
    Start,
    MixedRepaint,
    Increment,
    Rename(String),
}

struct CommandDemoBridge {
    state: DemoState,
}

#[test]
fn surface_runtime_treats_mixed_repaint_batches_as_surface_refreshes() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::MixedRepaint);

    assert!(outcome.repaint_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.paint_only_requested);
    assert!(outcome.surface_refresh_requested);
}

#[derive(Default)]
struct RuntimeCommandBridge {
    count: usize,
    pending: Arc<std::sync::Mutex<Vec<DemoMessage>>>,
}

#[test]
fn surface_runtime_executes_command_messages_and_repaint_requests() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(runtime.repaint_requested());
    assert!(runtime.take_repaint_requested());
    assert!(!runtime.repaint_requested());

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Commands (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "Commands"
    );
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

#[test]
fn surface_runtime_reports_dpi_scale_overrides_as_surface_refreshes() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let outcome = runtime.execute_command(Command::set_dpi_scale(DpiScale::new(2.0)));

    assert_eq!(outcome.dpi_scale_override, Some(DpiScale::new(2.0)));
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(!outcome.paint_only_requested);
}

#[test]
fn surface_runtime_reports_window_size_requests_as_surface_refreshes() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let requested = Vector2::new(760.0, 520.0);
    let outcome = runtime.execute_command(Command::set_window_logical_size(requested));

    assert_eq!(outcome.window_logical_size, Some(requested));
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(!outcome.paint_only_requested);
}

impl RuntimeBridge<CommandDemoMessage> for CommandDemoBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<CommandDemoMessage>> {
        project_demo_surface(&mut self.state)
    }

    fn update(&mut self, message: CommandDemoMessage) -> Command<CommandDemoMessage> {
        match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Commands"))),
                Command::request_repaint(),
                Command::message(CommandDemoMessage::Increment),
                Command::request_paint_only(),
            ]),
            CommandDemoMessage::MixedRepaint => Command::batch([
                Command::request_paint_only(),
                Command::repaint(RepaintScope::Surface),
            ]),
            CommandDemoMessage::Increment => {
                self.state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                self.state.name = name;
                Command::none()
            }
        }
    }
}

impl RuntimeBridge<DemoMessage> for RuntimeCommandBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        Arc::new(UiSurface::new(SurfaceNode::row(
            1,
            8.0,
            vec![SurfaceChild::fill(SurfaceNode::static_widget(
                ButtonWidget::new(11, "Focus", WidgetSizing::fixed(Vector2::new(80.0, 32.0))),
            ))],
        )))
    }

    fn update(&mut self, message: DemoMessage) -> Command<DemoMessage> {
        if matches!(message, DemoMessage::Increment) {
            self.count += 1;
        }
        Command::none()
    }

    fn schedule_message(&mut self, delay: Duration, message: DemoMessage) -> bool {
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            pending
                .lock()
                .expect("pending messages poisoned")
                .push(message);
        });
        true
    }

    fn spawn_message_task(
        &mut self,
        _name: &'static str,
        work: Box<dyn FnOnce() -> DemoMessage + Send + 'static>,
    ) -> bool {
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            pending
                .lock()
                .expect("pending messages poisoned")
                .push(work());
        });
        true
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        std::mem::take(&mut *self.pending.lock().expect("pending messages poisoned"))
    }
}

fn project_demo_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(11, "Run", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    );

    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| CommandDemoMessage::Start),
            )),
            SurfaceChild::fill(SurfaceNode::widget(input, WidgetMessageMapper::none())),
        ],
    )))
}

fn drain_until_messages<Bridge>(
    runtime: &mut SurfaceRuntime<Bridge, DemoMessage>,
    min_messages: usize,
) -> radiant::runtime::CommandOutcome
where
    Bridge: RuntimeBridge<DemoMessage>,
{
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut drained = radiant::runtime::CommandOutcome::default();
    loop {
        let outcome = runtime.drain_runtime_messages();
        drained.messages_dispatched += outcome.messages_dispatched;
        drained.repaint_requested |= outcome.repaint_requested;
        drained.surface_refresh_requested |= outcome.surface_refresh_requested;
        drained.exit_requested |= outcome.exit_requested;
        if drained.messages_dispatched >= min_messages || Instant::now() >= deadline {
            return drained;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
