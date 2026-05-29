use super::*;

pub(super) enum CommandDemoMessage {
    Start,
    MixedRepaint,
    Increment,
    Rename(String),
}

pub(super) struct CommandDemoBridge {
    pub(super) state: DemoState,
}

#[derive(Default)]
pub(super) struct RuntimeCommandBridge {
    pub(super) count: usize,
    pending: Arc<std::sync::Mutex<Vec<DemoMessage>>>,
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
        _priority: radiant::runtime::TaskPriority,
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

pub(super) fn project_demo_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
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

pub(super) fn drain_until_messages<Bridge>(
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
