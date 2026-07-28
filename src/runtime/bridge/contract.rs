use super::RuntimeHostCapabilities;
use crate::runtime::{Command, RuntimeUpdateSnapshot, UiSurface, WindowEnvironment};
use std::sync::Arc;

/// Minimal host/runtime bridge for declarative message-driven surfaces.
///
/// Projection and update form the core host contract. Every other host concern
/// is explicitly enabled through [`RuntimeHostCapabilities`] and its focused
/// capability traits. Radiant caches that table when [`crate::runtime::SurfaceRuntime`]
/// is created, so capability availability stays stable for the runtime lifetime.
pub trait RuntimeBridge<Message>: Sized {
    /// Project the latest immutable UI surface snapshot.
    fn project_surface(&mut self) -> Arc<UiSurface<Message>>;

    /// Pull the latest immutable UI surface snapshot as an owned value.
    /// Owned-surface bridges can override this to avoid temporary [`Arc`] clones.
    fn pull_surface(&mut self) -> UiSurface<Message> {
        Arc::unwrap_or_clone(self.project_surface())
    }

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

    /// Observe a changed native environment before the next deferred projection.
    ///
    /// The default is intentionally a no-op so existing custom bridges remain
    /// source-compatible. Hosts that project environment-aware views can retain
    /// this immutable value and use it from their next `project_surface` call.
    fn window_environment_changed(&mut self, _environment: WindowEnvironment) {}

    /// Observe a changed native environment before the next deferred projection.
    ///
    /// This is the primary additive hook for custom bridges. It is a default
    /// no-op so existing implementations remain source-compatible.
    fn set_window_environment(&mut self, environment: WindowEnvironment) {
        self.on_window_environment_changed(environment);
    }

    /// Compatibility spelling for hosts that group lifecycle observers under
    /// `on_*` hooks. The runtime calls this forwarding hook.
    fn on_window_environment_changed(&mut self, environment: WindowEnvironment) {
        self.window_environment_changed(environment);
    }

    /// Declare the optional host capabilities owned by this bridge.
    ///
    /// The returned table is evaluated once by `SurfaceRuntime::new` and then
    /// cached. Implementations must therefore derive it from configuration that
    /// remains stable for the bridge lifetime.
    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        RuntimeHostCapabilities::new()
    }
}
