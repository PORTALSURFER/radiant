use super::RuntimeHostCapabilities;
use crate::runtime::{Command, RuntimeUpdateSnapshot, UiSurface};
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

    /// Declare the optional host capabilities owned by this bridge.
    ///
    /// The returned table is evaluated once by `SurfaceRuntime::new` and then
    /// cached. Implementations must therefore derive it from configuration that
    /// remains stable for the bridge lifetime.
    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        RuntimeHostCapabilities::new()
    }
}
