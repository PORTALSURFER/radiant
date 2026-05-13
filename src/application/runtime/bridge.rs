use super::subscription::spawn_subscription;
use super::*;
use crate::{application::IntoView, runtime::Command};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

mod adapter;

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
