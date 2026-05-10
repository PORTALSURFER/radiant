//! Platform-neutral native runtime policy types.

use std::path::PathBuf;

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

/// Explicit native GPU backend preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeGpuBackend {
    /// Use WGPU's normal environment-aware backend selection.
    #[default]
    Auto,
    /// Prefer WGPU's primary production backends for the current platform.
    Primary,
    /// Restrict adapter selection to Vulkan.
    Vulkan,
    /// Restrict adapter selection to DirectX 12.
    Dx12,
    /// Restrict adapter selection to Metal.
    Metal,
    /// Restrict adapter selection to OpenGL or OpenGL ES.
    Gl,
    /// Restrict adapter selection to browser WebGPU.
    BrowserWebGpu,
}

/// Native GPU policy used by backend runtime adapters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuOptions {
    /// Preferred GPU backend for adapter selection.
    pub backend: NativeGpuBackend,
}

/// Native text/font policy used by backend runtime adapters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeTextOptions {
    /// Host-preferred font files checked before environment and system fallbacks.
    pub font_paths: Vec<PathBuf>,
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
    /// GPU adapter/backend policy for native renderers.
    pub gpu: NativeGpuOptions,
    /// Text and font policy for native renderers.
    pub text: NativeTextOptions,
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
            gpu: NativeGpuOptions::default(),
            text: NativeTextOptions::default(),
        }
    }
}
