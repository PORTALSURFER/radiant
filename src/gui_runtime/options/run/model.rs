use super::super::{NativeGpuOptions, NativeTextOptions, NativeWindowMode, WindowIconRgba};
use crate::runtime::RetainedSurfaceCachePolicy;

/// Default title for generic Radiant native windows.
pub const DEFAULT_NATIVE_WINDOW_TITLE: &str = "Radiant";

/// Lowest native animation frame rate Radiant will schedule.
pub const MIN_NATIVE_TARGET_FPS: u32 = 1;

/// Highest native animation frame rate Radiant will schedule.
pub const MAX_NATIVE_TARGET_FPS: u32 = 240;

/// Window configuration shared by native runtime entry points.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRunOptions {
    /// Window identity, geometry, and platform behavior policy.
    pub window: NativeWindowOptions,
    /// Native frame scheduling and diagnostics policy.
    pub frame: NativeFrameOptions,
    /// GPU adapter/backend policy for native renderers.
    pub gpu: NativeGpuOptions,
    /// Text and font policy for native renderers.
    pub text: NativeTextOptions,
}

/// Window identity, geometry, and platform behavior policy.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeWindowOptions {
    /// Window title.
    pub title: String,
    /// Initial and minimum logical window geometry.
    pub geometry: NativeWindowGeometry,
    /// Platform-level window behavior.
    pub behavior: NativeWindowBehavior,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
}

/// Initial and minimum logical window geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NativeWindowGeometry {
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Initial outer window position in logical screen coordinates.
    pub position: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
}

/// Platform-level window behavior.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeWindowBehavior {
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
    /// Native owner window handle for auxiliary top-level windows.
    ///
    /// On Windows this is an `HWND` encoded as an integer and creates an owned
    /// window. Other platforms may ignore this option until they expose a
    /// matching native ownership primitive through the backend.
    pub owner_window_handle: Option<isize>,
    /// Whether the native window should stay out of the platform taskbar when supported.
    pub skip_taskbar: bool,
    /// Native window presentation mode for this surface.
    pub mode: NativeWindowMode,
}

/// Native frame scheduling and diagnostics policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativeFrameOptions {
    /// Target frame rate for animation-driven redraws.
    ///
    /// Native runtimes clamp this to Radiant's supported scheduling range
    /// before using it for timed redraws or present-mode selection.
    pub target_fps: u32,
    /// Paint red layout-boundary strokes over every projected layout element.
    pub debug_layout: bool,
    /// Retained custom-surface frame cache policy.
    pub retained_surface_cache: RetainedSurfaceCachePolicy,
}

impl Default for NativeWindowOptions {
    fn default() -> Self {
        Self {
            title: String::from(DEFAULT_NATIVE_WINDOW_TITLE),
            geometry: NativeWindowGeometry::default(),
            behavior: NativeWindowBehavior::default(),
            icon: None,
        }
    }
}

impl Default for NativeWindowBehavior {
    fn default() -> Self {
        Self {
            maximized: false,
            decorations: true,
            drag_and_drop: true,
            owner_window_handle: None,
            skip_taskbar: false,
            mode: NativeWindowMode::default(),
        }
    }
}

impl Default for NativeFrameOptions {
    fn default() -> Self {
        Self {
            target_fps: 120,
            debug_layout: false,
            retained_surface_cache: RetainedSurfaceCachePolicy::default(),
        }
    }
}
