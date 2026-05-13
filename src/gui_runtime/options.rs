//! Platform-neutral native runtime policy types.

use std::{fmt, path::PathBuf, sync::Arc};

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
    /// Host-provided font bytes checked before file and native fallback fonts.
    pub embedded_fonts: Vec<EmbeddedFont>,
    /// Host-preferred font files checked before environment and system fallbacks.
    pub font_paths: Vec<PathBuf>,
}

impl NativeTextOptions {
    /// Add embedded TTF/OTF font bytes checked before file and native fallback fonts.
    pub fn embedded_font(mut self, font: impl Into<EmbeddedFont>) -> Self {
        self.embedded_fonts.push(font.into());
        self
    }

    /// Add a preferred font file checked after embedded fonts and before native fallbacks.
    pub fn font_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.font_paths.push(path.into());
        self
    }
}

/// Application-owned font bytes that can be bundled into the executable.
///
/// This is intended for `include_bytes!(...)` style packaging where the
/// application should not depend on an installed font file at runtime.
#[derive(Clone, PartialEq, Eq)]
pub struct EmbeddedFont {
    bytes: Arc<[u8]>,
    index: u32,
}

impl EmbeddedFont {
    /// Create an embedded font from static bytes, typically `include_bytes!`.
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self::from_bytes(bytes)
    }

    /// Create an embedded font from owned bytes.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Self {
        Self {
            bytes: Arc::from(bytes.as_ref()),
            index: 0,
        }
    }

    /// Set the font index for collection files.
    pub fn with_index(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    /// Borrow the embedded font bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn shared_bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }

    /// Return the font index used for this embedded font.
    pub const fn index(&self) -> u32 {
        self.index
    }
}

impl From<&'static [u8]> for EmbeddedFont {
    fn from(bytes: &'static [u8]) -> Self {
        Self::from_static(bytes)
    }
}

impl From<Vec<u8>> for EmbeddedFont {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::from(bytes),
            index: 0,
        }
    }
}

impl fmt::Debug for EmbeddedFont {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EmbeddedFont")
            .field("len", &self.bytes.len())
            .field("index", &self.index)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::EmbeddedFont;
    use std::sync::Arc;

    #[test]
    fn embedded_font_shared_bytes_reuses_storage() {
        let font = EmbeddedFont::from_static(b"font bytes");
        let first = font.shared_bytes();
        let second = font.shared_bytes();

        assert!(Arc::ptr_eq(&first, &second));
    }
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
    /// Paint red layout-boundary strokes over every projected layout element.
    pub debug_layout: bool,
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
            debug_layout: false,
        }
    }
}
