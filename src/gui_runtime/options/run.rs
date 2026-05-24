use super::{
    NativeGpuOptions, NativePopupOptions, NativeTextOptions, NativeWindowMode, WindowIconRgba,
};
use crate::runtime::RetainedSurfaceCachePolicy;

mod validation;
pub use validation::NativeRunOptionsError;
use validation::{validate_popup_drag_region, validate_position, validate_size};

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

impl NativeRunOptions {
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
