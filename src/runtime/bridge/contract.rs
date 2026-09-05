use super::RuntimeHostCapabilities;
use crate::application::ApplicationEnvironment;
use crate::runtime::{Command, RuntimeUpdateSnapshot, UiSurface, WindowEnvironment};
use crate::{gui::types::Rect, layout::NodeId};
use std::sync::Arc;

/// Maximum number of changed roots accepted in one exact update.
pub const MAX_EXACT_CHANGED_ROOTS: usize = 64;
/// Maximum total path components accepted by one exact update.
pub const MAX_EXACT_CHANGED_ROOT_PATH_COMPONENTS: usize = 256;

/// Minimal host/runtime bridge for declarative message-driven surfaces.
///
/// Projection and update form the core host contract. Every other host concern
/// is explicitly enabled through [`RuntimeHostCapabilities`] and its focused
/// capability traits. Radiant caches that table when [`crate::runtime::SurfaceRuntime`]
/// is created, so capability availability stays stable for the runtime lifetime.
pub trait RuntimeBridge<Message>: Sized {
    /// Return the current application presentation snapshot, when the host
    /// owns one independently of its projected surface.
    ///
    /// This boundary is intentionally separate from [`Self::pull_surface`]:
    /// runtimes can cheaply detect application-environment invalidation before
    /// deciding whether a requested paint-only refresh needs projection.
    fn application_environment(&mut self) -> Option<ApplicationEnvironment> {
        None
    }

    /// Project the latest immutable UI surface snapshot.
    fn project_surface(&mut self) -> Arc<UiSurface<Message>>;

    /// Pull the latest immutable UI surface snapshot as an owned value.
    /// Owned-surface bridges can override this to avoid temporary [`Arc`] clones.
    fn pull_surface(&mut self) -> UiSurface<Message> {
        Arc::unwrap_or_clone(self.project_surface())
    }

    /// Pull one complete successor surface, optionally carrying an exact
    /// changed-root authority supplied by an opt-in host.
    ///
    /// Existing bridges default to [`SurfaceUpdate::Full`]. The runtime only
    /// consumes [`SurfaceUpdate::ExactChangedRoots`] after validating every
    /// echoed fence and the complete changed-root superset.
    fn pull_surface_update(&mut self, _request: SurfaceRefreshRequest) -> SurfaceUpdate<Message> {
        SurfaceUpdate::Full(self.pull_surface())
    }

    /// Return exact authority for changed-root evidence supplied by this
    /// bridge.  Bridges that cannot prove ownership of the evidence retain
    /// the correctness-first full refresh path.
    fn surface_update_provider_authority(&self) -> Option<SurfaceUpdateProviderAuthority> {
        None
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

/// Runtime authority supplied to one bridge surface-update request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceRefreshRequest {
    /// Identity of the owning runtime instance.
    pub runtime_identity: u64,
    /// Monotonic refresh request revision.
    pub request_revision: u64,
    /// Generation of the currently admitted surface.
    pub active_surface_generation: u64,
    /// Current layout viewport.
    pub viewport: Rect,
    /// Current native window environment.
    pub window_environment: WindowEnvironment,
    /// Provider authority checked for optional exact updates.
    pub expected_provider_authority: Option<SurfaceUpdateProviderAuthority>,
}

/// Exact owner and revision authority for one bridge-provided update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceUpdateProviderAuthority {
    /// Stable owner identity for the provider of exact evidence.
    pub owner: u64,
    /// Checked revision of the provider evidence.
    pub checked_revision: u64,
}

/// One complete bridge update candidate.
pub enum SurfaceUpdate<Message> {
    /// The bridge does not provide exact changed-root evidence.
    Full(UiSurface<Message>),
    /// A complete successor plus bounded exact changed-root authority.
    ExactChangedRoots(ExactChangedRoots<Message>),
}

/// An opt-in exact changed-root update candidate.
pub struct ExactChangedRoots<Message> {
    /// Complete successor surface. The runtime may retain disjoint old roots.
    pub surface: UiSurface<Message>,
    /// Echoed runtime identity.
    pub runtime_identity: u64,
    /// Echoed request revision.
    pub request_revision: u64,
    /// Echoed active surface generation.
    pub active_surface_generation: u64,
    /// Echoed viewport.
    pub viewport: Rect,
    /// Echoed native environment.
    pub window_environment: WindowEnvironment,
    /// Provider authority checked by the runtime.
    pub provider_authority: Option<SurfaceUpdateProviderAuthority>,
    /// Complete bounded superset of changed widget leaves.
    pub changed_roots: Vec<ExactChangedRoot>,
}

/// One exact changed widget leaf and its path in the complete successor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactChangedRoot {
    /// Stable widget identity at `child_path`.
    pub node_id: NodeId,
    /// Root-relative child path to the widget leaf.
    pub child_path: Vec<usize>,
}
