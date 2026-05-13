use crate::gui_runtime::{
    EmbeddedFont, NativeGpuBackend, NativePopupOptions, NativeRunOptions, WindowIconRgba,
};
use std::{fmt, path::PathBuf};

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

/// Error returned when one window descriptor contains invalid geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowSpecError {
    /// Initial or minimum logical size is non-finite or non-positive.
    InvalidSize {
        /// Stable host-owned key for the invalid window.
        key: String,
        /// Name of the invalid size field.
        field: &'static str,
        /// Invalid logical width.
        width: f32,
        /// Invalid logical height.
        height: f32,
    },
    /// Popup position contains a non-finite coordinate.
    InvalidPopupPosition {
        /// Stable host-owned key for the invalid window.
        key: String,
        /// Invalid logical x coordinate.
        x: f32,
        /// Invalid logical y coordinate.
        y: f32,
    },
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

    /// Build a borderless floating popup descriptor.
    ///
    /// Popup specs are intended for transient Radiant surfaces such as drag
    /// previews, context menus, tooltips, and small floating panels that must
    /// render outside the main application window.
    pub fn popup(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self::from_options(key, NativeRunOptions::popup(title))
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

    /// Return the configured window title.
    pub fn title(&self) -> &str {
        self.options.title.as_str()
    }

    /// Return the configured initial logical window size, if one was set.
    pub const fn inner_size(&self) -> Option<[f32; 2]> {
        self.options.inner_size
    }

    /// Return the configured minimum logical window size, if one was set.
    pub const fn min_inner_size(&self) -> Option<[f32; 2]> {
        self.options.min_inner_size
    }

    /// Return whether native file drag-and-drop is enabled when supported.
    pub const fn drag_and_drop_enabled(&self) -> bool {
        self.options.drag_and_drop
    }

    /// Return whether this descriptor represents a floating popup window.
    pub const fn is_popup(&self) -> bool {
        self.options.is_popup()
    }

    /// Borrow the popup policy when this descriptor is a floating popup.
    pub const fn popup_options(&self) -> Option<&NativePopupOptions> {
        self.options.popup_options()
    }

    /// Return the target animation frame rate for this window.
    pub const fn target_frame_rate(&self) -> u32 {
        self.options.target_fps
    }

    /// Borrow the native options represented by this descriptor.
    pub const fn native_options(&self) -> &NativeRunOptions {
        &self.options
    }

    /// Validate host-authored window geometry before a platform adapter opens it.
    pub fn validate(&self) -> Result<(), WindowSpecError> {
        validate_size(self.key.as_str(), "inner_size", self.options.inner_size)?;
        validate_size(
            self.key.as_str(),
            "min_inner_size",
            self.options.min_inner_size,
        )?;
        if let Some(popup) = self.options.popup_options() {
            validate_position(self.key.as_str(), popup.position)?;
        }
        Ok(())
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

impl fmt::Display for WindowSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize {
                key,
                field,
                width,
                height,
            } => write!(
                formatter,
                "window '{key}' has invalid {field} [{width}, {height}]; logical sizes must be finite and positive"
            ),
            Self::InvalidPopupPosition { key, x, y } => write!(
                formatter,
                "window '{key}' has invalid popup position [{x}, {y}]; popup positions must be finite"
            ),
        }
    }
}

impl std::error::Error for WindowSpecError {}

fn validate_size(
    key: &str,
    field: &'static str,
    size: Option<[f32; 2]>,
) -> Result<(), WindowSpecError> {
    let Some([width, height]) = size else {
        return Ok(());
    };
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        return Ok(());
    }
    Err(WindowSpecError::InvalidSize {
        key: key.to_string(),
        field,
        width,
        height,
    })
}

fn validate_position(key: &str, position: Option<[f32; 2]>) -> Result<(), WindowSpecError> {
    let Some([x, y]) = position else {
        return Ok(());
    };
    if x.is_finite() && y.is_finite() {
        return Ok(());
    }
    Err(WindowSpecError::InvalidPopupPosition {
        key: key.to_string(),
        x,
        y,
    })
}
