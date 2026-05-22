use super::StatefulAppWithView;
use crate::{
    application::{IntoView, Subscription, UpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::AuxiliaryWindow,
};

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

    /// Observe native operating-system file drag/drop events.
    pub fn on_native_file_drop(
        mut self,
        drop: impl FnMut(&mut State, crate::runtime::NativeFileDrop, &mut UpdateContext<Message>)
        + 'static,
    ) -> Self {
        self.lifecycle.native_file_drop = Some(Box::new(drop));
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

    /// Project auxiliary top-level OS windows from app state.
    pub fn auxiliary_windows(
        mut self,
        windows: impl FnMut(&mut State) -> Vec<AuxiliaryWindow<Message>> + 'static,
    ) -> Self {
        self.lifecycle.auxiliary_windows = Some(Box::new(windows));
        self
    }
}
