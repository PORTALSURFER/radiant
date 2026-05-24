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
    Command, NativeFileDrop, NativeFrameDiagnostics, PaintPrimitive, PlatformCompletion,
    PlatformRequest, PlatformServiceFallback, ScrollUpdate, TransientOverlayContext, UiSurface,
};
use crate::widgets::RetainedSurfaceDescriptor;
use std::{sync::Arc, time::Duration};

/// Generic host/runtime bridge for declarative message-driven surfaces.
///
/// The host projects one immutable [`UiSurface`] snapshot per frame and reduces
/// host-defined messages emitted by widgets back into owned application state.
/// The trait is intentionally broad because it is the single explicit adapter
/// contract for custom hosts, but its default hooks are grouped by responsibility:
/// surface projection, state updates and input policy, runtime scheduling and
/// host work, platform services, runtime-owned queues, animation policy, retained
/// and transient rendering hooks, diagnostics, and lifecycle.
pub trait RuntimeBridge<Message> {
    // Surface projection.

    /// Project the latest immutable UI surface snapshot.
    fn project_surface(&mut self) -> Arc<UiSurface<Message>>;

    /// Pull the latest immutable UI surface snapshot as an owned value.
    ///
    /// Bridges that can project owned surfaces directly should override this
    /// method so runtime refreshes do not allocate a temporary [`Arc`] or clone
    /// a shared surface snapshot.
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
    ///
    /// Existing hosts can keep implementing [`RuntimeBridge::reduce_message`].
    /// Hosts that need command dispatch can override this method and return
    /// [`Command`] values without moving side-effect ownership into Radiant.
    fn update(&mut self, message: Message) -> Command<Message> {
        self.reduce_message(message);
        Command::none()
    }

    /// Observe runtime-owned scroll movement and optionally return follow-up work.
    ///
    /// Hosts with logical virtual-list windows can use this hook to synchronize
    /// application-owned viewport state from the runtime scroll offset while
    /// hosts that only need runtime-owned scrolling can rely on the default
    /// no-op implementation.
    fn scroll_updated(&mut self, _update: ScrollUpdate) -> Option<Command<Message>> {
        None
    }

    /// Handle a native operating-system file drag/drop event.
    ///
    /// Native backends populate the last known pointer position and widget
    /// target when available. Hosts can map hover, cancel, and drop phases to
    /// app messages or repaint commands while backends that do not use native
    /// file drops can rely on the default no-op implementation.
    fn native_file_drop(&mut self, _drop: NativeFileDrop) -> Command<Message> {
        Command::none()
    }

    /// Resolve one keyboard press against a host-owned shortcut catalog.
    ///
    /// The runtime supplies the pending chord, normalized keypress, and current
    /// logical focus bucket. Hosts can return a message to reduce immediately,
    /// mark the press handled, or carry a pending chord into the next keypress.
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        ShortcutResolution::unhandled()
    }

    // Runtime scheduling and host work.

    /// Install a repaint signal that host-owned background work can use to wake
    /// the native runtime after asynchronous state changes.
    ///
    /// Declarative hosts that do not run background work can rely on the
    /// default no-op implementation. Hosts that do should store this signal and
    /// forward it to their worker systems rather than depending on backend
    /// internals.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {}

    /// Queue a host-defined message from runtime-managed background work.
    ///
    /// The default returns `false` so low-level custom bridges keep full control.
    /// Application-builder bridges override this to support delayed messages,
    /// background tasks, and subscriptions through the unified app API.
    fn schedule_message(&mut self, _delay: Duration, _message: Message) -> bool {
        false
    }

    /// Spawn message-producing host work through the bridge-owned app runtime.
    ///
    /// Application-builder bridges run this work on runtime-managed business
    /// threads so the UI/event/render owner does not block on host work. Custom
    /// bridges may return `false` if they intentionally own a different
    /// scheduling policy.
    fn spawn_message_task(
        &mut self,
        _name: &'static str,
        _work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        false
    }

    // Platform services.

    /// Request a host-visible platform service such as a file picker or dialog.
    ///
    /// Native adapters or application hosts that own platform integration can
    /// override this method and dispatch the completion callback when the
    /// request finishes. The default returns `false`, which causes the runtime
    /// to report an unsupported platform service back through the callback.
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
    ///
    /// Implementations can override this to avoid allocating a new vector for
    /// every runtime drain. The default preserves compatibility with bridges
    /// that only implement [`Self::take_runtime_commands`].
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        commands.extend(self.take_runtime_commands());
    }

    /// Drain messages delivered by app-level tasks, timers, or subscriptions.
    fn take_runtime_messages(&mut self) -> Vec<Message> {
        Vec::new()
    }

    /// Drain messages into caller-owned scratch storage.
    ///
    /// Implementations can override this to avoid allocating a new vector for
    /// every runtime drain. The default preserves compatibility with bridges
    /// that only implement [`Self::take_runtime_messages`].
    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        messages.extend(self.take_runtime_messages());
    }

    // Animation policy.

    /// Return whether the host currently needs animation-driven redraws.
    ///
    /// Generic declarative hosts can stay repaint-driven by using the default
    /// `false`. Hosts with active playback, motion, or transient animation can
    /// opt into frame-interval redraws without making the native runtime poll
    /// while the UI is idle.
    fn needs_animation(&mut self) -> bool {
        false
    }

    /// Return the kind of animation work currently needed.
    ///
    /// The default preserves existing custom bridges: a bridge that overrides
    /// only [`Self::needs_animation`] is treated as frame-message animation so
    /// the native runtime still calls [`Self::queue_animation_frame`].
    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        if self.needs_animation() {
            RuntimeAnimationActivity::frame_messages()
        } else {
            RuntimeAnimationActivity::idle()
        }
    }

    /// Queue one host-defined animation-frame message if the host is currently
    /// animating.
    ///
    /// This is intentionally separate from [`Self::needs_animation`] so native
    /// backends can poll animation activity without producing queued work or
    /// repaint wakeups as a side effect.
    fn queue_animation_frame(&mut self) -> bool {
        false
    }

    // Retained and transient rendering hooks.

    /// Render a host-retained custom surface into backend-neutral paint data.
    ///
    /// Generic widgets can reserve custom paint through a retained canvas while
    /// keeping the actual application-specific rendering state host-owned. The
    /// runtime supplies the retained descriptor, assigned canvas rectangle, and
    /// current viewport. Hosts that do not use retained custom surfaces can rely
    /// on the default `None` implementation.
    fn render_retained_surface(
        &mut self,
        _descriptor: RetainedSurfaceDescriptor,
        _rect: Rect,
        _viewport: Vector2,
    ) -> Option<PaintFrame> {
        None
    }

    /// Paint transient overlay primitives for the current presentation frame.
    ///
    /// This hook is for lightweight visuals that need frame-rate updates
    /// without changing layout or refreshing the declarative surface snapshot:
    /// drag previews, playheads, tooltip affordances, spectrogram cursors, and
    /// other overlays that can be anchored to the latest
    /// [`SurfacePaintPlan`](crate::runtime::SurfacePaintPlan).
    /// Native backends render these primitives over the cached scene and GPU
    /// surfaces, so implementations should keep the output small and avoid
    /// mutating structural application state here.
    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
    }

    // Diagnostics and lifecycle.

    /// Observe structured diagnostics for one native presentation frame.
    ///
    /// Native renderers call this after a successful present. Hosts can use it
    /// for perf counters, regression probes, or app-owned telemetry without
    /// scraping backend logs. The default no-op keeps simple bridges silent.
    fn observe_frame_diagnostics(&mut self, _diagnostics: NativeFrameDiagnostics) {}

    /// Lifecycle hook fired when the native runtime exits.
    ///
    /// Hosts can return a structured artifact for diagnostics, telemetry, or
    /// shutdown validation. The generic runtime treats the payload as opaque so
    /// application-specific shutdown phases remain host-owned.
    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        None
    }

    /// Return whether the runtime should continue closing the active window.
    fn close_requested(&mut self) -> bool {
        true
    }
}
