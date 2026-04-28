//! Shared GUI runtime host implementations.
//!
//! The runtime is responsible for the host integration lifecycle:
//! 1. open/create a window and event loop,
//! 2. pull app snapshots and submit them to `radiant` for frame build,
//! 3. schedule redraws when input, timers, or host events require updates,
//! 4. emit platform input into normalized runtime events.
//!
//! Render work is intentionally performed through feature-gated hooks in the
//! underlying backend (for example `gui-performance`) so release builds that do
//! not request profiling have no measurable instrumentation overhead.
//!
//! Native Vello exposes both a generic [`crate::runtime::RuntimeBridge`]
//! entrypoint for reusable host applications and compatibility shell helpers
//! through [`crate::compat::sempal_shell`].

mod native_vello;

/// RGBA icon bytes used to initialize a native window icon.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// Window configuration shared by native runtime entry points.
#[derive(Clone, Debug)]
pub struct NativeRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Whether native window decorations remain enabled.
    pub decorations: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
    /// Target frame rate for animation-driven redraws.
    pub target_fps: u32,
}

impl Default for NativeRunOptions {
    fn default() -> Self {
        Self {
            title: String::from(crate::compat::sempal_shell::DEFAULT_APP_TITLE),
            inner_size: None,
            min_inner_size: None,
            maximized: false,
            decorations: true,
            icon: None,
            target_fps: 120,
        }
    }
}

pub use crate::sempal_app::NativeShutdownTimingArtifact;
pub use native_vello::{
    NativeGenericRunReport, NativeGenericRuntimeArtifacts, NativeRunReport, NativeRuntimeArtifacts,
    NativeStartupTimingArtifact, capture_gui_automation_snapshot,
    capture_native_shell_shot_snapshot, run_native_vello_app, run_native_vello_app_declarative,
    run_native_vello_app_declarative_with_artifacts, run_native_vello_app_with_artifacts,
    run_native_vello_preview, run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
};
