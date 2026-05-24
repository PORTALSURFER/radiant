use super::{NativeRunOptions, WindowSpec, WindowSpecError, validate_window_spec};
use crate::gui_runtime::NativePopupOptions;

impl WindowSpec {
    /// Return the configured window title.
    pub fn title(&self) -> &str {
        self.options.window.title.as_str()
    }

    /// Return the configured initial logical window size, if one was set.
    pub const fn inner_size(&self) -> Option<[f32; 2]> {
        self.options.window.geometry.inner_size
    }

    /// Return the configured minimum logical window size, if one was set.
    pub const fn min_inner_size(&self) -> Option<[f32; 2]> {
        self.options.window.geometry.min_inner_size
    }

    /// Return whether native file drag-and-drop is enabled when supported.
    pub const fn drag_and_drop_enabled(&self) -> bool {
        self.options.window.behavior.drag_and_drop
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
        self.options.frame.target_fps
    }

    /// Return the effective target animation frame rate after native policy clamping.
    pub const fn normalized_target_frame_rate(&self) -> u32 {
        self.options.normalized_target_fps()
    }

    /// Borrow the native options represented by this descriptor.
    pub const fn native_options(&self) -> &NativeRunOptions {
        &self.options
    }

    /// Validate host-authored window geometry before a platform adapter opens it.
    pub fn validate(&self) -> Result<(), WindowSpecError> {
        validate_window_spec(self)
    }

    /// Consume this descriptor and return the native runtime options.
    pub fn into_native_options(self) -> NativeRunOptions {
        self.options
    }
}
