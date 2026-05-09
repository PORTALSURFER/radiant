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

/// Default title for generic Radiant native windows.
pub const DEFAULT_NATIVE_WINDOW_TITLE: &str = "Radiant";

/// RGBA icon bytes used to initialize a native window icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// Window configuration shared by native runtime entry points.
#[derive(Clone, Debug, PartialEq)]
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
    /// Whether native file drag-and-drop should be enabled when supported.
    ///
    /// Unsupported platforms may ignore this option. Keeping the capability on
    /// the generic runtime options avoids hardcoding platform-specific window
    /// behavior into application-independent launch code.
    pub drag_and_drop: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
    /// Target frame rate for animation-driven redraws.
    pub target_fps: u32,
}

impl Default for NativeRunOptions {
    fn default() -> Self {
        Self {
            title: String::from(DEFAULT_NATIVE_WINDOW_TITLE),
            inner_size: None,
            min_inner_size: None,
            maximized: false,
            decorations: true,
            drag_and_drop: true,
            icon: None,
            target_fps: 120,
        }
    }
}

/// Platform-neutral descriptor for one application window.
///
/// `WindowSpec` is intentionally a manifest object, not an event-loop runtime.
/// Hosts that need multiple windows can keep a collection of specs, attach a
/// separate runtime bridge per spec, and let a platform adapter decide how to
/// open or embed each surface.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    /// Stable host-owned key for this window.
    pub key: String,
    /// Native launch options for this window.
    pub options: NativeRunOptions,
}

impl WindowSpec {
    /// Build a window descriptor from a stable key and title.
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            options: NativeRunOptions {
                title: title.into(),
                ..NativeRunOptions::default()
            },
        }
    }

    /// Build a window descriptor from explicit native runtime options.
    pub fn from_options(key: impl Into<String>, options: NativeRunOptions) -> Self {
        Self {
            key: key.into(),
            options,
        }
    }

    /// Set the initial logical window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.options.inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.options.min_inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set whether the window starts maximized.
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.options.maximized = maximized;
        self
    }

    /// Set whether native window decorations remain enabled.
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.options.decorations = decorations;
        self
    }

    /// Set whether native file drag-and-drop is enabled when supported.
    pub fn drag_and_drop(mut self, drag_and_drop: bool) -> Self {
        self.options.drag_and_drop = drag_and_drop;
        self
    }

    /// Set the optional native window icon.
    pub fn icon(mut self, icon: WindowIconRgba) -> Self {
        self.options.icon = Some(icon);
        self
    }

    /// Set the target animation frame rate for this window.
    pub fn target_fps(mut self, target_fps: u32) -> Self {
        self.options.target_fps = target_fps;
        self
    }

    /// Borrow the native options represented by this descriptor.
    pub const fn native_options(&self) -> &NativeRunOptions {
        &self.options
    }

    /// Consume this descriptor and return the native runtime options.
    pub fn into_native_options(self) -> NativeRunOptions {
        self.options
    }
}

impl From<WindowSpec> for NativeRunOptions {
    fn from(spec: WindowSpec) -> Self {
        spec.into_native_options()
    }
}

/// Generic result envelope returned by native runtime entry points.
///
/// The artifact payload is supplied by the concrete runtime path, keeping the
/// success/error transport generic while allowing compatibility shells to attach
/// host-specific diagnostics during migration.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeRunReport<Artifacts> {
    /// Structured artifacts captured during the run.
    pub artifacts: Artifacts,
    /// Native runtime success or error outcome.
    pub result: Result<(), String>,
}

pub use native_vello::{
    NativeGenericRunReport, NativeGenericRuntimeArtifacts, NativeStartupTimingArtifact,
    run_native_vello_runtime, run_native_vello_runtime_with_artifacts,
};
