//! Shared GUI runtime host implementations.
//!
//! The runtime is responsible for the host integration lifecycle:
//! 1. open/create a window and platform event pump,
//! 2. pull app snapshots and submit them to `radiant` for frame build,
//! 3. schedule redraws when input, timers, or host events require updates,
//! 4. emit platform input into normalized runtime events.
//!
//! Render work is intentionally performed through feature-gated hooks in the
//! underlying backend (for example `gui-performance`) so release builds that do
//! not request profiling have no measurable instrumentation overhead.
//!
//! Native Vello exposes a generic [`crate::runtime::RuntimeBridge`] entrypoint
//! for reusable host applications.

pub(crate) mod native_vello;
mod options;
mod window_manifest;

/// Generic result envelope returned by native runtime entry points.
///
/// The artifact payload is supplied by the concrete runtime path, keeping the
/// success/error transport generic while allowing compatibility shells to attach
/// host-specific diagnostics during migration.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeRunReport<Artifacts, Error = String> {
    /// Structured artifacts captured during the run.
    pub artifacts: Artifacts,
    /// Native runtime success or error outcome.
    pub result: Result<(), Error>,
}

pub use native_vello::{
    NativeGenericRunError, NativeGenericRunReport, NativeGenericRuntimeArtifacts,
    NativeStartupTimingArtifact, run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
};
pub use options::{
    DEFAULT_NATIVE_WINDOW_TITLE, EmbeddedFont, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS,
    NativeGpuBackend, NativeGpuOptions, NativePopupOptions, NativeRunOptions,
    NativeRunOptionsError, NativeTextOptions, NativeWindowMode, WindowIconRgba,
};
pub use window_manifest::{
    WindowManifest, WindowManifestError, WindowSpec, WindowSpecError, WindowSpecParts,
};
