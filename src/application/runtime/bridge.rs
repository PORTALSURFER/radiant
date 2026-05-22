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
    pub(in crate::application) native_file_drop: Option<AppNativeFileDrop<State, Message>>,
    pub(in crate::application) startup: Option<AppStartup<State, Message>>,
    pub(in crate::application) shutdown: Option<AppShutdown<State>>,
    pub(in crate::application) close_requested: Option<AppCloseRequested<State>>,
    pub(in crate::application) auxiliary_windows: Option<AppAuxiliaryWindows<State, Message>>,
    pub(in crate::application) retained_painters: HashMap<u64, RetainedPainter<State>>,
    pub(in crate::application) transient_overlay_activity: Option<TransientOverlayActivity<State>>,
    pub(in crate::application) transient_overlay: Option<TransientOverlayPainter<State>>,
    pub(in crate::application) pending_animation_frame_activity: Option<bool>,
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
    pub(in crate::application) native_file_drop: Option<AppNativeFileDrop<State, Message>>,
    /// Startup hook.
    pub(in crate::application) startup: Option<AppStartup<State, Message>>,
    /// Shutdown artifact hook.
    pub(in crate::application) shutdown: Option<AppShutdown<State>>,
    /// Close-request hook.
    pub(in crate::application) close_requested: Option<AppCloseRequested<State>>,
    /// Auxiliary top-level native windows projected from app state.
    pub(in crate::application) auxiliary_windows: Option<AppAuxiliaryWindows<State, Message>>,
    /// Retained-surface painters keyed by descriptor key.
    pub(in crate::application) retained_painters: HashMap<u64, RetainedPainter<State>>,
    /// Transient-overlay frame activity callback.
    pub(in crate::application) transient_overlay_activity: Option<TransientOverlayActivity<State>>,
    /// Lightweight frame-time overlay painter.
    pub(in crate::application) transient_overlay: Option<TransientOverlayPainter<State>>,
}

impl<State, Message> Default for AppBridgeLifecycle<State, Message> {
    fn default() -> Self {
        Self {
            animation: None,
            frame_message: None,
            subscriptions: None,
            shortcuts: None,
            scroll: None,
            native_file_drop: None,
            startup: None,
            shutdown: None,
            close_requested: None,
            auxiliary_windows: None,
            retained_painters: HashMap::new(),
            transient_overlay_activity: None,
            transient_overlay: None,
        }
    }
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
            native_file_drop: lifecycle.native_file_drop,
            startup: lifecycle.startup,
            shutdown: lifecycle.shutdown,
            close_requested: lifecycle.close_requested,
            auxiliary_windows: lifecycle.auxiliary_windows,
            retained_painters: lifecycle.retained_painters,
            transient_overlay_activity: lifecycle.transient_overlay_activity,
            transient_overlay: lifecycle.transient_overlay,
            pending_animation_frame_activity: None,
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

    pub(in crate::application) fn project_app_auxiliary_windows(
        &mut self,
    ) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
        self.auxiliary_windows
            .as_mut()
            .map(|project| project(&mut self.state))
            .unwrap_or_default()
    }
}
