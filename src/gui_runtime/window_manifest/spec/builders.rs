use super::{WindowSpec, WindowSpecParts};
use crate::gui_runtime::{
    EmbeddedFont, NativeGpuBackend, NativePopupOptions, NativeRunOptions, WindowIconRgba,
};
use std::path::PathBuf;

impl WindowSpec {
    /// Build a window descriptor from named parts.
    pub fn from_parts(parts: WindowSpecParts) -> Self {
        Self {
            key: parts.key,
            options: parts.options,
        }
    }

    /// Build a window descriptor from a stable key and title.
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self::from_parts(WindowSpecParts {
            key: key.into(),
            options: NativeRunOptions {
                title: title.into(),
                ..NativeRunOptions::default()
            },
        })
    }

    /// Build a window descriptor from explicit native runtime options.
    pub fn from_options(key: impl Into<String>, options: NativeRunOptions) -> Self {
        Self::from_parts(WindowSpecParts {
            key: key.into(),
            options,
        })
    }

    /// Build a borderless floating popup descriptor.
    ///
    /// Popup specs are intended for transient Radiant surfaces such as drag
    /// previews, context menus, tooltips, and small floating panels that must
    /// render outside the main application window.
    pub fn popup(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self::from_options(key, NativeRunOptions::popup(title))
    }

    /// Build a prewarmed floating popup descriptor.
    ///
    /// The popup starts at the supplied logical screen position, presents once,
    /// and hides itself so the host can reveal a prepared window later.
    pub fn prewarmed_popup(
        key: impl Into<String>,
        title: impl Into<String>,
        x: f32,
        y: f32,
    ) -> Self {
        Self::from_options(key, NativeRunOptions::prewarmed_popup(title, x, y))
    }

    /// Set the initial logical window size.
    pub fn size(self, width: u32, height: u32) -> Self {
        self.logical_size(width as f32, height as f32)
    }

    /// Set the initial logical window size using floating-point logical pixels.
    pub fn logical_size(mut self, width: f32, height: f32) -> Self {
        self.options.inner_size = Some([width, height]);
        self
    }

    /// Set the initial logical window position in screen coordinates.
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.options.position = Some([x, y]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(self, width: u32, height: u32) -> Self {
        self.min_logical_size(width as f32, height as f32)
    }

    /// Set the minimum logical window size using floating-point logical pixels.
    pub fn min_logical_size(mut self, width: f32, height: f32) -> Self {
        self.options.min_inner_size = Some([width, height]);
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

    /// Configure this descriptor as a floating popup with default popup policy.
    pub fn floating_popup(mut self) -> Self {
        self.options = self.options.floating_popup();
        self
    }

    /// Configure this descriptor as a floating popup with explicit popup policy.
    pub fn popup_policy(mut self, popup: NativePopupOptions) -> Self {
        self.options = self.options.popup_policy(popup);
        self
    }

    /// Set the initial popup position in logical screen coordinates.
    pub fn popup_position(mut self, x: f32, y: f32) -> Self {
        self.options = self.options.popup_position(x, y);
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

    /// Set the preferred native GPU backend for this window.
    pub fn gpu_backend(mut self, backend: NativeGpuBackend) -> Self {
        self.options.gpu.backend = backend;
        self
    }

    /// Add a preferred font file checked before native fallback fonts.
    pub fn font_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.text.font_paths.push(path.into());
        self
    }

    /// Add embedded TTF/OTF font bytes checked before file and native fallback fonts.
    pub fn embedded_font(mut self, font: impl Into<EmbeddedFont>) -> Self {
        self.options.text.embedded_fonts.push(font.into());
        self
    }
}
