//! Native Vello runtime entrypoint for the transitional legacy-shell bridge.

use crate::compat::legacy_shell::NativeAppBridge;
use crate::gui_runtime::{NativeRunOptions, native_vello};

/// Structured runtime artifacts exported after one native compatibility-shell run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<crate::gui_runtime::NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Result plus structured artifacts returned by one native compatibility-shell runtime execution.
pub type NativeRunReport = crate::gui_runtime::RuntimeRunReport<NativeRuntimeArtifacts>;

/// Run the native Vello backend window with a host-provided legacy shell bridge.
///
/// The runtime loop is owned by winit and blocks until the native window
/// closes. The host receives user input each frame through the bridge-driven
/// action path, and this function returns the host result from the event loop
/// invocation.
pub fn run_native_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    native_vello::run_legacy_shell_vello_app_with_artifacts(options, bridge)
}
