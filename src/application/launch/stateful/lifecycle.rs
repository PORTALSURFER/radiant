use super::StatefulAppWithView;
use crate::application::{
    AppAnimation, AppBridgeLifecycle, AppCloseRequested, AppFrameMessage, AppScroll, AppShortcuts,
    AppShutdown, AppStartup, AppSubscriptions, RetainedPainter, TransientOverlayActivity,
    TransientOverlayPainter,
};
use crate::{
    application::{IntoView, Subscription, UpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
};
use std::collections::HashMap;

pub(super) struct StatefulLifecycle<State, Message> {
    pub(super) animation: Option<AppAnimation<State>>,
    pub(super) frame_message: Option<AppFrameMessage<Message>>,
    pub(super) subscriptions: Option<AppSubscriptions<State, Message>>,
    pub(super) shortcuts: Option<AppShortcuts<State, Message>>,
    pub(super) scroll: Option<AppScroll<State, Message>>,
    pub(super) startup: Option<AppStartup<State, Message>>,
    pub(super) shutdown: Option<AppShutdown<State>>,
    pub(super) close_requested: Option<AppCloseRequested<State>>,
    pub(super) retained_painters: HashMap<u64, RetainedPainter<State>>,
    pub(super) transient_overlay_activity: Option<TransientOverlayActivity<State>>,
    pub(super) transient_overlay: Option<TransientOverlayPainter<State>>,
}

impl<State, Message> Default for StatefulLifecycle<State, Message> {
    fn default() -> Self {
        Self {
            animation: None,
            frame_message: None,
            subscriptions: None,
            shortcuts: None,
            scroll: None,
            startup: None,
            shutdown: None,
            close_requested: None,
            retained_painters: HashMap::new(),
            transient_overlay_activity: None,
            transient_overlay: None,
        }
    }
}

impl<State, Message> StatefulLifecycle<State, Message> {
    pub(super) fn into_bridge_lifecycle(self) -> AppBridgeLifecycle<State, Message> {
        AppBridgeLifecycle {
            animation: self.animation,
            frame_message: self.frame_message,
            subscriptions: self.subscriptions,
            shortcuts: self.shortcuts,
            scroll: self.scroll,
            startup: self.startup,
            shutdown: self.shutdown,
            close_requested: self.close_requested,
            retained_painters: self.retained_painters,
            transient_overlay_activity: self.transient_overlay_activity,
            transient_overlay: self.transient_overlay,
        }
    }
}

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
    /// Declare whether this app currently needs animation-driven frames.
    ///
    /// Pair this with [`Self::on_frame`] when each frame should update
    /// application state. Pair it with [`Self::transient_overlay`] alone when
    /// an overlay can derive motion from frame time and only needs paint-only
    /// redraws over the cached surface.
    pub fn animation(mut self, animation: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.lifecycle.animation = Some(Box::new(animation));
        self
    }

    /// Declare the message emitted for each active animation frame.
    ///
    /// This is optional for paint-only transient overlays. Use it only when
    /// frame ticks need to mutate host state or produce commands.
    pub fn on_frame(mut self, message: impl FnMut() -> Message + 'static) -> Self {
        self.lifecycle.frame_message = Some(Box::new(message));
        self
    }

    /// Declare app-level subscriptions started when the native runtime starts.
    pub fn subscriptions(
        mut self,
        subscriptions: impl FnMut(&mut State) -> Subscription<Message> + 'static,
    ) -> Self {
        self.lifecycle.subscriptions = Some(Box::new(subscriptions));
        self
    }

    /// Resolve app-level keyboard shortcuts before focused-widget key routing.
    pub fn shortcuts(
        mut self,
        shortcuts: impl FnMut(
            &mut State,
            Option<KeyPress>,
            KeyPress,
            FocusSurface,
        ) -> ShortcutResolution<Message>
        + 'static,
    ) -> Self {
        self.lifecycle.shortcuts = Some(Box::new(shortcuts));
        self
    }

    /// Observe runtime-owned scroll movement before the next paint.
    ///
    /// Mutating app state from this hook refreshes the projected surface
    /// automatically. Use the context only for follow-up work such as repaint
    /// signals, messages, focus changes, or background tasks.
    pub fn on_scroll(
        mut self,
        scroll: impl FnMut(&mut State, crate::runtime::ScrollUpdate, &mut UpdateContext<Message>)
        + 'static,
    ) -> Self {
        self.lifecycle.scroll = Some(Box::new(scroll));
        self
    }

    /// Register a startup hook.
    pub fn on_startup(
        mut self,
        startup: impl FnMut(&mut State, &mut UpdateContext<Message>) + 'static,
    ) -> Self {
        self.lifecycle.startup = Some(Box::new(startup));
        self
    }

    /// Register a shutdown artifact hook.
    pub fn on_shutdown(
        mut self,
        shutdown: impl FnMut(&mut State) -> Option<serde_json::Value> + 'static,
    ) -> Self {
        self.lifecycle.shutdown = Some(Box::new(shutdown));
        self
    }

    /// Register a close-request hook. Return `true` to close.
    pub fn on_close_requested(
        mut self,
        close_requested: impl FnMut(&mut State) -> bool + 'static,
    ) -> Self {
        self.lifecycle.close_requested = Some(Box::new(close_requested));
        self
    }
}
