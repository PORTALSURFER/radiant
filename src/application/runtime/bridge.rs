use super::*;
use super::{subscription::spawn_subscription, threading::spawn_runtime_thread};
use crate::{
    application::IntoView,
    gui::{
        focus::FocusSurface, input::KeyPress, paint::PaintFrame as GuiPaintFrame,
        repaint::RepaintSignal, shortcuts::ShortcutResolution, types::Rect,
    },
    layout::Vector2,
    runtime::{Command, RuntimeBridge, UiSurface},
    widgets::RetainedSurfaceDescriptor,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, thread, time::Duration};

pub(in crate::application) struct AppBridge<State, Message, Project, Update, View> {
    pub(in crate::application) state: State,
    pub(in crate::application) project: Project,
    pub(in crate::application) update: Update,
    pub(in crate::application) runtime: Arc<AppRuntime<Message>>,
    pub(in crate::application) animation: Option<AppAnimation<State>>,
    pub(in crate::application) frame_message: Option<AppFrameMessage<Message>>,
    pub(in crate::application) subscriptions: Option<AppSubscriptions<State, Message>>,
    pub(in crate::application) shortcuts: Option<AppShortcuts<State, Message>>,
    pub(in crate::application) scroll: Option<AppScroll<State, Message>>,
    pub(in crate::application) startup: Option<AppStartup<State, Message>>,
    pub(in crate::application) shutdown: Option<AppShutdown<State>>,
    pub(in crate::application) close_requested: Option<AppCloseRequested<State>>,
    pub(in crate::application) retained_painters: HashMap<u64, RetainedPainter<State>>,
    pub(in crate::application) subscriptions_started: bool,
    pub(in crate::application) startup_ran: bool,
    pub(in crate::application) _view: PhantomData<View>,
}

/// Lifecycle hooks carried from application launch builders into the runtime bridge.
pub(in crate::application) struct AppBridgeLifecycle<State, Message> {
    /// Animation activity callback.
    pub(in crate::application) animation: Option<AppAnimation<State>>,
    /// Frame-message callback.
    pub(in crate::application) frame_message: Option<AppFrameMessage<Message>>,
    /// App-level subscription factory.
    pub(in crate::application) subscriptions: Option<AppSubscriptions<State, Message>>,
    /// App-level shortcut resolver.
    pub(in crate::application) shortcuts: Option<AppShortcuts<State, Message>>,
    /// Runtime scroll observer.
    pub(in crate::application) scroll: Option<AppScroll<State, Message>>,
    /// Startup hook.
    pub(in crate::application) startup: Option<AppStartup<State, Message>>,
    /// Shutdown artifact hook.
    pub(in crate::application) shutdown: Option<AppShutdown<State>>,
    /// Close-request hook.
    pub(in crate::application) close_requested: Option<AppCloseRequested<State>>,
    /// Retained-surface painters keyed by descriptor key.
    pub(in crate::application) retained_painters: HashMap<u64, RetainedPainter<State>>,
}

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>),
    View: IntoView<Message>,
{
    /// Build an app bridge from host state, projection, reducer, and lifecycle hooks.
    pub(in crate::application) fn new(
        state: State,
        project: Project,
        update: Update,
        lifecycle: AppBridgeLifecycle<State, Message>,
    ) -> Self {
        Self {
            state,
            project,
            update,
            runtime: Arc::new(AppRuntime::default()),
            animation: lifecycle.animation,
            frame_message: lifecycle.frame_message,
            subscriptions: lifecycle.subscriptions,
            shortcuts: lifecycle.shortcuts,
            scroll: lifecycle.scroll,
            startup: lifecycle.startup,
            shutdown: lifecycle.shutdown,
            close_requested: lifecycle.close_requested,
            retained_painters: lifecycle.retained_painters,
            subscriptions_started: false,
            startup_ran: false,
            _view: PhantomData,
        }
    }

    fn run_update(&mut self, message: Message) -> Command<Message> {
        let mut context = UpdateContext::default();
        (self.update)(&mut self.state, message, &mut context);
        context.into_command()
    }

    fn run_startup_once(&mut self) {
        if self.startup_ran {
            return;
        }
        if let Some(startup) = self.startup.as_mut() {
            let mut context = UpdateContext::default();
            startup(&mut self.state, &mut context);
            self.runtime.enqueue_command(context.into_command());
        }
        self.startup_ran = true;
    }

    fn start_subscriptions_once(&mut self)
    where
        Message: Send + 'static,
    {
        if self.subscriptions_started {
            return;
        }
        if let Some(subscriptions) = self.subscriptions.as_mut() {
            spawn_subscription(
                Arc::downgrade(&self.runtime),
                subscriptions(&mut self.state),
            );
        }
        self.subscriptions_started = true;
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

    fn scroll_updated(&mut self, update: crate::runtime::ScrollUpdate) -> Option<Command<Message>> {
        let scroll = self.scroll.as_mut()?;
        let mut context = UpdateContext::default();
        scroll(&mut self.state, update, &mut context);
        Some(context.into_command())
    }

    fn resolve_key_press(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        self.shortcuts
            .as_mut()
            .map(|shortcuts| shortcuts(&mut self.state, pending_chord, press, focus))
            .unwrap_or_else(ShortcutResolution::unhandled)
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.install_repaint(signal);
        self.run_startup_once();
        self.start_subscriptions_once();
    }

    fn schedule_message(&mut self, delay: Duration, message: Message) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        spawn_runtime_thread("radiant-delayed-message", move || {
            if !delay.is_zero() {
                thread::sleep(delay);
            }
            if let Some(runtime) = runtime.upgrade() {
                let _ = runtime.enqueue(message);
            }
        })
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
        spawn_runtime_thread(format!("radiant-task-{name}"), move || {
            let message = work();
            if let Some(runtime) = runtime.upgrade() {
                let _ = runtime.enqueue(message);
            }
        })
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
