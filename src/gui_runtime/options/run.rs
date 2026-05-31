use super::{NativePopupOptions, NativeWindowMode};

mod model;
mod validation;
pub use model::{
    DEFAULT_NATIVE_WINDOW_TITLE, MAX_NATIVE_TARGET_FPS, MIN_NATIVE_TARGET_FPS, NativeFrameOptions,
    NativeRunOptions, NativeWindowBehavior, NativeWindowGeometry, NativeWindowOptions,
};
pub use validation::NativeRunOptionsError;
use validation::{validate_popup_drag_region, validate_position, validate_size};

impl NativeRunOptions {
    /// Return options configured for a secondary utility window.
    ///
    /// Utility windows are ordinary decorated top-level windows intended for
    /// settings, inspectors, and tool panels. They stay out of the taskbar,
    /// use the supplied logical size as both their initial and minimum inner
    /// size, and disable native file drag-and-drop by default.
    pub fn utility_window(title: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            window: NativeWindowOptions {
                title: title.into(),
                geometry: NativeWindowGeometry {
                    inner_size: Some([width, height]),
                    min_inner_size: Some([width, height]),
                    ..NativeWindowGeometry::default()
                },
                behavior: NativeWindowBehavior {
                    drag_and_drop: false,
                    skip_taskbar: true,
                    ..NativeWindowBehavior::default()
                },
                ..NativeWindowOptions::default()
            },
            ..Self::default()
        }
    }

    /// Return options configured for a transient floating popup window.
    pub fn popup(title: impl Into<String>) -> Self {
        Self {
            window: NativeWindowOptions {
                title: title.into(),
                behavior: NativeWindowBehavior {
                    decorations: false,
                    drag_and_drop: false,
                    mode: NativeWindowMode::Popup(NativePopupOptions::default()),
                    ..NativeWindowBehavior::default()
                },
                ..NativeWindowOptions::default()
            },
            ..Self::default()
        }
    }

    /// Return options configured for a prewarmed transient popup window.
    ///
    /// The popup first presents at the supplied logical screen position, hides
    /// after that first frame, and can then be revealed by the host on demand.
    pub fn prewarmed_popup(title: impl Into<String>, x: f32, y: f32) -> Self {
        Self::popup(title).popup_policy(NativePopupOptions::prewarmed_at(x, y))
    }

    /// Return whether these options describe a floating popup window.
    pub const fn is_popup(&self) -> bool {
        matches!(self.window.behavior.mode, NativeWindowMode::Popup(_))
    }

    /// Borrow the popup policy when this window is configured as a popup.
    pub const fn popup_options(&self) -> Option<&NativePopupOptions> {
        match &self.window.behavior.mode {
            NativeWindowMode::Popup(options) => Some(options),
            NativeWindowMode::Window => None,
        }
    }

    /// Configure this window as a floating popup with default popup policy.
    pub fn floating_popup(mut self) -> Self {
        self.window.behavior.decorations = false;
        self.window.behavior.drag_and_drop = false;
        self.window.behavior.mode = NativeWindowMode::Popup(NativePopupOptions::default());
        self
    }

    /// Configure this window as a floating popup with explicit popup policy.
    pub fn popup_policy(mut self, popup: NativePopupOptions) -> Self {
        self.window.behavior.decorations = false;
        self.window.behavior.drag_and_drop = false;
        self.window.behavior.mode = NativeWindowMode::Popup(popup);
        self
    }

    /// Set the initial popup position, configuring this window as a popup when needed.
    pub fn popup_position(self, x: f32, y: f32) -> Self {
        let popup = match self.window.behavior.mode {
            NativeWindowMode::Popup(options) => options.position(x, y),
            NativeWindowMode::Window => NativePopupOptions::default().position(x, y),
        };
        self.popup_policy(popup)
    }

    /// Return the effective native animation frame rate after policy clamping.
    pub const fn normalized_target_fps(&self) -> u32 {
        normalize_native_target_fps(self.frame.target_fps)
    }

    /// Validate native launch geometry before handing options to a platform runtime.
    pub fn validate(&self) -> Result<(), NativeRunOptionsError> {
        validate_size("inner_size", self.window.geometry.inner_size)?;
        validate_size("min_inner_size", self.window.geometry.min_inner_size)?;
        if let Some(popup) = self.popup_options() {
            validate_position("popup_position", popup.position)?;
            validate_popup_drag_region(popup.drag_region_height)?;
        }
        Ok(())
    }
}

pub(crate) const fn normalize_native_target_fps(target_fps: u32) -> u32 {
    if target_fps < MIN_NATIVE_TARGET_FPS {
        MIN_NATIVE_TARGET_FPS
    } else if target_fps > MAX_NATIVE_TARGET_FPS {
        MAX_NATIVE_TARGET_FPS
    } else {
        target_fps
    }
}
