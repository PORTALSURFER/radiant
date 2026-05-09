struct AppBridge<State, Message, Project, Update, View> {
    state: State,
    project: Project,
    update: Update,
    runtime: Arc<AppRuntime<Message>>,
    animation: Option<AppAnimation<State>>,
    frame_message: Option<AppFrameMessage<Message>>,
    subscriptions: Option<AppSubscriptions<State, Message>>,
    startup: Option<AppStartup<State, Message>>,
    shutdown: Option<AppShutdown<State>>,
    close_requested: Option<AppCloseRequested<State>>,
    retained_painters: HashMap<u64, RetainedPainter<State>>,
    subscriptions_started: bool,
    startup_ran: bool,
    _view: PhantomData<View>,
}

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>),
    View: IntoView<Message>,
{
    fn run_update(&mut self, message: Message) -> Command<Message> {
        let mut context = UpdateContext::default();
        (self.update)(&mut self.state, message, &mut context);
        context.into_command()
    }
}

impl<State, Message, Project, Update, View> RuntimeBridge<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new((self.project)(&mut self.state).into_surface())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.run_update(message)
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.install_repaint(signal);
        if !self.startup_ran {
            if let Some(startup) = self.startup.as_mut() {
                let mut context = UpdateContext::default();
                startup(&mut self.state, &mut context);
                self.runtime.enqueue_command(context.into_command());
            }
            self.startup_ran = true;
        }
        if !self.subscriptions_started {
            if let Some(subscriptions) = self.subscriptions.as_mut() {
                spawn_subscription(Arc::downgrade(&self.runtime), subscriptions(&mut self.state));
            }
            self.subscriptions_started = true;
        }
    }

    fn schedule_message(&mut self, delay: Duration, message: Message) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        let _ = thread::Builder::new()
            .name(String::from("radiant-delayed-message"))
            .spawn(move || {
                if !delay.is_zero() {
                    thread::sleep(delay);
                }
                if let Some(runtime) = runtime.upgrade() {
                    let _ = runtime.enqueue(message);
                }
            });
        true
    }

    fn spawn_message_task(
        &mut self,
        name: &'static str,
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        let _ = thread::Builder::new()
            .name(format!("radiant-task-{name}"))
            .spawn(move || {
                let message = work();
                if let Some(runtime) = runtime.upgrade() {
                    let _ = runtime.enqueue(message);
                }
            });
        true
    }

    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        self.runtime.take_commands()
    }

    fn take_runtime_messages(&mut self) -> Vec<Message> {
        self.runtime.take_pending()
    }

    fn needs_animation(&mut self) -> bool {
        let active = self
            .animation
            .as_mut()
            .is_some_and(|animation| animation(&mut self.state));
        if active && let Some(frame_message) = self.frame_message.as_mut() {
            self.runtime.enqueue_frame(frame_message());
        }
        active
    }

    fn render_retained_surface(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: Rect,
        viewport: Vector2,
    ) -> Option<GuiPaintFrame> {
        self.retained_painters
            .get_mut(&descriptor.key)
            .and_then(|paint| paint(&mut self.state, descriptor, rect, viewport))
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.runtime.shutdown();
        self.shutdown
            .as_mut()
            .and_then(|shutdown| shutdown(&mut self.state))
    }

    fn close_requested(&mut self) -> bool {
        self.close_requested
            .as_mut()
            .is_none_or(|close_requested| close_requested(&mut self.state))
    }
}
