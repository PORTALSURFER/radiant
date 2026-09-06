use super::StatefulAppWithView;
use crate::{
    application::{IntoView, Presentation, Subscription, UiUpdateContext},
    gui::{focus::FocusSurface, input::KeyPress, shortcuts::ShortcutResolution},
    runtime::AuxiliaryWindow,
};

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    /// Advanced lifecycle hook for animation-driven native frames.
    ///
    /// Prefer [`crate::application::Scene::frame_clock`] or
    /// [`Self::presentation`] with [`crate::application::FrameClock`] for
    /// normal root-scoped presentation. Use this lower-level hook when a host
    /// needs to wire runtime frame activity directly. Pair it with
    /// [`Self::on_frame`] when each frame should update application state.
    /// Pair it with [`Self::transient_overlay`] only for custom lifecycle
    /// wiring where an overlay derives motion from frame time and needs
    /// paint-only redraws over the cached surface.
    pub fn animation(mut self, animation: impl FnMut(&mut State) -> bool + 'static) -> Self {
        self.lifecycle.animation = Some(Box::new(animation));
        self
    }

    /// Advanced lifecycle hook for messages emitted on active animation frames.
    ///
    /// Prefer [`crate::application::Scene::frame_clock`] or
    /// [`Self::presentation`] with [`crate::application::FrameClock`] for
    /// ordinary frame-message presentation. This lower-level hook remains
    /// public for hosts that need direct runtime lifecycle control. It is
    /// optional for paint-only transient overlays; use it only when frame ticks
    /// need to mutate host state or produce commands.
    pub fn on_frame(mut self, message: impl FnMut() -> Message + 'static) -> Self {
        self.lifecycle.frame_message = Some(Box::new(message));
        self
    }

    /// Declare typed presentation hooks such as frame clocks and paint-only overlays.
    pub fn presentation(mut self, presentation: Presentation<State, Message>) -> Self {
        presentation.apply_to_lifecycle(&mut self.lifecycle);
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

    /// Register one semantic command mapper and a read-only current-scope projection.
    ///
    /// The projection supplies active scopes from current application state and runtime
    /// focus. Clone unchanged scope snapshots; replace them when captured context changes.
    /// Mapped invocations enter the ordinary application reducer. Only an unmatched input
    /// permits subsequent legacy shortcut handling.
    pub fn commands<Context: 'static>(
        mut self,
        registry: crate::application::CommandRegistry,
        project: impl Fn(
            &State,
            crate::application::CommandFocus,
        ) -> crate::application::CommandSnapshot<Context>
        + 'static,
        dispatcher: crate::application::CommandDispatcher<Context, Message>,
    ) -> Self {
        self.lifecycle.declarative_command_router = None;
        self.lifecycle.command_router = Some(Box::new(move |state, request, focus| {
            let snapshot = project(state, focus);
            match request {
                crate::application::CommandRequest::Input(input) => {
                    dispatcher.input(&registry, &snapshot.scopes, &snapshot.keymap, input)
                }
                crate::application::CommandRequest::Target(target, source) => {
                    dispatcher.target(&registry, &snapshot.scopes, target, source)
                }
            }
        }));
        self
    }

    /// Register one mapper for command scopes attached to declarative views.
    ///
    /// The runtime selects the focused ancestry and current eligible layers from
    /// its committed tree. Application code does not reconstruct that scope list.
    pub fn command_registry<Context: 'static>(
        mut self,
        registry: crate::application::CommandRegistry,
        dispatcher: crate::application::CommandDispatcher<Context, Message>,
    ) -> Self {
        self.lifecycle.command_router = None;
        self.lifecycle.declarative_command_router = Some(crate::application::CommandService::new(
            registry,
            dispatcher,
            crate::application::Keymap::new(),
        ));
        self
    }

    /// Project persisted keymap overrides for the declarative command registry.
    pub fn command_keymap(
        mut self,
        project: impl Fn(&State) -> crate::application::Keymap + 'static,
    ) -> Self {
        self.lifecycle.command_keymap = Some(Box::new(project));
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
        scroll: impl FnMut(&mut State, crate::runtime::ScrollUpdate, &mut UiUpdateContext<Message>)
        + 'static,
    ) -> Self {
        self.lifecycle.scroll = Some(Box::new(scroll));
        self
    }

    /// Observe native operating-system file drag/drop events.
    pub fn on_native_file_drop(
        mut self,
        drop: impl FnMut(&mut State, crate::runtime::NativeFileDrop, &mut UiUpdateContext<Message>)
        + 'static,
    ) -> Self {
        self.lifecycle.native_file_drop = Some(Box::new(drop));
        self
    }

    /// Observe native operating-system document/file-open requests.
    pub fn on_native_file_open(
        mut self,
        open: impl FnMut(&mut State, crate::runtime::NativeFileOpen, &mut UiUpdateContext<Message>)
        + 'static,
    ) -> Self {
        self.lifecycle.native_file_open = Some(Box::new(open));
        self
    }

    /// Observe the native main window regaining operating-system focus.
    pub fn on_native_focus_regained(
        mut self,
        focus_regained: impl FnMut(&mut State, &mut UiUpdateContext<Message>) + 'static,
    ) -> Self {
        self.lifecycle.native_focus_regained = Some(Box::new(focus_regained));
        self
    }

    /// Observe structured native frame diagnostics when the backend presents a frame.
    ///
    /// Register this only for opt-in diagnostics. Native backends use the presence
    /// of this hook as a hint to collect frame timing and routing metadata.
    pub fn on_native_frame_diagnostics(
        mut self,
        diagnostics: impl FnMut(&mut State, crate::runtime::NativeFrameDiagnostics) + 'static,
    ) -> Self {
        self.lifecycle.native_frame_diagnostics = Some(Box::new(diagnostics));
        self
    }

    /// Observe the locked-Winit IME adapter capabilities for each admitted
    /// native window.
    ///
    /// The callback is one-shot per admitted window and observational only.
    /// It does not participate in focus, composition, or input routing.
    pub fn on_native_ime_adapter_observation(
        mut self,
        observation: impl FnMut(&mut State, crate::runtime::NativeImeAdapterObservation) + 'static,
    ) -> Self {
        self.lifecycle.native_ime_adapter_observation = Some(Box::new(observation));
        self
    }

    /// Observe a backend-neutral fixed-cost profile after a successful native
    /// frame presentation.
    ///
    /// Delivery requires [`crate::runtime::ProfilingOptions::frame`] on the
    /// app's native frame options. The callback is ordered with the existing
    /// successful-presentation publication path and is independent of the
    /// compatibility diagnostics callback.
    pub fn on_frame_profile(
        mut self,
        profile: impl FnMut(&mut State, crate::runtime::FrameProfile) + 'static,
    ) -> Self {
        self.lifecycle.native_frame_profile = Some(Box::new(profile));
        self
    }

    /// Opt in to correlated asynchronous aggregate GPU timing after a
    /// successful native frame presentation.
    ///
    /// The public callback boundary is established independently of
    /// [`Self::on_frame_profile`]. The generic native primary and auxiliary
    /// runners can emit per-window samples when that window's frame profiling
    /// and this observer are enabled. Auxiliary samples use the existing parent
    /// handoff, with lifecycle fencing discarding stale or invalid completions.
    pub fn on_frame_gpu_timing(
        mut self,
        timing: impl FnMut(&mut State, crate::runtime::FrameGpuTimingSample) + 'static,
    ) -> Self {
        self.lifecycle.native_frame_gpu_timing = Some(Box::new(timing));
        self
    }

    /// Register a startup hook.
    pub fn on_startup(
        mut self,
        startup: impl FnMut(&mut State, &mut UiUpdateContext<Message>) + 'static,
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
