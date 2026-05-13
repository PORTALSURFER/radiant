/// Native window presentation mode used by runtime adapters.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum NativeWindowMode {
    /// Standard application window managed by the host runtime.
    #[default]
    Window,
    /// Floating popup window for transient UI such as drag previews, tooltips,
    /// context menus, and other borderless surfaces.
    Popup(NativePopupOptions),
}

/// Platform-neutral policy for floating popup windows.
///
/// Popup windows still render normal Radiant surfaces. The policy only
/// describes native-window behavior that differs from a regular application
/// window: borderless chrome, optional transparency, z-order, taskbar presence,
/// focus behavior, resizability, drag movement, and optional initial screen
/// position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NativePopupOptions {
    /// Initial outer-window position in logical screen coordinates.
    pub position: Option<[f32; 2]>,
    /// Whether the native window background should support transparency.
    pub transparent: bool,
    /// Whether the popup should be kept above normal windows when supported.
    pub always_on_top: bool,
    /// Whether the popup window should be resizable.
    pub resizable: bool,
    /// Whether the popup should request focus when it opens.
    pub initially_focused: bool,
    /// Whether the popup should stay out of the platform taskbar when supported.
    pub skip_taskbar: bool,
    /// Optional top-edge logical height that can initiate native window dragging.
    ///
    /// The native runtime only starts the drag when primary pointer press hits
    /// otherwise inactive popup surface inside this region, so controls in the
    /// same area can still receive normal widget input.
    pub drag_region_height: Option<f32>,
}

impl Default for NativePopupOptions {
    fn default() -> Self {
        Self {
            position: None,
            transparent: true,
            always_on_top: true,
            resizable: false,
            initially_focused: false,
            skip_taskbar: true,
            drag_region_height: None,
        }
    }
}

impl NativePopupOptions {
    /// Set the initial outer-window position in logical screen coordinates.
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = Some([x, y]);
        self
    }

    /// Set whether the popup should support background transparency.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Set whether the popup should stay above normal windows when supported.
    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }

    /// Set whether the popup should be resizable.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set whether the popup should request focus when it opens.
    pub fn initially_focused(mut self, initially_focused: bool) -> Self {
        self.initially_focused = initially_focused;
        self
    }

    /// Set whether the popup should stay out of the platform taskbar when supported.
    pub fn skip_taskbar(mut self, skip_taskbar: bool) -> Self {
        self.skip_taskbar = skip_taskbar;
        self
    }

    /// Set the top-edge logical height that should allow native popup dragging.
    pub fn drag_region_height(mut self, height: f32) -> Self {
        self.drag_region_height = Some(height);
        self
    }
}
