//! Legacy native Vello compatibility facade used by Radiant.

use super::{NativeAppBridge, NativeRunReport};
use crate::gui_runtime::{NativeRunOptions, native_vello};

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
