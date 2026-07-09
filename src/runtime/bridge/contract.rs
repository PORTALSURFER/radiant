use super::{AuxiliaryWindow, RuntimeAnimationActivity};
use crate::gui::{
    focus::FocusSurface,
    input::KeyPress,
    paint::PaintFrame,
    repaint::RepaintSignal,
    shortcuts::ShortcutResolution,
    types::{Rect, Vector2},
};
use crate::runtime::{
    BusinessMessageSink, Command, NativeFileDrop, NativeFileOpen, NativeFrameDiagnostics,
    PaintPrimitive, PlatformCompletion, PlatformRequest, PlatformServiceFallback,
    RuntimeDiagnostics, RuntimeUpdateSnapshot, ScrollUpdate, TaskPriority, TransientOverlayContext,
    UiSurface,
};
use crate::widgets::RetainedSurfaceDescriptor;
use std::{sync::Arc, time::Duration};

/// Generic host/runtime bridge for declarative message-driven surfaces.
///
/// The single explicit adapter contract for custom hosts.
pub trait RuntimeBridge<Message> {
    // Surface projection.

    /// Project the latest immutable UI surface snapshot.
    fn project_surface(&mut self) -> Arc<UiSurface<Message>>;

    /// Pull the latest immutable UI surface snapshot as an owned value.
    /// Owned-surface bridges can override this to avoid temporary [`Arc`] clones.
    fn pull_surface(&mut self) -> UiSurface<Message> {
        Arc::unwrap_or_clone(self.project_surface())
    }

    /// Project additional top-level OS windows owned by this application runtime.
    fn project_auxiliary_windows(&mut self) -> Vec<AuxiliaryWindow<Message>> {
        Vec::new()
    }

    // State updates and input policy.

    /// Reduce one host-defined message into application state.
    fn reduce_message(&mut self, _message: Message) {}

    /// Update application state and return runtime-visible follow-up work.
    fn update(&mut self, message: Message) -> Command<Message> {
        self.reduce_message(message);
        Command::none()
    }

    /// Update state with a read-only snapshot of runtime-owned input state.
    fn update_with_runtime(
        &mut self,
        message: Message,
        _snapshot: RuntimeUpdateSnapshot,
    ) -> Command<Message> {
        self.update(message)
    }

    /// Observe runtime-owned scroll movement and optionally return follow-up work.
    fn scroll_updated(&mut self, _update: ScrollUpdate) -> Option<Command<Message>> {
        None
    }

    /// Handle a native file drag/drop event with available target metadata.
    fn native_file_drop(&mut self, _drop: NativeFileDrop) -> Command<Message> {
        Command::none()
    }

    /// Handle a native operating-system request to open documents or files.
    fn native_file_open(&mut self, _open: NativeFileOpen) -> Command<Message> {
        Command::none()
    }

    /// Resolve one keyboard press against a host-owned shortcut catalog.
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        ShortcutResolution::unhandled()
    }

    // Runtime scheduling and host work.

    /// Install a repaint signal for host-owned background work.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {}

    /// Queue a host-defined message from runtime-managed background work.
    /// The default returns `false` so custom bridges keep full control.
    fn schedule_message(&mut self, _delay: Duration, _message: Message) -> bool {
        false
    }

    /// Spawn message-producing host work through the bridge-owned app runtime.
    fn spawn_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        false
    }

    /// Spawn host work that may emit multiple messages before it completes.
    fn spawn_streaming_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        false
    }

    /// Spawn host work whose intermediate messages may be coalesced.
    /// Bridges must override this; the default refuses instead of forwarding to
    /// ordered streaming, where `emit_latest` would degrade to ordinary emits.
    fn spawn_latest_streaming_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        false
    }

    // Platform services.

    /// Request a host-visible platform service such as a file picker or dialog.
    fn request_platform_service(
        &mut self,
        request: PlatformRequest,
        on_completed: PlatformCompletion<Message>,
    ) -> Result<(), PlatformServiceFallback<Message>> {
        Err(Box::new((request, on_completed)))
    }

    // Runtime-owned queues.

    /// Drain commands delivered by app startup hooks or bridge-owned runtime work.
    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        Vec::new()
    }

    /// Drain commands into caller-owned scratch storage.
    /// The default preserves bridges that implement [`Self::take_runtime_commands`].
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        commands.extend(self.take_runtime_commands());
    }

    /// Drain messages delivered by app-level tasks, timers, or subscriptions.
    fn take_runtime_messages(&mut self) -> Vec<Message> {
        Vec::new()
    }

    /// Drain messages into caller-owned scratch storage.
    /// The default preserves bridges that implement [`Self::take_runtime_messages`].
    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        messages.extend(self.take_runtime_messages());
    }

    /// Drain one controller pass of messages and report whether more remain.
    /// Coalesced lanes should keep undispatched messages bridge-owned so later
    /// runtime work can replace them before dispatch.
    fn drain_runtime_message_batch_into(
        &mut self,
        messages: &mut Vec<Message>,
        _max_messages: usize,
    ) -> bool {
        self.drain_runtime_messages_into(messages);
        false
    }

    // Animation policy.

    /// Return whether the host currently needs animation-driven redraws.
    fn needs_animation(&mut self) -> bool {
        false
    }

    /// Return the kind of animation work currently needed.
    /// Bridges overriding only [`Self::needs_animation`] use frame messages.
    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        if self.needs_animation() {
            RuntimeAnimationActivity::frame_messages()
        } else {
            RuntimeAnimationActivity::idle()
        }
    }

    /// Queue one host-defined animation-frame message if the host is animating.
    fn queue_animation_frame(&mut self) -> bool {
        false
    }

    // Retained and transient rendering hooks.

    /// Render a host-retained custom surface into backend-neutral paint data.
    fn render_retained_surface(
        &mut self,
        _descriptor: RetainedSurfaceDescriptor,
        _rect: Rect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        None
    }

    /// Return whether [`Self::paint_transient_overlay`] can emit primitives.
    /// The default preserves existing custom overlay painters.
    fn has_transient_overlay_painter(&self) -> bool {
        true
    }

    /// Paint lightweight transient primitives over the cached scene.
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
    }

    // Diagnostics and lifecycle.

    /// Return whether [`Self::observe_frame_diagnostics`] consumes frame diagnostics.
    ///
    /// Native runners cache this capability when they are created, so implementations
    /// must return a stable value for the lifetime of the bridge. The default preserves
    /// existing custom diagnostics observers.
    fn has_frame_diagnostics_observer(&self) -> bool {
        true
    }

    /// Observe structured diagnostics for one native presentation frame.
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}

    /// Return application-runtime diagnostics contributed by this bridge.
    fn runtime_diagnostics(&self) -> RuntimeDiagnostics {
        RuntimeDiagnostics::default()
    }

    /// Lifecycle hook fired when the native runtime exits.
    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        None
    }

    /// Return whether the runtime should continue closing the active window.
    fn close_requested(&mut self) -> bool {
        true
    }
}
