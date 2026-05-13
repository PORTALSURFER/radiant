//! Host-owned native window descriptors and manifest validation.

use super::{EmbeddedFont, NativeGpuBackend, NativePopupOptions, NativeRunOptions, WindowIconRgba};
use std::{collections::HashSet, path::PathBuf};

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

/// Host-managed collection of application window descriptors.
///
/// This is a manifest and validation object, not a native event-loop runner.
/// Multi-window hosts can pair each spec with a separate runtime bridge or view
/// and let their platform adapter decide how to open, embed, or route windows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowManifest {
    specs: Vec<WindowSpec>,
}

impl WindowManifest {
    /// Build an empty window manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a manifest from explicit specs.
    pub fn from_specs(specs: impl IntoIterator<Item = WindowSpec>) -> Result<Self, String> {
        let mut manifest = Self::new();
        for spec in specs {
            manifest.push(spec)?;
        }
        Ok(manifest)
    }

    /// Add one window spec, rejecting duplicate stable keys.
    pub fn push(&mut self, spec: WindowSpec) -> Result<(), String> {
        if self.specs.iter().any(|existing| existing.key == spec.key) {
            return Err(format!("duplicate window key '{}'", spec.key));
        }
        self.specs.push(spec);
        Ok(())
    }

    /// Return the number of window specs.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Return whether this manifest has no window specs.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Return all stable window keys in manifest order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.specs.iter().map(|spec| spec.key.as_str())
    }

    /// Borrow one spec by stable key.
    pub fn get(&self, key: &str) -> Option<&WindowSpec> {
        self.specs.iter().find(|spec| spec.key == key)
    }

    /// Borrow all specs in manifest order.
    pub fn specs(&self) -> &[WindowSpec] {
        &self.specs
    }

    /// Consume the manifest into its ordered specs.
    pub fn into_specs(self) -> Vec<WindowSpec> {
        self.specs
    }

    /// Verify that all window keys are unique.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::with_capacity(self.specs.len());
        for spec in &self.specs {
            if !seen.insert(spec.key.as_str()) {
                return Err(format!("duplicate window key '{}'", spec.key));
            }
        }
        Ok(())
    }
}
