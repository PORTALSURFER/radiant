use super::{
    NativeGpuOptions, NativePopupOptions, NativeTextOptions, NativeWindowMode, WindowIconRgba,
};

/// Default title for generic Radiant native windows.
pub const DEFAULT_NATIVE_WINDOW_TITLE: &str = "Radiant";

/// Lowest native animation frame rate Radiant will schedule.
pub const MIN_NATIVE_TARGET_FPS: u32 = 1;

/// Highest native animation frame rate Radiant will schedule.
pub const MAX_NATIVE_TARGET_FPS: u32 = 240;

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
    ///
    /// Native runtimes clamp this to Radiant's supported scheduling range
    /// before using it for timed redraws or present-mode selection.
    pub target_fps: u32,
    /// GPU adapter/backend policy for native renderers.
    pub gpu: NativeGpuOptions,
    /// Text and font policy for native renderers.
    pub text: NativeTextOptions,
    /// Paint red layout-boundary strokes over every projected layout element.
    pub debug_layout: bool,
    /// Native window presentation mode for this surface.
    pub window_mode: NativeWindowMode,
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
            window_mode: NativeWindowMode::default(),
        }
    }
}

impl NativeRunOptions {
    /// Return options configured for a transient floating popup window.
    pub fn popup(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            decorations: false,
            drag_and_drop: false,
            window_mode: NativeWindowMode::Popup(NativePopupOptions::default()),
            ..Self::default()
        }
    }

    /// Return whether these options describe a floating popup window.
    pub const fn is_popup(&self) -> bool {
        matches!(self.window_mode, NativeWindowMode::Popup(_))
    }

    /// Borrow the popup policy when this window is configured as a popup.
    pub const fn popup_options(&self) -> Option<&NativePopupOptions> {
        match &self.window_mode {
            NativeWindowMode::Popup(options) => Some(options),
            NativeWindowMode::Window => None,
        }
    }

    /// Configure this window as a floating popup with default popup policy.
    pub fn floating_popup(mut self) -> Self {
        self.decorations = false;
        self.drag_and_drop = false;
        self.window_mode = NativeWindowMode::Popup(NativePopupOptions::default());
        self
    }

    /// Configure this window as a floating popup with explicit popup policy.
    pub fn popup_policy(mut self, popup: NativePopupOptions) -> Self {
        self.decorations = false;
        self.drag_and_drop = false;
        self.window_mode = NativeWindowMode::Popup(popup);
        self
    }

    /// Set the initial popup position, configuring this window as a popup when needed.
    pub fn popup_position(self, x: f32, y: f32) -> Self {
        let popup = match self.window_mode {
            NativeWindowMode::Popup(options) => options.position(x, y),
            NativeWindowMode::Window => NativePopupOptions::default().position(x, y),
        };
        self.popup_policy(popup)
    }

    /// Return the effective native animation frame rate after policy clamping.
    pub const fn normalized_target_fps(&self) -> u32 {
        normalize_native_target_fps(self.target_fps)
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
