use crate::{
    application::{Result, launch::IntoView},
    gui_runtime::{EmbeddedFont, NativePopupOptions, NativeRunOptions, WindowSpec},
    runtime::{
        Command, RuntimeBridge, declarative_command_runtime_bridge, run_native_vello_runtime,
    },
};
use std::sync::Arc;

/// Builder for no-state native windows.
pub struct WindowBuilder {
    options: NativeRunOptions,
}

impl WindowBuilder {
    pub(super) fn new(title: impl Into<String>) -> Self {
        Self {
            options: NativeRunOptions {
                title: title.into(),
                ..NativeRunOptions::default()
            },
        }
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

    /// Set the full native runtime options, preserving this builder as a thin adapter.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Configure this native window as a borderless floating popup.
    pub fn floating_popup(mut self) -> Self {
        self.options = self.options.floating_popup();
        self
    }

    /// Configure this native window as a floating popup with explicit policy.
    pub fn popup_policy(mut self, popup: NativePopupOptions) -> Self {
        self.options = self.options.popup_policy(popup);
        self
    }

    /// Set the initial popup position in logical screen coordinates.
    pub fn popup_position(mut self, x: f32, y: f32) -> Self {
        self.options = self.options.popup_position(x, y);
        self
    }

    /// Add embedded TTF/OTF font bytes checked before file and native fallback fonts.
    pub fn embedded_font(mut self, font: impl Into<EmbeddedFont>) -> Self {
        self.options.text.embedded_fonts.push(font.into());
        self
    }

    /// Add a preferred font file checked after embedded fonts and before native fallbacks.
    pub fn font_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.options.text.font_paths.push(path.into());
        self
    }

    /// Convert this launch builder into a platform-neutral window descriptor.
    pub fn spec(self, key: impl Into<String>) -> WindowSpec {
        WindowSpec::from_options(key, self.options)
    }

    /// Run one static view through the native Vello runtime.
    pub fn run<View>(self, view: View) -> Result
    where
        View: IntoView<()> + 'static,
    {
        let surface = Arc::new(view.into_surface());
        let bridge = declarative_command_runtime_bridge(
            surface,
            |surface| Arc::clone(surface),
            |_, ()| Command::none(),
        );
        run_native_vello_runtime(self.options, bridge)
    }

    /// Run an existing runtime bridge through this window builder.
    ///
    /// This keeps host-specific bridges on the same application/window launch
    /// API as ordinary Radiant examples while preserving their custom state and
    /// diagnostics.
    pub fn run_bridge<Bridge, Message>(self, bridge: Bridge) -> Result
    where
        Bridge: RuntimeBridge<Message> + 'static,
        Message: 'static,
    {
        run_native_vello_runtime(self.options, bridge)
    }

    /// Run an existing runtime bridge and return native runtime artifacts.
    pub fn run_bridge_with_artifacts<Bridge, Message>(
        self,
        bridge: Bridge,
    ) -> crate::gui_runtime::NativeGenericRunReport
    where
        Bridge: RuntimeBridge<Message> + 'static,
        Message: 'static,
    {
        crate::runtime::run_native_vello_runtime_with_artifacts(self.options, bridge)
    }
}
