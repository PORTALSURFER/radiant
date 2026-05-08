/// Build a native window launcher for a simple Radiant view.
pub fn window(title: impl Into<String>) -> WindowBuilder {
    WindowBuilder::new(title)
}

/// Build a stateful app launcher over the existing command runtime bridge.
pub fn app<State>(state: State) -> StatefulAppBuilder<State> {
    StatefulAppBuilder::new(state)
}

/// Converts application view values into the existing runtime surface.
pub trait IntoView<Message> {
    /// Lower this value into a runtime surface node.
    fn into_node(self) -> SurfaceNode<Message>;

    /// Lower this value into a top-level runtime surface.
    fn into_surface(self) -> UiSurface<Message>
    where
        Self: Sized,
    {
        UiSurface::new(self.into_node())
    }
}

impl<Message> IntoView<Message> for SurfaceNode<Message> {
    fn into_node(self) -> SurfaceNode<Message> {
        self
    }
}

impl<Message> IntoView<Message> for UiSurface<Message> {
    fn into_node(self) -> SurfaceNode<Message> {
        self.into_root()
    }

    fn into_surface(self) -> UiSurface<Message> {
        self
    }
}

/// Builder for no-state native windows.
pub struct WindowBuilder {
    options: NativeRunOptions,
}

impl WindowBuilder {
    fn new(title: impl Into<String>) -> Self {
        Self {
            options: NativeRunOptions {
                title: title.into(),
                ..NativeRunOptions::default()
            },
        }
    }

    /// Set the initial logical window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.options.inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.options.min_inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the full native runtime options, preserving this builder as a thin adapter.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Run one static view through the native Vello runtime.
    pub fn run<View>(self, view: View) -> Result
    where
        View: IntoView<()> + 'static,
    {
        let surface = Arc::new(view.into_surface());
        let bridge = declarative_command_runtime_bridge(
            surface,
            |surface| Arc::clone(surface),
            |_, ()| Command::none(),
        );
        run_native_vello_runtime(self.options, bridge)
    }

    /// Run an existing runtime bridge through this window builder.
    ///
    /// This keeps host-specific bridges on the same application/window launch
    /// API as ordinary Radiant examples while preserving their custom state and
    /// diagnostics.
    pub fn run_bridge<Bridge, Message>(self, bridge: Bridge) -> Result
    where
        Bridge: RuntimeBridge<Message> + 'static,
        Message: 'static,
    {
        run_native_vello_runtime(self.options, bridge)
    }

    /// Run an existing runtime bridge and return native runtime artifacts.
    pub fn run_bridge_with_artifacts<Bridge, Message>(
        self,
        bridge: Bridge,
    ) -> crate::gui_runtime::NativeGenericRunReport
    where
        Bridge: RuntimeBridge<Message> + 'static,
        Message: 'static,
    {
        crate::runtime::run_native_vello_runtime_with_artifacts(self.options, bridge)
    }
}

/// Initial builder for simple stateful Radiant apps.
pub struct StatefulAppBuilder<State> {
    state: State,
    options: NativeRunOptions,
}

impl<State> StatefulAppBuilder<State> {
    fn new(state: State) -> Self {
        Self {
            state,
            options: NativeRunOptions::default(),
        }
    }

    /// Set the native window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = title.into();
        self
    }

    /// Set the initial logical window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.options.inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.options.min_inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the full native runtime options for apps that need explicit launch control.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Attach a state projection closure.
    pub fn view<Message, Project, View>(
        self,
        project: Project,
    ) -> StatefulAppWithView<State, Message, Project, View>
    where
        Project: FnMut(&mut State) -> View,
        View: IntoView<Message>,
    {
        StatefulAppWithView {
            state: self.state,
            options: self.options,
            project,
            animation: None,
            frame_message: None,
            subscriptions: None,
            startup: None,
            shutdown: None,
            close_requested: None,
            retained_painters: HashMap::new(),
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}

/// Stateful app builder after a view projection has been supplied.
pub struct StatefulAppWithView<State, Message, Project, View> {
    state: State,
    options: NativeRunOptions,
    project: Project,
    animation: Option<Box<dyn FnMut(&mut State) -> bool>>,
    frame_message: Option<Box<dyn FnMut() -> Message>>,
    subscriptions: Option<Box<dyn FnMut(&mut State) -> Subscription<Message>>>,
    startup: Option<Box<dyn FnMut(&mut State, &mut UpdateContext<Message>)>>,
    shutdown: Option<Box<dyn FnMut(&mut State) -> Option<serde_json::Value>>>,
    close_requested: Option<Box<dyn FnMut(&mut State) -> bool>>,
    retained_painters: HashMap<u64, RetainedPainter<State>>,
    _message: PhantomData<Message>,
    _view: PhantomData<View>,
}

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    /// Attach a reducer that mutates app state and requests a repaint.
    pub fn update<Update>(
        self,
        mut update: Update,
    ) -> RunnableStatefulApp<
        State,
        Message,
        Project,
        impl FnMut(&mut State, Message, &mut UpdateContext<Message>),
        View,
    >
    where
        Update: FnMut(&mut State, Message) + 'static,
    {
        self.update_command(move |state, message| {
            update(state, message);
            Command::request_repaint()
        })
    }

    /// Attach a reducer that can queue runtime-visible work through an update context.
    pub fn update_with<Update>(
        self,
        update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, Update, View>
    where
        Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    {
        RunnableStatefulApp {
            state: self.state,
            options: self.options,
            project: self.project,
            update,
            animation: self.animation,
            frame_message: self.frame_message,
            subscriptions: self.subscriptions,
            startup: self.startup,
            shutdown: self.shutdown,
            close_requested: self.close_requested,
            retained_painters: self.retained_painters,
            _message: PhantomData,
            _view: PhantomData,
        }
    }

    /// Attach a reducer that returns runtime-visible commands.
    pub fn update_command<Update>(
        self,
        mut update: Update,
    ) -> RunnableStatefulApp<
        State,
        Message,
        Project,
        impl FnMut(&mut State, Message, &mut UpdateContext<Message>),
        View,
    >
    where
        Update: FnMut(&mut State, Message) -> Command<Message> + 'static,
    {
        self.update_with(move |state, message, context| {
            context.command(update(state, message));
        })
    }

    /// Declare whether this app currently needs animation-driven frames.
    pub fn animation(mut self, animation: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.animation = Some(Box::new(animation));
        self
    }

    /// Declare the message emitted for each active animation frame.
    pub fn on_frame(mut self, message: impl FnMut() -> Message + 'static) -> Self {
        self.frame_message = Some(Box::new(message));
        self
    }

    /// Declare app-level subscriptions started when the native runtime starts.
    pub fn subscriptions(
        mut self,
        subscriptions: impl FnMut(&mut State) -> Subscription<Message> + 'static,
    ) -> Self {
        self.subscriptions = Some(Box::new(subscriptions));
        self
    }

    /// Register a startup hook.
    pub fn on_startup(
        mut self,
        startup: impl FnMut(&mut State, &mut UpdateContext<Message>) + 'static,
    ) -> Self {
        self.startup = Some(Box::new(startup));
        self
    }

    /// Register a shutdown artifact hook.
    pub fn on_shutdown(
        mut self,
        shutdown: impl FnMut(&mut State) -> Option<serde_json::Value> + 'static,
    ) -> Self {
        self.shutdown = Some(Box::new(shutdown));
        self
    }

    /// Register a close-request hook. Return `true` to close.
    pub fn on_close_requested(mut self, close_requested: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.close_requested = Some(Box::new(close_requested));
        self
    }

    /// Register a retained-surface painter by descriptor key.
    pub fn retained_painter(
        mut self,
        key: u64,
        paint: impl FnMut(
                &mut State,
                RetainedSurfaceDescriptor,
                crate::gui::types::Rect,
                Vector2,
            ) -> Option<crate::gui::paint::PaintFrame>
            + 'static,
    ) -> Self {
        self.retained_painters.insert(key, Box::new(paint));
        self
    }
}

impl<State, Project, View> StatefulAppWithView<State, StateAction<State>, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<StateAction<State>> + 'static,
    State: 'static,
{
    /// Run this direct-callback app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Run this app and return native runtime artifacts.
    pub fn run_with_artifacts(self) -> crate::gui_runtime::NativeGenericRunReport {
        let options = self.options.clone();
        crate::runtime::run_native_vello_runtime_with_artifacts(options, self.into_bridge())
    }

    /// Lower this direct-callback app into the existing runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<StateAction<State>> {
        let mut project = self.project;
        declarative_command_runtime_bridge(
            self.state,
            move |state| Arc::new(project(state).into_surface()),
            |state, action| {
                action.run(state);
                Command::request_repaint()
            },
        )
    }
}

/// Runnable stateful app builder.
pub struct RunnableStatefulApp<State, Message, Project, Update, View> {
    state: State,
    options: NativeRunOptions,
    project: Project,
    update: Update,
    animation: Option<Box<dyn FnMut(&mut State) -> bool>>,
    frame_message: Option<Box<dyn FnMut() -> Message>>,
    subscriptions: Option<Box<dyn FnMut(&mut State) -> Subscription<Message>>>,
    startup: Option<Box<dyn FnMut(&mut State, &mut UpdateContext<Message>)>>,
    shutdown: Option<Box<dyn FnMut(&mut State) -> Option<serde_json::Value>>>,
    close_requested: Option<Box<dyn FnMut(&mut State) -> bool>>,
    retained_painters: HashMap<u64, RetainedPainter<State>>,
    _message: PhantomData<Message>,
    _view: PhantomData<View>,
}

impl<State, Message, Project, Update, View>
    RunnableStatefulApp<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    /// Run this app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Run this app and return native runtime artifacts.
    pub fn run_with_artifacts(self) -> crate::gui_runtime::NativeGenericRunReport {
        let options = self.options.clone();
        crate::runtime::run_native_vello_runtime_with_artifacts(options, self.into_bridge())
    }

    /// Lower this app into the existing runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<Message> {
        AppBridge {
            state: self.state,
            project: self.project,
            update: self.update,
            runtime: Arc::new(AppRuntime::default()),
            animation: self.animation,
            frame_message: self.frame_message,
            subscriptions: self.subscriptions,
            startup: self.startup,
            shutdown: self.shutdown,
            close_requested: self.close_requested,
            retained_painters: self.retained_painters,
            subscriptions_started: false,
            startup_ran: false,
            _view: PhantomData,
        }
    }
}
