impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
    State: 'static,
{
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
        self.shortcuts = Some(Box::new(shortcuts));
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
    pub fn on_close_requested(
        mut self,
        close_requested: impl FnMut(&mut State) -> bool + 'static,
    ) -> Self {
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
