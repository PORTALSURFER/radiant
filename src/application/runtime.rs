use crate::{
    gui::{paint::PaintFrame as GuiPaintFrame, repaint::RepaintSignal, types::Rect},
    widgets::{RetainedSurfaceDescriptor, WidgetId},
};

type RetainedPainter<State> =
    Box<dyn FnMut(&mut State, RetainedSurfaceDescriptor, Rect, Vector2) -> Option<GuiPaintFrame>>;
type AppAnimation<State> = Box<dyn FnMut(&mut State) -> bool>;
type AppFrameMessage<Message> = Box<dyn FnMut() -> Message>;
type AppSubscriptions<State, Message> = Box<dyn FnMut(&mut State) -> Subscription<Message>>;
type AppStartup<State, Message> = Box<dyn FnMut(&mut State, &mut UpdateContext<Message>)>;
type AppShutdown<State> = Box<dyn FnMut(&mut State) -> Option<serde_json::Value>>;
type AppCloseRequested<State> = Box<dyn FnMut(&mut State) -> bool>;
type AppUpdate<State, Message> = Box<dyn FnMut(&mut State, Message, &mut UpdateContext<Message>)>;
type StateStringCallback<State> = Arc<dyn Fn(&mut State, String) + Send + Sync>;
type StateDragCallback<State> =
    Arc<dyn Fn(&mut State, String, crate::widgets::DragHandleMessage) + Send + Sync>;

struct AppRuntime<Message> {
    pending: Mutex<Vec<Message>>,
    commands: Mutex<Vec<Command<Message>>>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    alive: AtomicBool,
    frame_pending: AtomicBool,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            commands: Mutex::new(Vec::new()),
            repaint: Mutex::new(None),
            alive: AtomicBool::new(true),
            frame_pending: AtomicBool::new(false),
        }
    }
}

impl<Message> AppRuntime<Message> {
    fn enqueue(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        self.pending.lock().expect("pending messages poisoned").push(message);
        self.request_repaint();
        true
    }

    fn enqueue_frame(&self, message: Message) -> bool {
        if self.frame_pending.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.enqueue(message)
    }

    fn enqueue_command(&self, command: Command<Message>) -> bool {
        if !self.is_alive() || command.is_empty() {
            return false;
        }
        self.commands
            .lock()
            .expect("pending commands poisoned")
            .push(command);
        self.request_repaint();
        true
    }

    fn take_pending(&self) -> Vec<Message> {
        let pending = std::mem::take(&mut *self.pending.lock().expect("pending messages poisoned"));
        self.frame_pending.store(false, Ordering::Release);
        pending
    }

    fn take_commands(&self) -> Vec<Command<Message>> {
        std::mem::take(&mut *self.commands.lock().expect("pending commands poisoned"))
    }

    fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *self.repaint.lock().expect("repaint signal poisoned") = Some(signal);
    }

    fn request_repaint(&self) {
        if let Some(signal) = self
            .repaint
            .lock()
            .expect("repaint signal poisoned")
            .as_ref()
        {
            signal.request_repaint();
        }
    }

    fn shutdown(&self) {
        self.alive.store(false, Ordering::Release);
        self.frame_pending.store(false, Ordering::Release);
        self.pending
            .lock()
            .expect("pending messages poisoned")
            .clear();
        self.commands
            .lock()
            .expect("pending commands poisoned")
            .clear();
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
}

/// Context supplied to app update closures for runtime-visible follow-up work.
pub struct UpdateContext<Message> {
    commands: Vec<Command<Message>>,
}

impl<Message> Default for UpdateContext<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<Message> UpdateContext<Message> {
    /// Queue a command produced by the current update.
    pub fn command(&mut self, command: Command<Message>) {
        self.commands.push(command);
    }

    /// Queue a host-defined message.
    pub fn emit(&mut self, message: Message) {
        self.command(Command::message(message));
    }

    /// Request another repaint from the active runtime.
    pub fn request_repaint(&mut self) {
        self.command(Command::request_repaint());
    }

    /// Request repaint without forcing declarative surface reprojection.
    pub fn request_paint_only(&mut self) {
        self.command(Command::request_paint_only());
    }

    /// Dispatch a message after a delay.
    pub fn after(&mut self, delay: Duration, message: Message) {
        self.command(Command::after(delay, message));
    }

    /// Run background work and map the output into a host message.
    pub fn spawn<Output>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.command(Command::perform(name, work, map));
    }

    /// Move keyboard focus to a widget.
    pub fn focus(&mut self, widget_id: WidgetId) {
        self.command(Command::focus(widget_id));
    }

    /// Request runtime exit.
    pub fn exit(&mut self) {
        self.command(Command::exit());
    }

    fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}

/// App-level subscription sources evaluated when the native runtime starts.
pub enum Subscription<Message> {
    /// No subscription.
    None,
    /// Multiple subscriptions.
    Batch(Vec<Subscription<Message>>),
    /// Dispatch messages on a fixed interval.
    Interval {
        /// Human-readable subscription id.
        id: &'static str,
        /// Delay between emitted messages.
        every: Duration,
        /// Message factory invoked for each tick.
        message: Arc<dyn Fn() -> Message + Send + Sync>,
    },
    /// Forward messages from a host-owned receiver.
    Worker {
        /// Human-readable subscription id.
        id: &'static str,
        /// Receiver drained on a background thread.
        receiver: std::sync::mpsc::Receiver<Message>,
    },
}

impl<Message> Subscription<Message> {
    /// Empty subscription.
    pub const fn none() -> Self {
        Self::None
    }

    /// Batch multiple subscriptions.
    pub fn batch(subscriptions: impl IntoIterator<Item = Subscription<Message>>) -> Self {
        let subscriptions = subscriptions
            .into_iter()
            .filter(|subscription| !matches!(subscription, Subscription::None))
            .collect::<Vec<_>>();
        if subscriptions.is_empty() {
            Self::None
        } else {
            Self::Batch(subscriptions)
        }
    }

    /// Build an interval subscription.
    pub fn interval(
        id: &'static str,
        every: Duration,
        message: impl Fn() -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::Interval {
            id,
            every,
            message: Arc::new(message),
        }
    }

    /// Build a worker-message subscription from a receiver.
    pub const fn worker(
        id: &'static str,
        receiver: std::sync::mpsc::Receiver<Message>,
    ) -> Self {
        Self::Worker { id, receiver }
    }
}

fn spawn_subscription<Message>(
    runtime: Weak<AppRuntime<Message>>,
    subscription: Subscription<Message>,
) where
    Message: Send + 'static,
{
    match subscription {
        Subscription::None => {}
        Subscription::Batch(subscriptions) => {
            for subscription in subscriptions {
                spawn_subscription(runtime.clone(), subscription);
            }
        }
        Subscription::Interval { id, every, message } => {
            let delay = every.max(Duration::from_millis(1));
            let _ = thread::Builder::new()
                .name(format!("radiant-subscription-{id}"))
                .spawn(move || {
                    loop {
                        thread::sleep(delay);
                        let Some(runtime) = runtime.upgrade() else {
                            break;
                        };
                        if !runtime.enqueue(message()) {
                            break;
                        }
                    }
                });
        }
        Subscription::Worker { id, receiver } => {
            let _ = thread::Builder::new()
                .name(format!("radiant-worker-subscription-{id}"))
                .spawn(move || {
                    for message in receiver {
                        let Some(runtime) = runtime.upgrade() else {
                            break;
                        };
                        if !runtime.enqueue(message) {
                            break;
                        }
                    }
                });
        }
    }
}

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
